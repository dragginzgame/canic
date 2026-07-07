//! Module: workflow::runtime::auth::provisioning
//!
//! Responsibility: orchestrate root-triggered delegated-auth proof provisioning.
//! Does not own: endpoint authorization, proof storage, or proof verification.
//! Boundary: root auth API calls this to validate pending proof batches and
//! broadcast issuer-local install requests.

use crate::{
    InternalError, InternalErrorOrigin,
    cdk::types::Principal,
    config::schema::DelegatedTokenConfig,
    domain::auth::DelegatedAuthNetwork,
    dto::{
        auth::{
            InstallActiveDelegationProofRequest, InstallActiveDelegationProofResponse,
            RootDelegationProofBatchInstallRequest, RootDelegationProofBatchProof,
            RootDelegationProofInstallOutcome,
        },
        error::Error,
    },
    ids::BuildNetwork,
    ops::{
        auth::AuthOps,
        ic::{
            IcOps,
            call::{CallOps, CallResult},
        },
        runtime::env::EnvOps,
    },
    protocol,
    workflow::runtime::auth::RuntimeAuthWorkflow,
};
use std::future::Future;

impl RuntimeAuthWorkflow {
    /// Return or create one chain-key root delegation proof for the calling issuer.
    pub async fn get_or_create_chain_key_delegation_proof_for_issuer_root(
        issuer_pid: Principal,
    ) -> Result<RootDelegationProofBatchProof, InternalError> {
        EnvOps::require_root()?;
        let config = crate::ops::config::ConfigOps::delegated_tokens_config()?;
        require_chain_key_root_proof_mode(&config)?;
        let build_network = build_network_from_delegated_auth_config(&config)?;
        let max_cert_ttl_ns = delegated_token_max_ttl_ns(&config)?;
        let min_accepted_proof_epoch = chain_key_min_accepted_proof_epoch(&config)?;
        let now_ns = IcOps::now_nanos();

        AuthOps::get_or_create_chain_key_delegation_proof_for_issuer(
            issuer_pid,
            build_network,
            max_cert_ttl_ns,
            min_accepted_proof_epoch,
            now_ns,
        )
        .await?
        .ok_or_else(|| {
            InternalError::public(Error::unavailable(
                "chain-key root delegation proof is not available yet; retry",
            ))
        })
    }
}

pub(super) async fn install_chain_key_delegation_proof_batch(
    request: RootDelegationProofBatchInstallRequest,
    now_ns: u64,
) -> Result<bool, InternalError> {
    install_chain_key_delegation_proof_batch_with_issuer_install(
        request,
        now_ns,
        install_delegation_proof_on_issuer,
    )
    .await
}

async fn install_chain_key_delegation_proof_batch_with_issuer_install<F, Fut>(
    request: RootDelegationProofBatchInstallRequest,
    now_ns: u64,
    mut install_issuer: F,
) -> Result<bool, InternalError>
where
    F: FnMut(Principal, InstallActiveDelegationProofRequest) -> Fut,
    Fut: Future<Output = RootDelegationProofInstallOutcome>,
{
    let mut installed_any = false;
    for proof in request.proofs {
        let issuer_pid = proof.issuer_pid;
        let cert_hash = proof.cert_hash;
        let outcome = install_issuer(
            issuer_pid,
            InstallActiveDelegationProofRequest { proof: proof.proof },
        )
        .await;
        match outcome {
            RootDelegationProofInstallOutcome::Installed
            | RootDelegationProofInstallOutcome::AlreadyInstalled => {
                installed_any = AuthOps::record_chain_key_root_delegation_install_success(
                    request.batch_id,
                    issuer_pid,
                    cert_hash,
                    now_ns,
                ) || installed_any;
            }
            outcome => {
                AuthOps::record_chain_key_root_delegation_install_failure(
                    request.batch_id,
                    issuer_pid,
                    cert_hash,
                    outcome,
                );
            }
        }
    }
    Ok(installed_any)
}

async fn install_delegation_proof_on_issuer(
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
    issuer_install_outcome(call)
}

