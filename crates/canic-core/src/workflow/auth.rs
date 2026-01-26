//! Delegation issuance and rotation workflow.
//!
//! This module defines the **operational workflow** for:
//! - issuing delegated signing authority
//! - rotating that authority on a timer
//!
//! It is intentionally *thin* and orchestration-only.
//! All cryptographic validation, authorization, and policy enforcement
//! occur elsewhere.

use crate::{
    InternalError,
    cdk::types::Principal,
    dto::{
        auth::{
            DelegationCert, DelegationProof, DelegationProvisionRequest,
            DelegationProvisionResponse, DelegationProvisionStatus, DelegationProvisionTargetKind,
            DelegationProvisionTargetResponse,
        },
        error::Error as ErrorDto,
    },
    log,
    log::Topic,
    ops::{auth::DelegatedTokenOps, ic::call::CallOps, storage::auth::DelegationStateOps},
    protocol,
    workflow::runtime::timer::{TimerId, TimerWorkflow},
};
use std::{cell::RefCell, sync::Arc, time::Duration};

thread_local! {
    /// Guarded timer handle for delegation rotation.
    ///
    /// WHY THIS IS THREAD-LOCAL:
    /// - Ensures exactly one rotation task is active per canister instance
    /// - Prevents duplicate timers after upgrades or reinitialization
    ///
    /// Access is mediated exclusively through TimerWorkflow.
    static ROTATION_TIMER: RefCell<Option<TimerId>> = const {
        RefCell::new(None)
    };
}

///
/// DelegationWorkflow
///
/// WHY THIS MODULE EXISTS
/// ----------------------
/// This module coordinates **delegation issuance and rotation** as a workflow,
/// separating *orchestration* from:
/// - cryptographic operations
/// - storage details
/// - authorization policy
///
/// Responsibilities:
/// - Call cryptographic primitives in the correct order
/// - Coordinate persistence and publication
/// - Schedule and manage rotation timers
///
/// Explicit non-responsibilities:
/// - Authorization (caller must enforce)
/// - Validation (delegation certs are assumed valid inputs)
/// - Retry or recovery logic
/// - Token verification
///
/// This separation ensures delegation remains auditable and predictable.
///

pub struct DelegationWorkflow;

impl DelegationWorkflow {
    // -------------------------------------------------------------------------
    // Issuance
    // -------------------------------------------------------------------------

    /// Issue a root-signed delegation proof for a delegated signer key.
    ///
    /// WHAT THIS DOES:
    /// - Signs the provided DelegationCert using the root authority
    /// - Produces a DelegationProof suitable for verification
    ///
    /// WHAT THIS DOES NOT DO:
    /// - Persist the proof
    /// - Validate cert contents
    /// - Enforce caller authority
    ///
    /// WHY:
    /// - Keeps cryptographic issuance separable from storage and policy
    ///
    /// Authority MUST be enforced by the caller.
    pub fn issue_delegation(cert: DelegationCert) -> Result<DelegationProof, InternalError> {
        DelegatedTokenOps::sign_delegation_cert(cert)
    }

    /// Issue and persist the delegation proof in stable state.
    ///
    /// Semantics:
    /// - Any previously stored delegation proof is replaced
    /// - All previously issued delegated tokens become invalid
    ///
    /// Intended usage:
    /// - Initial delegation bootstrap
    /// - Controlled, immediate rotation
    ///
    /// Authority MUST be enforced by the caller.
    pub fn issue_and_store(cert: DelegationCert) -> Result<DelegationProof, InternalError> {
        let proof = Self::issue_delegation(cert)?;
        DelegationStateOps::set_proof_from_dto(proof.clone());

        Ok(proof)
    }

    // -------------------------------------------------------------------------
    // Provisioning
    // -------------------------------------------------------------------------

    pub async fn provision(
        request: DelegationProvisionRequest,
    ) -> Result<DelegationProvisionResponse, InternalError> {
        let proof = Self::issue_delegation(request.cert)?;
        let mut results = Vec::new();

        for target in request.signer_targets {
            let result = push_proof(target, &proof, DelegationProvisionTargetKind::Signer).await;
            results.push(result);
        }

        for target in request.verifier_targets {
            let result = push_proof(target, &proof, DelegationProvisionTargetKind::Verifier).await;
            results.push(result);
        }

        Ok(DelegationProvisionResponse { proof, results })
    }

    // -------------------------------------------------------------------------
    // Rotation
    // -------------------------------------------------------------------------

