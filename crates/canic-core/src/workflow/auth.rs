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
            DelegationAdminCommand, DelegationAdminResponse, DelegationCert, DelegationProof,
            DelegationProofInstallIntent, DelegationProofInstallRequest,
            DelegationProvisionRequest, DelegationProvisionResponse, DelegationProvisionStatus,
            DelegationProvisionTargetKind, DelegationProvisionTargetResponse,
            DelegationVerifierProofPushResponse,
        },
        error::Error as ErrorDto,
    },
    log,
    log::Topic,
    ops::{
        auth::DelegatedTokenOps,
        ic::call::CallOps,
        runtime::metrics::auth::{
            DelegationProvisionRole, record_delegation_install_total,
            record_delegation_push_attempt, record_delegation_push_complete,
            record_delegation_push_failed, record_delegation_push_success,
        },
    },
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
    Prewarm,
    Repair,
}

impl DelegationPushOrigin {
    const fn label(self) -> &'static str {
        match self {
            Self::Provisioning => "provisioning",
            Self::Prewarm => "prewarm",
            Self::Repair => "repair",
        }
    }

    const fn intent(self) -> DelegationProofInstallIntent {
        match self {
            Self::Provisioning => DelegationProofInstallIntent::Provisioning,
            Self::Prewarm => DelegationProofInstallIntent::Prewarm,
            Self::Repair => DelegationProofInstallIntent::Repair,
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
    async fn issue_delegation(cert: DelegationCert) -> Result<DelegationProof, InternalError> {
        DelegatedTokenOps::sign_delegation_cert(cert).await
    }

    // -------------------------------------------------------------------------
    // Provisioning
    // -------------------------------------------------------------------------

    pub(crate) async fn provision(
        request: DelegationProvisionRequest,
    ) -> Result<DelegationProvisionResponse, InternalError> {
        record_delegation_install_total(DelegationProofInstallIntent::Provisioning);
        let proof = Self::issue_delegation(request.cert).await?;
        log!(
            Topic::Auth,
            Info,
            "delegation provision issued proof shard={} issued_at={} expires_at={}",
            proof.cert.shard_pid,
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

        record_delegation_push_complete(DelegationProofInstallIntent::Provisioning);
        Ok(DelegationProvisionResponse { proof, results })
    }

    /// Execute explicit root-controlled verifier repair/prewarm pushes.
    pub async fn handle_admin(
        cmd: DelegationAdminCommand,
    ) -> Result<DelegationAdminResponse, InternalError> {
        match cmd {
            DelegationAdminCommand::PrewarmVerifiers(request) => {
                let result = Self::push_verifier_targets(
                    &request.proof,
                    request.verifier_targets,
                    DelegationPushOrigin::Prewarm,
                )
                .await;
                Ok(DelegationAdminResponse::PrewarmedVerifiers { result })
            }
            DelegationAdminCommand::RepairVerifiers(request) => {
                let result = Self::push_verifier_targets(
                    &request.proof,
                    request.verifier_targets,
                    DelegationPushOrigin::Repair,
                )
                .await;
                Ok(DelegationAdminResponse::RepairedVerifiers { result })
            }
        }
    }

    /// Push a validated proof to an explicit verifier target set.
    pub(crate) async fn push_verifier_targets(
        proof: &DelegationProof,
        verifier_targets: Vec<Principal>,
        origin: DelegationPushOrigin,
    ) -> DelegationVerifierProofPushResponse {
        let mut results = Vec::new();
        for target in verifier_targets {
            let result = Self::push_proof(
                target,
                proof,
                DelegationProvisionTargetKind::Verifier,
                origin,
            )
            .await;
            results.push(result);
        }

        record_delegation_push_complete(origin.intent());
        DelegationVerifierProofPushResponse { results }
    }

    pub(crate) async fn push_proof(
        target: Principal,
        proof: &DelegationProof,
        kind: DelegationProvisionTargetKind,
        origin: DelegationPushOrigin,
    ) -> DelegationProvisionTargetResponse {
        let role = Self::metric_role(kind);
        record_delegation_push_attempt(role, origin.intent());
        log!(
            Topic::Auth,
            Info,
            "delegation push attempt origin={} kind={:?} target={} shard={} issued_at={} expires_at={}",
            origin.label(),
            kind,
            target,
            proof.cert.shard_pid,
            proof.cert.issued_at,
            proof.cert.expires_at
        );

        let method = match kind {
            DelegationProvisionTargetKind::Signer => protocol::CANIC_DELEGATION_SET_SIGNER_PROOF,
            DelegationProvisionTargetKind::Verifier => {
                protocol::CANIC_DELEGATION_SET_VERIFIER_PROOF
            }
        };

        let request = DelegationProofInstallRequest {
            proof: proof.clone(),
            intent: origin.intent(),
        };
        let call = match CallOps::unbounded_wait(target, method).with_arg(request) {
            Ok(call) => call,
            Err(err) => {
                let response = Self::failure(target, kind, ErrorDto::from(err));
                Self::record_push_result_metric(role, origin, response.status);
                Self::log_push_result(&response, origin);
                return response;
            }
        };

        let result = match call.execute().await {
            Ok(result) => result,
            Err(err) => {
                let response = Self::failure(target, kind, ErrorDto::from(err));
                Self::record_push_result_metric(role, origin, response.status);
                Self::log_push_result(&response, origin);
                return response;
            }
        };

        let response: Result<(), ErrorDto> = match result.candid() {
            Ok(response) => response,
            Err(err) => {
                let response = Self::failure(target, kind, ErrorDto::from(err));
                Self::record_push_result_metric(role, origin, response.status);
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

        Self::record_push_result_metric(role, origin, response.status);
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

    const fn metric_role(kind: DelegationProvisionTargetKind) -> DelegationProvisionRole {
        match kind {
            DelegationProvisionTargetKind::Signer => DelegationProvisionRole::Signer,
            DelegationProvisionTargetKind::Verifier => DelegationProvisionRole::Verifier,
        }
    }

    fn record_push_result_metric(
        role: DelegationProvisionRole,
        origin: DelegationPushOrigin,
        status: DelegationProvisionStatus,
    ) {
        match status {
            DelegationProvisionStatus::Ok => record_delegation_push_success(role, origin.intent()),
            DelegationProvisionStatus::Failed => {
                record_delegation_push_failed(role, origin.intent());
            }
        }
    }
}
