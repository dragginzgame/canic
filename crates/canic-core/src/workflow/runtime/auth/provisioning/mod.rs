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
    dto::{
        auth::{
            InstallActiveDelegationProofRequest, InstallActiveDelegationProofResponse,
            RootDelegationProofBatchProof, RootProof,
        },
        error::{Error, ErrorCode},
    },
    model::auth::ChainKeyRootDelegationInstallFailure,
    ops::{
        auth::{
            AuthOps, ChainKeyRootDelegationBatchInstallPlan,
            PrepareChainKeyRootDelegationBatchInput,
        },
        ic::{
            IcOps,
            call::{CallOps, CallResult},
        },
        runtime::env::EnvOps,
    },
    protocol,
    workflow::runtime::auth::{RuntimeAuthWorkflow, root_delegation_batch},
};
use std::future::Future;

impl RuntimeAuthWorkflow {
    /// Create or reuse and install one chain-key root delegation proof.
    pub async fn provision_chain_key_delegation_proof_for_issuer_root(
        issuer_pid: Principal,
    ) -> Result<(), InternalError> {
        EnvOps::require_root()?;
        let proof =
            Self::get_or_create_chain_key_delegation_proof_for_issuer_root(issuer_pid).await?;
        crate::perf!("root_proof_get_or_create");
        let RootProof::IcChainKeyBatchSignatureV1(root_proof) = &proof.proof.root_proof;
        let result = install_chain_key_delegation_proofs(
            root_proof.header.batch_id,
            vec![proof],
            IcOps::now_nanos(),
            install_delegation_proof_on_issuer,
        )
        .await;
        crate::perf!("root_proof_install_batch");
        result.into_explicit_result(issuer_pid)
    }

    /// Return or create one chain-key root delegation proof for the calling issuer.
    pub async fn get_or_create_chain_key_delegation_proof_for_issuer_root(
        issuer_pid: Principal,
    ) -> Result<RootDelegationProofBatchProof, InternalError> {
        EnvOps::require_root()?;
        let config = crate::ops::config::ConfigOps::delegated_tokens_config()?;
        let build_network = config.build_network;
        let max_cert_ttl_ns = delegated_token_max_ttl_ns(&config)?;
        let min_accepted_proof_epoch = chain_key_min_accepted_proof_epoch(&config)?;
        let now_ns = IcOps::now_nanos();
        crate::perf!("root_proof_resolve_policy");

        let prepared = root_delegation_batch::prepare_due_chain_key_root_delegation_batch(
            PrepareChainKeyRootDelegationBatchInput {
                build_network,
                max_cert_ttl_ns,
                min_accepted_proof_epoch,
                required_issuer_pid: Some(issuer_pid),
                now_ns,
            },
        )?;
        crate::perf!("root_proof_prepare_batch");
        let Some(batch_id) = prepared.batch_id else {
            return Err(InternalError::auth_proof_pending(
                "chain-key root delegation proof is not available yet; retry",
            ));
        };
        let signing =
            AuthOps::sign_chain_key_root_delegation_batch(build_network, batch_id, now_ns).await?;
        crate::perf!("root_proof_sign_batch");
        if signing.signing_in_flight {
            return Err(InternalError::auth_proof_pending(
                "chain-key root delegation proof is not available yet; retry",
            ));
        }

        let proof = AuthOps::signed_chain_key_delegation_proof_for_issuer(issuer_pid, now_ns)?
            .ok_or_else(|| {
                InternalError::auth_proof_pending(
                    "chain-key root delegation proof is not available yet; retry",
                )
            })?;
        crate::perf!("root_proof_load_issuer");
        Ok(proof)
    }
}

pub(super) async fn install_chain_key_delegation_proof_batch(
    plan: ChainKeyRootDelegationBatchInstallPlan,
    now_ns: u64,
) -> ChainKeyDelegationProofBatchInstallOutcome {
    let result = install_chain_key_delegation_proofs(
        plan.batch_id,
        plan.proofs,
        now_ns,
        install_delegation_proof_on_issuer,
    )
    .await;
    ChainKeyDelegationProofBatchInstallOutcome {
        installed_count: result.installed_count,
        failure: result
            .first_failure
            .map(IssuerProofInstallError::into_renewal_error),
    }
}

