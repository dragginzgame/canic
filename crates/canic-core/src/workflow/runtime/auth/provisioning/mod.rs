//! Module: workflow::runtime::auth::provisioning
//!
//! Responsibility: orchestrate root-triggered delegated-auth proof provisioning.
//! Does not own: endpoint authorization, proof storage, or proof verification.
//! Boundary: root auth API calls this to validate pending proof batches and
//! broadcast signer-local install requests.

use super::RuntimeAuthWorkflow;
use crate::{
    InternalError,
    dto::{
        auth::{
            InstallActiveDelegationProofRequest, InstallActiveDelegationProofResponse,
            RootDelegationProofBatchInstallRequest, RootDelegationProofBatchInstallResponse,
            RootDelegationProofBatchInstallResult, RootDelegationProofInstallOutcome,
        },
        error::Error,
    },
    ops::{
        auth::AuthOps,
        ic::{
            IcOps,
            call::{CallOps, CallResult},
        },
        runtime::env::EnvOps,
    },
    protocol,
    workflow::prelude::*,
};
use std::future::Future;

impl RuntimeAuthWorkflow {
    /// Install retrieved root delegation proofs on issuer canisters.
    pub async fn install_delegation_proof_batch_root(
        request: RootDelegationProofBatchInstallRequest,
    ) -> Result<RootDelegationProofBatchInstallResponse, InternalError> {
        EnvOps::require_root()?;
        let now_ns = IcOps::now_nanos();
        install_delegation_proof_batch_with_signer_install(
            request,
            now_ns,
            install_delegation_proof_on_signer,
        )
        .await
    }
}

async fn install_delegation_proof_batch_with_signer_install<F, Fut>(
    request: RootDelegationProofBatchInstallRequest,
    now_ns: u64,
    mut install_signer: F,
) -> Result<RootDelegationProofBatchInstallResponse, InternalError>
where
    F: FnMut(Principal, InstallActiveDelegationProofRequest) -> Fut,
    Fut: Future<Output = RootDelegationProofInstallOutcome>,
{
    if request.proofs.is_empty() {
        return Err(InternalError::public(Error::invalid(
            "root delegation proof batch install must contain at least one proof",
        )));
    }

    let mut outcomes = Vec::with_capacity(request.proofs.len());
    for proof in request.proofs {
        let issuer_pid = proof.issuer_pid;
        let cert_hash = proof.cert_hash;
        let outcome = match AuthOps::preflight_delegation_proof_batch_install_proof(
            request.batch_id,
            &proof,
            now_ns,
        ) {
            Ok(()) => {
                let outcome = install_signer(
                    issuer_pid,
                    InstallActiveDelegationProofRequest { proof: proof.proof },
                )
                .await;
                if outcome == RootDelegationProofInstallOutcome::Installed {
                    AuthOps::mark_delegation_proof_batch_installed(
                        request.batch_id,
                        issuer_pid,
                        cert_hash,
                    );
                }
                outcome
            }
            Err(outcome) => outcome,
        };
        outcomes.push(RootDelegationProofBatchInstallResult {
            issuer_pid,
            cert_hash,
            outcome,
        });
    }

    Ok(RootDelegationProofBatchInstallResponse {
        batch_id: request.batch_id,
        outcomes,
    })
}

async fn install_delegation_proof_on_signer(
    issuer_pid: Principal,
    request: InstallActiveDelegationProofRequest,
) -> RootDelegationProofInstallOutcome {
    let Ok(builder) =
        CallOps::unbounded_wait(issuer_pid, protocol::CANIC_INSTALL_ACTIVE_DELEGATION_PROOF)
            .with_arg(request)
    else {
        return RootDelegationProofInstallOutcome::CallFailed;
    };
    let Ok(call) = builder.execute().await else {
        return RootDelegationProofInstallOutcome::CallFailed;
    };
    signer_install_outcome(call)
}

fn signer_install_outcome(call: CallResult) -> RootDelegationProofInstallOutcome {
    let result: Result<InstallActiveDelegationProofResponse, Error> = match call.candid() {
        Ok(result) => result,
        Err(_) => return RootDelegationProofInstallOutcome::CallFailed,
    };
    match result {
        Ok(_) => RootDelegationProofInstallOutcome::Installed,
        Err(_) => RootDelegationProofInstallOutcome::RejectedBySigner,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cdk::types::Principal,
        dto::auth::{
            DelegatedRoleGrant, DelegationAudience, DelegationCert, DelegationProof,
            IcCanisterSignatureProofV1, IssuerProofAlgorithm, IssuerProofBinding,
            RootDelegationProofBatchProof, RootProof,
        },
        ids::{CanisterRole, cap},
    };
    use futures::executor::block_on;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn proof() -> RootDelegationProofBatchProof {
        RootDelegationProofBatchProof {
            issuer_pid: p(2),
            cert_hash: [3; 32],
            proof: DelegationProof {
                cert: DelegationCert {
                    root_pid: p(1),
                    issuer_pid: p(2),
                    issuer_proof_alg: IssuerProofAlgorithm::IcCanisterSignatureV1,
                    issuer_proof_binding_hash: [4; 32],
                    issuer_proof_binding: IssuerProofBinding::IcCanisterSignatureV1 {
                        seed_hash: [5; 32],
                    },
                    issued_at_ns: 10,
                    not_before_ns: 10,
                    expires_at_ns: 100,
                    max_token_ttl_ns: 30,
                    aud: DelegationAudience::Project("test".to_string()),
                    grants: vec![DelegatedRoleGrant {
                        target: CanisterRole::owned("project_instance".to_string()),
                        scopes: vec![cap::READ.to_string()],
                    }],
                },
                root_proof: RootProof::IcCanisterSignatureV1(IcCanisterSignatureProofV1 {
                    signature_cbor: vec![8; 64],
                    public_key_der: vec![9; 32],
                }),
            },
        }
    }

    #[test]
    fn install_batch_rejects_empty_request() {
        let err = block_on(install_delegation_proof_batch_with_signer_install(
            RootDelegationProofBatchInstallRequest {
                batch_id: [1; 32],
                proofs: vec![],
            },
            20,
            |_issuer_pid, _request| async { RootDelegationProofInstallOutcome::Installed },
        ))
        .expect_err("empty install batch must fail");
        let public = err.public_error().expect("public install error");

        assert_eq!(public.code, crate::dto::error::ErrorCode::InvalidInput);
    }

    #[test]
    fn install_batch_does_not_call_signer_when_local_validation_fails() {
        let response = block_on(install_delegation_proof_batch_with_signer_install(
            RootDelegationProofBatchInstallRequest {
                batch_id: [2; 32],
                proofs: vec![proof()],
            },
            20,
            |_issuer_pid, _request| async { panic!("invalid local proof must not be broadcast") },
        ))
        .expect("invalid proof should be returned as a per-signer outcome");

        assert_eq!(response.batch_id, [2; 32]);
        assert_eq!(response.outcomes.len(), 1);
        assert_eq!(response.outcomes[0].issuer_pid, p(2));
        assert_eq!(
            response.outcomes[0].outcome,
            RootDelegationProofInstallOutcome::ProofMismatch
        );
    }
}