fn issuer_install_outcome(call: CallResult) -> RootDelegationProofInstallOutcome {
    let result: Result<InstallActiveDelegationProofResponse, Error> = match call.candid() {
        Ok(result) => result,
        Err(_) => return RootDelegationProofInstallOutcome::CallFailed,
    };
    match result {
        Ok(_) => RootDelegationProofInstallOutcome::Installed,
        Err(_) => RootDelegationProofInstallOutcome::RejectedBySigner,
    }
}

fn require_chain_key_root_proof_mode(config: &DelegatedTokenConfig) -> Result<(), InternalError> {
    if config.root_proof_mode.trim() == "chain_key_batch" {
        return Ok(());
    }
    Err(InternalError::invariant(
        InternalErrorOrigin::Workflow,
        "0.76 delegated-auth lazy repair requires root_proof_mode=\"chain_key_batch\"",
    ))
}

fn build_network_from_delegated_auth_config(
    config: &DelegatedTokenConfig,
) -> Result<BuildNetwork, InternalError> {
    let network = DelegatedAuthNetwork::parse(config.network.trim()).ok_or_else(|| {
        InternalError::invalid_input(
            "auth.delegated_tokens.network must be one of mainnet, local, pocketic, testnet",
        )
    })?;
    if network.is_mainnet() {
        Ok(BuildNetwork::Ic)
    } else {
        Ok(BuildNetwork::Local)
    }
}

fn delegated_token_max_ttl_ns(config: &DelegatedTokenConfig) -> Result<u64, InternalError> {
    let max_ttl_secs = config.max_ttl_secs.unwrap_or(24 * 60 * 60);
    max_ttl_secs.checked_mul(1_000_000_000).ok_or_else(|| {
        InternalError::invalid_input("auth.delegated_tokens.max_ttl_secs overflows nanoseconds")
    })
}

fn chain_key_min_accepted_proof_epoch(config: &DelegatedTokenConfig) -> Result<u64, InternalError> {
    config
        .chain_key_root_proof
        .min_accepted_proof_epoch
        .ok_or_else(|| {
            InternalError::invariant(
                InternalErrorOrigin::Workflow,
                "auth.delegated_tokens.chain_key_root_proof.min_accepted_proof_epoch is required for chain-key lazy repair",
            )
        })
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cdk::types::Principal,
        dto::auth::{
            DelegatedRoleGrant, DelegationAudience, DelegationCert, DelegationProof,
            IssuerProofAlgorithm, IssuerProofBinding, RootDelegationProofBatchProof,
        },
        ids::{CanisterRole, cap},
    };
    use futures::executor::block_on;
    use std::cell::Cell;

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
                root_proof: crate::ops::auth::test_fixtures::chain_key_root_proof(8),
            },
        }
    }

    #[test]
    fn install_chain_key_batch_empty_request_is_noop() {
        let installed = block_on(
            install_chain_key_delegation_proof_batch_with_issuer_install(
                RootDelegationProofBatchInstallRequest {
                    batch_id: [1; 32],
                    proofs: vec![],
                },
                20,
                |_issuer_pid, _request| async { RootDelegationProofInstallOutcome::Installed },
            ),
        )
        .expect("empty chain-key install batch should be a no-op");

        assert!(!installed);
    }

    #[test]
    fn install_chain_key_batch_broadcasts_proofs_to_issuers() {
        let calls = Cell::new(0);
        let installed = block_on(
            install_chain_key_delegation_proof_batch_with_issuer_install(
                RootDelegationProofBatchInstallRequest {
                    batch_id: [2; 32],
                    proofs: vec![proof()],
                },
                20,
                |issuer_pid, _request| {
                    assert_eq!(issuer_pid, p(2));
                    calls.set(calls.get() + 1);
                    async { RootDelegationProofInstallOutcome::CallFailed }
                },
            ),
        )
        .expect("chain-key proof should be broadcast to the issuer");

        assert_eq!(calls.get(), 1);
        assert!(!installed);
    }
}