async fn install_chain_key_delegation_proofs<F, Fut>(
    batch_id: [u8; 32],
    proofs: Vec<RootDelegationProofBatchProof>,
    now_ns: u64,
    mut install_issuer: F,
) -> ChainKeyDelegationProofBatchInstallResult
where
    F: FnMut(Principal, InstallActiveDelegationProofRequest) -> Fut,
    Fut: Future<Output = Result<(), IssuerProofInstallError>>,
{
    let mut installed_count = 0u64;
    let mut first_failure = None;
    for proof in proofs {
        let issuer_pid = proof.issuer_pid;
        let cert_hash = proof.cert_hash;
        let result = install_issuer(
            issuer_pid,
            InstallActiveDelegationProofRequest { proof: proof.proof },
        )
        .await;
        match result {
            Ok(()) => {
                if AuthOps::record_chain_key_root_delegation_install_success(
                    batch_id, issuer_pid, cert_hash, now_ns,
                ) {
                    installed_count = installed_count.saturating_add(1);
                }
            }
            Err(failure) => {
                AuthOps::record_chain_key_root_delegation_install_failure(
                    batch_id,
                    issuer_pid,
                    cert_hash,
                    failure.record_failure(),
                );
                if first_failure.is_none() {
                    first_failure = Some(failure);
                }
            }
        }
    }
    ChainKeyDelegationProofBatchInstallResult {
        installed_count,
        first_failure,
    }
}

async fn install_delegation_proof_on_issuer(
    issuer_pid: Principal,
    request: InstallActiveDelegationProofRequest,
) -> Result<(), IssuerProofInstallError> {
    let builder =
        CallOps::unbounded_wait(issuer_pid, protocol::CANIC_INSTALL_ACTIVE_DELEGATION_PROOF)
            .with_arg(request)
            .map_err(IssuerProofInstallError::RequestEncoding)?;
    let call = builder
        .execute()
        .await
        .map_err(IssuerProofInstallError::Transport)?;
    issuer_install_outcome(call)
}

fn issuer_install_outcome(call: CallResult) -> Result<(), IssuerProofInstallError> {
    let result: Result<InstallActiveDelegationProofResponse, Error> = call
        .candid()
        .map_err(IssuerProofInstallError::InvalidResponse)?;
    issuer_install_response(result)
}

fn issuer_install_response(
    result: Result<InstallActiveDelegationProofResponse, Error>,
) -> Result<(), IssuerProofInstallError> {
    match result {
        Ok(_) => Ok(()),
        Err(err) => Err(IssuerProofInstallError::RejectedByIssuer(err)),
    }
}

pub(super) struct ChainKeyDelegationProofBatchInstallResult {
    installed_count: u64,
    first_failure: Option<IssuerProofInstallError>,
}

pub(super) struct ChainKeyDelegationProofBatchInstallOutcome {
    pub(super) installed_count: u64,
    pub(super) failure: Option<InternalError>,
}

impl ChainKeyDelegationProofBatchInstallResult {
    fn into_explicit_result(self, issuer_pid: Principal) -> Result<(), InternalError> {
        if self.installed_count > 0 {
            return Ok(());
        }
        match self.first_failure {
            Some(failure) => Err(failure.into_internal_error(issuer_pid)),
            None => Err(InternalError::public(Error::unavailable(format!(
                "chain-key delegation proof installation for issuer {issuer_pid} did not complete"
            )))),
        }
    }
}

enum IssuerProofInstallError {
    RequestEncoding(InternalError),
    Transport(InternalError),
    InvalidResponse(InternalError),
    RejectedByIssuer(Error),
}

impl IssuerProofInstallError {
    const fn record_failure(&self) -> ChainKeyRootDelegationInstallFailure {
        match self {
            Self::RequestEncoding(_) | Self::Transport(_) | Self::InvalidResponse(_) => {
                ChainKeyRootDelegationInstallFailure::CallFailed
            }
            Self::RejectedByIssuer(err) => match err.code {
                ErrorCode::AuthProofExpired => {
                    ChainKeyRootDelegationInstallFailure::ExpiredOrSuperseded
                }
                ErrorCode::AuthMaterialStale
                | ErrorCode::AuthProofPending
                | ErrorCode::InvalidInput => ChainKeyRootDelegationInstallFailure::ProofMismatch,
                _ => ChainKeyRootDelegationInstallFailure::RejectedBySigner,
            },
        }
    }

    fn into_internal_error(self, issuer_pid: Principal) -> InternalError {
        match self {
            Self::RequestEncoding(cause) => InternalError::public(Error::internal(format!(
                "chain-key delegation proof request for issuer {issuer_pid} could not be encoded"
            )))
            .with_diagnostic_context(cause.to_string()),
            Self::Transport(cause) => InternalError::public(Error::unavailable(format!(
                "chain-key delegation proof installation transport for issuer {issuer_pid} failed"
            )))
            .with_diagnostic_context(cause.to_string()),
            Self::InvalidResponse(cause) => InternalError::public(Error::internal(format!(
                "chain-key delegation proof installation response from issuer {issuer_pid} was invalid"
            )))
            .with_diagnostic_context(cause.to_string()),
            Self::RejectedByIssuer(err) => InternalError::public(err),
        }
    }