    /// Start a periodic delegation rotation task.
    ///
    /// Rotation model:
    /// - A new delegation certificate is periodically built
    /// - The certificate is signed by the root
    /// - The resulting proof is published by the caller
    ///
    /// Caller responsibilities:
    /// - Enforce authority (root-only)
    /// - Ensure key ownership and correctness
    /// - Decide how and where proofs are published
    ///
    /// Design notes:
    /// - Rotation failures are logged and skipped
    /// - No retries are performed
    /// - Partial failures do NOT stop future rotations
    ///
    /// Returns:
    /// - true if rotation was started
    /// - false if a rotation task was already running
    pub fn start_rotation(
        interval: Duration,
        build_cert: Arc<dyn Fn() -> Result<DelegationCert, InternalError> + Send + Sync>,
        publish: Arc<dyn Fn(DelegationProof) -> Result<(), InternalError> + Send + Sync>,
    ) -> bool {
        // Clone closures for initial and periodic execution.
        // This avoids capturing moved values inside async blocks.
        let init_build = build_cert.clone();
        let init_publish = publish.clone();
        let tick_build = build_cert;
        let tick_publish = publish;

        // Schedule a guarded timer:
        // - Runs immediately once
        // - Then runs periodically at the specified interval
        TimerWorkflow::set_guarded_interval(
            &ROTATION_TIMER,
            Duration::ZERO,
            "delegation:rotate:init",
            move || async move {
                Self::rotate_once(init_build.as_ref(), init_publish.as_ref());
            },
            interval,
            "delegation:rotate:interval",
            move || {
                let build = tick_build.clone();
                let publish = tick_publish.clone();
                async move {
                    Self::rotate_once(build.as_ref(), publish.as_ref());
                }
            },
        )
    }

    async fn push_proof(
        target: Principal,
        proof: &DelegationProof,
        kind: DelegationProvisionTargetKind,
    ) -> DelegationProvisionTargetResponse {
        let method = match kind {
            DelegationProvisionTargetKind::Signer => protocol::CANIC_DELEGATION_SET_SIGNER_PROOF,
            DelegationProvisionTargetKind::Verifier => {
                protocol::CANIC_DELEGATION_SET_VERIFIER_PROOF
            }
        };

        let call = match CallOps::unbounded_wait(target, method).with_arg(proof.clone()) {
            Ok(call) => call,
            Err(err) => {
                return failure(target, kind, ErrorDto::from(err));
            }
        };

        let result = match call.execute().await {
            Ok(result) => result,
            Err(err) => {
                return failure(target, kind, ErrorDto::from(err));
            }
        };

        let response: Result<(), ErrorDto> = match result.candid() {
            Ok(response) => response,
            Err(err) => {
                return failure(target, kind, ErrorDto::from(err));
            }
        };

        match response {
            Ok(()) => DelegationProvisionTargetResponse {
                target,
                kind,
                status: DelegationProvisionStatus::Ok,
                error: None,
            },
            Err(err) => failure(target, kind, err),
        }
    }

    fn failure(
        target: Principal,
        kind: DelegationProvisionTargetKind,
        err: ErrorDto,
    ) -> DelegationProvisionTargetResponse {
        DelegationProvisionTargetResponse {
            target,
            kind,
            status: DelegationProvisionStatus::Failed,
            error: Some(err),
        }
    }

    /// Stop the periodic delegation rotation task.
    ///
    /// Semantics:
    /// - No further rotations will occur
    /// - The currently active delegation proof remains valid
    ///
    /// Returns:
    /// - true if a task was stopped
    /// - false if no rotation task was running
    pub fn stop_rotation() -> bool {
        TimerWorkflow::clear_guarded(&ROTATION_TIMER)
    }

    /// Execute a single delegation rotation step.
    ///
    /// This function is intentionally:
    /// - synchronous in control flow
    /// - non-panicking
    /// - failure-tolerant
    ///
    /// Any failure results in a logged warning and early return.
    /// The rotation schedule continues unaffected.
    fn rotate_once(
        build_cert: &dyn Fn() -> Result<DelegationCert, InternalError>,
        publish: &dyn Fn(DelegationProof) -> Result<(), InternalError>,
    ) {
        let cert = match build_cert() {
            Ok(cert) => cert,
            Err(err) => {
                log!(Topic::Auth, Warn, "delegation rotation build failed: {err}");
                return;
            }
        };

        let proof = match Self::issue_delegation(cert) {
            Ok(proof) => proof,
            Err(err) => {
                log!(
                    Topic::Auth,
                    Warn,
                    "delegation rotation signing failed: {err}"
                );
                return;
            }
        };

        if let Err(err) = publish(proof) {
            log!(
                Topic::Auth,
                Warn,
                "delegation rotation publish failed: {err}"
            );
        }
    }
}
