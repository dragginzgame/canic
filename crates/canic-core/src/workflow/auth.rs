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
            DelegationProofInstallIntent, DelegationProvisionRequest, DelegationProvisionResponse,
            DelegationProvisionStatus, DelegationProvisionTargetKind,
            DelegationProvisionTargetResponse, DelegationVerifierProofPushResponse,
        },
        error::Error as ErrorDto,
    },
    log,
    log::Topic,
    ops::{
        auth::{DelegatedTokenOps, DelegationValidationError},
        ic::call::CallOps,
        runtime::metrics::auth::{
            DelegationProvisionRole, record_delegation_install_total,
            record_delegation_push_attempt, record_delegation_push_complete,
            record_delegation_push_failed, record_delegation_push_success,
        },
    },
    protocol,
};
use candid::{CandidType, encode_one};

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

#[derive(CandidType)]
struct DelegationProofInstallRequestRef<'a> {
    proof: &'a DelegationProof,
    intent: DelegationProofInstallIntent,
    shard_public_key_sec1: Option<&'a [u8]>,
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
        let proof_install_args = Self::encode_proof_install_request(
            &proof,
            DelegationPushOrigin::Provisioning,
            request.shard_public_key_sec1.as_deref(),
        )?;
        crate::perf!("encode_install_request");
        crate::perf!("issue_proof");
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
                &proof_install_args,
                DelegationProvisionTargetKind::Signer,
                DelegationPushOrigin::Provisioning,
            )
            .await;
            results.push(result);
        }
        crate::perf!("push_signers");

        for target in request.verifier_targets {
            let result = Self::push_proof(
                target,
                &proof,
                &proof_install_args,
                DelegationProvisionTargetKind::Verifier,
                DelegationPushOrigin::Provisioning,
            )
            .await;
            results.push(result);
        }
        crate::perf!("push_verifiers");

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
        let proof_install_args = match Self::encode_proof_install_request(proof, origin, None) {
            Ok(args) => args,
            Err(err) => {
                let err = ErrorDto::from(err);
                let results = verifier_targets
                    .into_iter()
                    .map(|target| {
                        let response = Self::failure(
                            target,
                            DelegationProvisionTargetKind::Verifier,
                            err.clone(),
                        );
                        Self::record_push_result_metric(
                            DelegationProvisionRole::Verifier,
                            origin,
                            response.status,
                        );
                        Self::log_push_result(&response, origin);
                        response
                    })
                    .collect();
                record_delegation_push_complete(origin.intent());
                return DelegationVerifierProofPushResponse { results };
            }
        };
        crate::perf!("encode_install_request");

        let mut results = Vec::new();
        for target in verifier_targets {
            let result = Self::push_proof(
                target,
                proof,
                &proof_install_args,
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
        proof_install_args: &[u8],
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

        let call =
            CallOps::unbounded_wait(target, method).with_raw_args(proof_install_args.to_vec());
        crate::perf!("prepare_call");

        let result = match call.execute().await {
            Ok(result) => result,
            Err(err) => {
                let response = Self::failure(target, kind, ErrorDto::from(err));
                Self::record_push_result_metric(role, origin, response.status);
                Self::log_push_result(&response, origin);
                return response;
            }
        };
        crate::perf!("execute_call");

        let response: Result<(), ErrorDto> = match result.candid() {
            Ok(response) => response,
            Err(err) => {
                let response = Self::failure(target, kind, ErrorDto::from(err));
                Self::record_push_result_metric(role, origin, response.status);
                Self::log_push_result(&response, origin);
                return response;
            }
        };
        crate::perf!("decode_response");

        let response = match response {
            Ok(()) => DelegationProvisionTargetResponse {
                target,
                kind,
                status: DelegationProvisionStatus::Ok,
                error: None,
            },
            Err(err) => Self::failure(target, kind, err),
        };
        crate::perf!("finalize_result");

        Self::record_push_result_metric(role, origin, response.status);
        Self::log_push_result(&response, origin);
        response
    }

    // Encode one proof-install payload once so fanout only pays transport cost.
    fn encode_proof_install_request(
        proof: &DelegationProof,
        origin: DelegationPushOrigin,
        shard_public_key_sec1: Option<&[u8]>,
    ) -> Result<Vec<u8>, InternalError> {
        let request = DelegationProofInstallRequestRef {
            proof,
            intent: origin.intent(),
            shard_public_key_sec1,
        };
        encode_one(&request).map_err(|source| {
            DelegationValidationError::EncodeFailed {
                context: "delegation proof install request",
                source,
            }
            .into()
        })
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

#[cfg(test)]
mod tests {
    use super::{DelegationProofInstallRequestRef, DelegationPushOrigin, DelegationWorkflow};
    use crate::cdk::types::Principal;
    use crate::dto::auth::{
        DelegationCert, DelegationProof, DelegationProofInstallIntent,
        DelegationProofInstallRequest,
    };
    use candid::decode_one;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    #[test]
    fn borrowed_install_request_encoding_matches_owned_request_shape() {
        let proof = DelegationProof {
            cert: DelegationCert {
                root_pid: p(1),
                shard_pid: p(2),
                issued_at: 10,
                expires_at: 20,
                scopes: vec!["verify".to_string()],
                aud: vec![p(3)],
            },
            cert_sig: vec![9, 8, 7],
        };
        let shard_public_key_sec1 = vec![4, 5, 6];

        let encoded = DelegationWorkflow::encode_proof_install_request(
            &proof,
            DelegationPushOrigin::Provisioning,
            Some(&shard_public_key_sec1),
        )
        .expect("borrowed install request must encode");

        let decoded: DelegationProofInstallRequest =
            decode_one(&encoded).expect("borrowed payload must decode into owned request");

        assert_eq!(decoded.proof.cert.root_pid, proof.cert.root_pid);
        assert_eq!(decoded.proof.cert.shard_pid, proof.cert.shard_pid);
        assert_eq!(decoded.proof.cert.issued_at, proof.cert.issued_at);
        assert_eq!(decoded.proof.cert.expires_at, proof.cert.expires_at);
        assert_eq!(decoded.proof.cert.scopes, proof.cert.scopes);
        assert_eq!(decoded.proof.cert.aud, proof.cert.aud);
        assert_eq!(decoded.proof.cert_sig, proof.cert_sig);
        assert_eq!(decoded.intent, DelegationProofInstallIntent::Provisioning);
        assert_eq!(decoded.shard_public_key_sec1, Some(shard_public_key_sec1));
    }

    #[test]
    fn borrowed_wire_shape_matches_declared_owned_wire_shape() {
        let _ = std::mem::size_of::<DelegationProofInstallRequestRef<'static>>();
    }
}