    fn into_renewal_error(self) -> InternalError {
        match self {
            Self::RequestEncoding(cause) => InternalError::invariant(
                InternalErrorOrigin::Workflow,
                format!("chain-key delegation proof request encoding failed: {cause}"),
            ),
            Self::Transport(cause) => InternalError::infra(
                InternalErrorOrigin::Infra,
                format!("chain-key delegation proof installation transport failed: {cause}"),
            ),
            Self::InvalidResponse(cause) => InternalError::invariant(
                InternalErrorOrigin::Workflow,
                format!("chain-key delegation proof installation response was invalid: {cause}"),
            ),
            Self::RejectedByIssuer(err) => InternalError::public(err),
        }
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
        let result = block_on(install_chain_key_delegation_proofs(
            [1; 32],
            vec![],
            20,
            |_issuer_pid, _request| async { Ok(()) },
        ));

        assert_eq!(result.installed_count, 0);
        assert!(result.first_failure.is_none());
    }

    #[test]
    fn install_chain_key_batch_broadcasts_proofs_to_issuers() {
        let calls = Cell::new(0);
        let result = block_on(install_chain_key_delegation_proofs(
            [2; 32],
            vec![proof()],
            20,
            |issuer_pid, _request| {
                assert_eq!(issuer_pid, p(2));
                calls.set(calls.get() + 1);
                async {
                    Err(IssuerProofInstallError::Transport(InternalError::infra(
                        InternalErrorOrigin::Infra,
                        "transport failed",
                    )))
                }
            },
        ));

        assert_eq!(calls.get(), 1);
        assert_eq!(result.installed_count, 0);
        assert!(result.first_failure.is_some());
    }

    #[test]
    fn explicit_provisioning_transport_failure_is_typed_as_unavailable() {
        let result = ChainKeyDelegationProofBatchInstallResult {
            installed_count: 0,
            first_failure: Some(IssuerProofInstallError::Transport(InternalError::infra(
                InternalErrorOrigin::Infra,
                "transport failed",
            ))),
        };
        let err = result
            .into_explicit_result(p(2))
            .expect_err("transport failure must reject explicit provisioning");

        assert_eq!(
            err.public_error().map(|err| err.code),
            Some(crate::dto::error::ErrorCode::Unavailable)
        );
    }

    #[test]
    fn explicit_provisioning_preserves_issuer_application_error() {
        let rejected = Error::new(
            ErrorCode::AuthProofExpired,
            "issuer rejected expired proof".to_string(),
        );
        let failure = issuer_install_response(Err(rejected.clone()))
            .expect_err("issuer application rejection must remain an error");
        assert_eq!(
            failure.record_failure(),
            ChainKeyRootDelegationInstallFailure::ExpiredOrSuperseded
        );

        let result = ChainKeyDelegationProofBatchInstallResult {
            installed_count: 0,
            first_failure: Some(failure),
        };
        let err = result
            .into_explicit_result(p(2))
            .expect_err("issuer application rejection must reach the root facade");

        assert_eq!(err.public_error(), Some(&rejected));
    }

    #[test]
    fn renewal_install_failures_preserve_retry_and_terminal_classification() {
        let transport = IssuerProofInstallError::Transport(InternalError::infra(
            InternalErrorOrigin::Infra,
            "transport failed",
        ))
        .into_renewal_error();
        assert_eq!(transport.class(), crate::InternalErrorClass::Infra);
        assert_eq!(transport.origin(), InternalErrorOrigin::Infra);

        let invalid_response = IssuerProofInstallError::InvalidResponse(InternalError::infra(
            InternalErrorOrigin::Infra,
            "invalid response",
        ))
        .into_renewal_error();
        assert_eq!(
            invalid_response.class(),
            crate::InternalErrorClass::Invariant
        );
        assert_eq!(invalid_response.origin(), InternalErrorOrigin::Workflow);

        let request_encoding = IssuerProofInstallError::RequestEncoding(InternalError::infra(
            InternalErrorOrigin::Infra,
            "encoding failed",
        ))
        .into_renewal_error();
        assert_eq!(
            request_encoding.class(),
            crate::InternalErrorClass::Invariant
        );
        assert_eq!(request_encoding.origin(), InternalErrorOrigin::Workflow);

        let rejected = Error::new(
            ErrorCode::AuthProofExpired,
            "issuer rejected expired proof".to_string(),
        );
        let public =
            IssuerProofInstallError::RejectedByIssuer(rejected.clone()).into_renewal_error();
        assert_eq!(public.public_error(), Some(&rejected));
    }
}
