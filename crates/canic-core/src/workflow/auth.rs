//! Delegation issuance workflow.
//!
//! This module defines the **operational workflow** for:
//! - issuing delegated signing authority
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
    ops::{auth::DelegatedTokenOps, ic::call::CallOps},
    protocol,
};

///
/// DelegationWorkflow
///
/// WHY THIS MODULE EXISTS
/// ----------------------
/// This module coordinates **delegation issuance** as a workflow,
/// separating *orchestration* from:
/// - cryptographic operations
/// - storage details
/// - authorization policy
///
/// Responsibilities:
/// - Call cryptographic primitives in the correct order
/// - Coordinate persistence and publication
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

// -------------------------------------------------------------------------
// Logging context
// -------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub enum DelegationPushOrigin {
    Provisioning,
}

impl DelegationPushOrigin {
    const fn label(self) -> &'static str {
        match self {
            Self::Provisioning => "provisioning",
        }
    }
}

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
    fn issue_delegation(cert: DelegationCert) -> Result<DelegationProof, InternalError> {
        DelegatedTokenOps::sign_delegation_cert(cert)
    }

    // -------------------------------------------------------------------------
    // Provisioning
    // -------------------------------------------------------------------------

    pub(crate) async fn provision(
        request: DelegationProvisionRequest,
    ) -> Result<DelegationProvisionResponse, InternalError> {
        let proof = Self::issue_delegation(request.cert)?;
        log!(
            Topic::Auth,
            Info,
            "delegation provision issued proof signer={} issued_at={} expires_at={}",
            proof.cert.signer_pid,
            proof.cert.issued_at,
            proof.cert.expires_at
        );
        let mut results = Vec::new();

        for target in request.signer_targets {
            let result = Self::push_proof(
                target,
                &proof,
                DelegationProvisionTargetKind::Signer,
                DelegationPushOrigin::Provisioning,
            )
            .await;
            results.push(result);
        }

        for target in request.verifier_targets {
            let result = Self::push_proof(
                target,
                &proof,
                DelegationProvisionTargetKind::Verifier,
                DelegationPushOrigin::Provisioning,
            )
            .await;
            results.push(result);
        }

        Ok(DelegationProvisionResponse { proof, results })
    }

    pub(crate) async fn push_proof(
        target: Principal,
        proof: &DelegationProof,
        kind: DelegationProvisionTargetKind,
        origin: DelegationPushOrigin,
    ) -> DelegationProvisionTargetResponse {
        log!(
            Topic::Auth,
            Info,
            "delegation push attempt origin={} kind={:?} target={} signer={} issued_at={} expires_at={}",
            origin.label(),
            kind,
            target,
            proof.cert.signer_pid,
            proof.cert.issued_at,
            proof.cert.expires_at
        );

        let method = match kind {
            DelegationProvisionTargetKind::Signer => protocol::CANIC_DELEGATION_SET_SIGNER_PROOF,
            DelegationProvisionTargetKind::Verifier => {
                protocol::CANIC_DELEGATION_SET_VERIFIER_PROOF
            }
        };

        let call = match CallOps::unbounded_wait(target, method).with_arg(proof.clone()) {
            Ok(call) => call,
            Err(err) => {
                let response = Self::failure(target, kind, ErrorDto::from(err));
                Self::log_push_result(&response, origin);
                return response;
            }
        };

        let result = match call.execute().await {
            Ok(result) => result,
            Err(err) => {
                let response = Self::failure(target, kind, ErrorDto::from(err));
                Self::log_push_result(&response, origin);
                return response;
            }
        };

        let response: Result<(), ErrorDto> = match result.candid() {
            Ok(response) => response,
            Err(err) => {
                let response = Self::failure(target, kind, ErrorDto::from(err));
                Self::log_push_result(&response, origin);
                return response;
            }
        };

        let response = match response {
            Ok(()) => DelegationProvisionTargetResponse {
                target,
                kind,
                status: DelegationProvisionStatus::Ok,
                error: None,
            },
            Err(err) => Self::failure(target, kind, err),
        };

        Self::log_push_result(&response, origin);
        response
    }

    const fn failure(
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

    fn log_push_result(response: &DelegationProvisionTargetResponse, origin: DelegationPushOrigin) {
        match response.status {
            DelegationProvisionStatus::Ok => {
                log!(
                    Topic::Auth,
                    Info,
                    "delegation push ok origin={} kind={:?} target={}",
                    origin.label(),
                    response.kind,
                    response.target
                );
            }
            DelegationProvisionStatus::Failed => {
                let err = response
                    .error
                    .as_ref()
                    .map_or_else(|| "unknown error".to_string(), ToString::to_string);
                log!(
                    Topic::Auth,
                    Warn,
                    "delegation push failed origin={} kind={:?} target={} error={}",
                    origin.label(),
                    response.kind,
                    response.target,
                    err
                );
            }
        }
    }
}
