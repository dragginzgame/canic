//! Module: workflow::runtime::auth::provisioning
//!
//! Responsibility: orchestrate root-triggered delegated-auth proof provisioning.
//! Does not own: endpoint authorization, proof storage, or proof verification.
//! Boundary: root auth API calls this to validate pending proof batches and
//! broadcast issuer-local install requests.

use crate::{
    InternalError,
    cdk::types::Principal,
    config::schema::DelegatedTokenConfig,
    diagnostics::codes,
    dto::{
        auth::{
            InstallActiveDelegationProofRequest, InstallActiveDelegationProofResponse,
            RootDelegationProofBatchProof, RootProof,
        },
        error::Error,
    },
    model::auth::ChainKeyRootDelegationInstallFailure,
    ops::{
        auth::{
            AuthOps, ChainKeyRootDelegationBatchInstallPlan,
            PrepareChainKeyRootDelegationBatchInput,
        },
        ic::{IcOps, call::execute as execute_call},
        runtime::env::EnvOps,
    },
    workflow::runtime::auth::{RuntimeAuthWorkflow, root_delegation_batch},
};
use ic_call::{Call, WaitPolicy};
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
            IcOps::now_nanos,
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
            return Err(InternalError::auth_proof_pending());
        };
        let signing =
            AuthOps::sign_chain_key_root_delegation_batch(build_network, batch_id, now_ns).await?;
        crate::perf!("root_proof_sign_batch");
        if signing.signing_in_flight {
            return Err(InternalError::auth_proof_pending());
        }

        let proof =
            AuthOps::signed_chain_key_delegation_proof_for_issuer(issuer_pid, IcOps::now_nanos())?
                .ok_or_else(InternalError::auth_proof_pending)?;
        crate::perf!("root_proof_load_issuer");
        Ok(proof)
    }
}

pub(super) async fn install_chain_key_delegation_proof_batch(
    plan: ChainKeyRootDelegationBatchInstallPlan,
) -> ChainKeyDelegationProofBatchInstallOutcome {
    let result = install_chain_key_delegation_proofs(
        plan.batch_id,
        plan.proofs,
        IcOps::now_nanos,
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
    completion_clock: impl Fn() -> u64,
    mut install_issuer: F,
) -> ChainKeyDelegationProofBatchInstallResult
where
    F: FnMut(Principal, InstallActiveDelegationProofRequest) -> Fut,
    Fut: Future<Output = Result<InstallActiveDelegationProofResponse, IssuerProofInstallError>>,
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
        let result = result.and_then(|response| {
            if AuthOps::record_chain_key_root_delegation_install_success(
                batch_id,
                issuer_pid,
                &response.active_proof,
                completion_clock(),
            ) {
                Ok(())
            } else {
                Err(IssuerProofInstallError::InvalidResponse)
            }
        });
        match result {
            Ok(()) => installed_count = installed_count.saturating_add(1),
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
) -> Result<InstallActiveDelegationProofResponse, IssuerProofInstallError> {
    use canic_contracts::{dto::issuer_proof as native, protocol::issuer_proof as wire};
    let request: native::InstallActiveDelegationProofRequest = request.into();
    let bytes = ic_wire::native::encode_request(wire::INSTALL_ID, &request, wire::REQUEST_LIMITS)
        .map_err(|_| IssuerProofInstallError::RequestEncoding)?;
    let bytes = execute_call(
        Call::new(issuer_pid.into(), wire::INSTALL, WaitPolicy::Unbounded).take_raw_args(bytes),
    )
    .await
    .map_err(IssuerProofInstallError::Transport)?
    .into_bytes();
    issuer_install_outcome(&bytes)
}

fn issuer_install_outcome(
    bytes: &[u8],
) -> Result<InstallActiveDelegationProofResponse, IssuerProofInstallError> {
    use canic_contracts::{dto::issuer_proof as native, protocol::issuer_proof as wire};
    let result: Result<native::InstallActiveDelegationProofResponse, Error> =
        ic_wire::native::decode_response(wire::INSTALL_ID, bytes, wire::RESPONSE_LIMITS)
            .map_err(|_| IssuerProofInstallError::InvalidResponse)?;
    issuer_install_response(result.map(Into::into))
}

fn issuer_install_response(
    result: Result<InstallActiveDelegationProofResponse, Error>,
) -> Result<InstallActiveDelegationProofResponse, IssuerProofInstallError> {
    match result {
        Ok(response) => Ok(response),
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
            None => Err(InternalError::public(
                crate::diagnostics::codes::SECURITY_UNAVAILABLE,
            )),
        }
    }
}

enum IssuerProofInstallError {
    RequestEncoding,
    Transport(InternalError),
    InvalidResponse,
    RejectedByIssuer(Error),
}

impl IssuerProofInstallError {
    const fn record_failure(&self) -> ChainKeyRootDelegationInstallFailure {
        match self {
            Self::RequestEncoding | Self::Transport(_) | Self::InvalidResponse => {
                ChainKeyRootDelegationInstallFailure::CallFailed
            }
            Self::RejectedByIssuer(err) => match err.raw_code() {
                code if code == codes::AUTH_CERT_EXPIRED.raw_code().raw() => {
                    ChainKeyRootDelegationInstallFailure::ExpiredOrSuperseded
                }
                code if code == codes::SECURITY_CONFLICT.raw_code().raw()
                    || code == codes::SECURITY_UNAVAILABLE.raw_code().raw()
                    || code == codes::REQUEST_INVALID.raw_code().raw() =>
                {
                    ChainKeyRootDelegationInstallFailure::ProofMismatch
                }
                _ => ChainKeyRootDelegationInstallFailure::RejectedBySigner,
            },
        }
    }

    fn into_internal_error(self, _issuer_pid: Principal) -> InternalError {
        match self {
            Self::RequestEncoding | Self::InvalidResponse => {
                InternalError::projected(codes::CODEC_FAILED, codes::CONTROL_PLANE_STATE_INVALID)
            }
            Self::Transport(cause) => {
                InternalError::projected(cause.code(), codes::PLATFORM_UNAVAILABLE)
            }
            Self::RejectedByIssuer(err) => InternalError::observed_public(err),
        }
    }

    fn into_renewal_error(self) -> InternalError {
        match self {
            Self::RequestEncoding | Self::InvalidResponse => {
                InternalError::projected(codes::CODEC_FAILED, codes::CONTROL_PLANE_STATE_INVALID)
            }
            Self::Transport(cause) => {
                InternalError::projected(cause.code(), codes::PLATFORM_UNAVAILABLE)
            }
            Self::RejectedByIssuer(err) => InternalError::observed_public(err),
        }
    }
}

fn delegated_token_max_ttl_ns(config: &DelegatedTokenConfig) -> Result<u64, InternalError> {
    let max_ttl_secs = config.max_ttl_secs.unwrap_or(24 * 60 * 60);
    max_ttl_secs
        .checked_mul(1_000_000_000)
        .ok_or_else(|| InternalError::public(codes::TIME_CAPACITY))
}

fn chain_key_min_accepted_proof_epoch(config: &DelegatedTokenConfig) -> Result<u64, InternalError> {
    config
        .chain_key_root_proof
        .min_accepted_proof_epoch
        .ok_or_else(|| InternalError::public(codes::CONFIGURATION_UNAVAILABLE))
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
    };
    use canic_contracts::ids::{CanisterRole, cap};
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
                    aud: DelegationAudience::Fleet(crate::test::support::fleet_key(1)),
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
    fn native_installation_preserves_signed_proof_and_original_installation_evidence() {
        use canic_contracts::{dto::issuer_proof as native, protocol::issuer_proof as wire};
        let request = InstallActiveDelegationProofRequest {
            proof: proof().proof,
        };
        let mut canonical = Vec::new();
        serde_cbor::to_writer(&mut canonical, &request).unwrap();
        let frame = ic_wire::native::encode_request(
            wire::INSTALL_ID,
            &native::InstallActiveDelegationProofRequest::from(request.clone()),
            wire::REQUEST_LIMITS,
        )
        .unwrap();
        let decoded: native::InstallActiveDelegationProofRequest =
            ic_wire::native::decode_request(wire::INSTALL_ID, &frame, wire::REQUEST_LIMITS)
                .unwrap();
        let restored = InstallActiveDelegationProofRequest::from(decoded);
        assert_eq!(restored, request);
        let mut restored_canonical = Vec::new();
        serde_cbor::to_writer(&mut restored_canonical, &restored).unwrap();
        assert_eq!(restored_canonical, canonical);
        let expected = InstallActiveDelegationProofResponse {
            active_proof: crate::dto::auth::ActiveDelegationProof {
                proof: restored.proof,
                cert_hash: [9; 32],
                not_before_ns: 10,
                expires_at_ns: 100,
                refresh_after_ns: 80,
                installed_at_ns: 11,
                installed_by: p(1),
            },
        };
        let frame = ic_wire::native::encode_response(
            wire::INSTALL_ID,
            &Ok::<_, Error>(native::InstallActiveDelegationProofResponse::from(
                expected.clone(),
            )),
            wire::RESPONSE_LIMITS,
        )
        .unwrap();
        assert!(matches!(issuer_install_outcome(&frame), Ok(actual) if actual == expected));
    }

    #[test]
    fn native_installation_keeps_rejection_distinct_from_malformed_response() {
        use canic_contracts::{dto::issuer_proof as native, protocol::issuer_proof as wire};
        let rejection: Result<native::InstallActiveDelegationProofResponse, Error> =
            Err(Error::from_registered(codes::AUTH_CERT_EXPIRED));
        let mut frame =
            ic_wire::native::encode_response(wire::INSTALL_ID, &rejection, wire::RESPONSE_LIMITS)
                .unwrap();
        assert!(matches!(
            issuer_install_outcome(&frame),
            Err(IssuerProofInstallError::RejectedByIssuer(_))
        ));
        frame.push(0);
        assert!(matches!(
            issuer_install_outcome(&frame),
            Err(IssuerProofInstallError::InvalidResponse)
        ));
        let wrong = ic_wire::native::encode_response(
            wire::INSTALL_ID + 1,
            &rejection,
            wire::RESPONSE_LIMITS,
        )
        .unwrap();
        assert!(matches!(
            issuer_install_outcome(&wrong),
            Err(IssuerProofInstallError::InvalidResponse)
        ));
    }

    #[test]
    fn install_chain_key_batch_empty_request_is_noop() {
        let result = block_on(install_chain_key_delegation_proofs(
            [1; 32],
            vec![],
            || 20,
            |_issuer_pid, _request| async { Err(IssuerProofInstallError::InvalidResponse) },
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
            || 20,
            |issuer_pid, _request| {
                assert_eq!(issuer_pid, p(2));
                calls.set(calls.get() + 1);
                async {
                    Err(IssuerProofInstallError::Transport(
                        InternalError::platform_failure(),
                    ))
                }
            },
        ));

        assert_eq!(calls.get(), 1);
        assert_eq!(result.installed_count, 0);
        assert!(result.first_failure.is_some());
    }

    fn retain_installation(proof: &RootDelegationProofBatchProof) -> [u8; 32] {
        use crate::ops::storage::auth::{
            ChainKeyRootDelegationBatch, ChainKeyRootDelegationBatchIssuer,
            ChainKeyRootDelegationBatchStatus, RootDelegationStateOps,
        };
        let RootProof::IcChainKeyBatchSignatureV1(root) = &proof.proof.root_proof;
        let batch_id = root.header.batch_id;
        RootDelegationStateOps::upsert_chain_key_root_delegation_batch(
            ChainKeyRootDelegationBatch {
                batch_id,
                status: ChainKeyRootDelegationBatchStatus::Installing,
                header_hash: [0; 32],
                header: root.header.clone(),
                signature: Some(root.signature.clone()),
                issuers: vec![ChainKeyRootDelegationBatchIssuer {
                    issuer_pid: proof.issuer_pid,
                    cert_hash: proof.cert_hash,
                    delegation_cert: proof.proof.cert.clone(),
                    chain_key_delegation_cert: root.delegation_cert.clone(),
                    issuer_witness: root.issuer_witness.clone(),
                    refresh_after_ns: 90,
                    installed_at_ns: None,
                    last_failure: None,
                }],
                prepared_at_ns: 10,
                signed_at_ns: Some(11),
                install_started_at_ns: Some(20),
                installed_at_ns: None,
                retry_after_ns: None,
                failure: None,
            },
        );
        batch_id
    }

    #[test]
    fn installer_preserves_effective_deadlines_through_typed_callback() {
        use crate::ops::storage::auth::RootDelegationStateOps;
        let requested = proof();
        let batch_id = retain_installation(&requested);
        let result = block_on(install_chain_key_delegation_proofs(
            batch_id,
            vec![requested],
            || 30,
            |_issuer, request| async move {
                Ok(InstallActiveDelegationProofResponse {
                    active_proof: crate::dto::auth::ActiveDelegationProof {
                        proof: request.proof,
                        cert_hash: [3; 32],
                        not_before_ns: 10,
                        expires_at_ns: 60,
                        refresh_after_ns: 50,
                        installed_at_ns: 21,
                        installed_by: p(1),
                    },
                })
            },
        ));
        assert_eq!(result.installed_count, 1);
        assert!(result.first_failure.is_none());
        let state = RootDelegationStateOps::root_issuer_renewal_state(p(2)).unwrap();
        assert_eq!(state.last_installed_expires_at_ns, Some(60));
        assert_eq!(state.last_installed_refresh_after_ns, Some(50));
        assert_eq!(state.next_attempt_after_ns, 50);
        assert_eq!(state.updated_at_ns, 30);
    }

    #[test]
    fn installer_reads_completion_clock_after_await_and_rejects_expired_reply() {
        let batch_id = retain_installation(&proof());
        let clock = Cell::new(20);
        let observed = Cell::new(0);
        let result = block_on(install_chain_key_delegation_proofs(
            batch_id,
            vec![proof()],
            || {
                observed.set(clock.get());
                clock.get()
            },
            |_issuer, request| {
                clock.set(100);
                async move {
                    Ok(InstallActiveDelegationProofResponse {
                        active_proof: crate::dto::auth::ActiveDelegationProof {
                            proof: request.proof,
                            cert_hash: [3; 32],
                            not_before_ns: 10,
                            expires_at_ns: 100,
                            refresh_after_ns: 80,
                            installed_at_ns: 20,
                            installed_by: p(1),
                        },
                    })
                }
            },
        ));
        assert_eq!(observed.get(), 100);
        assert_eq!(result.installed_count, 0);
        assert!(matches!(
            result.first_failure,
            Some(IssuerProofInstallError::InvalidResponse)
        ));
    }

    #[test]
    fn explicit_provisioning_transport_failure_is_typed_as_unavailable() {
        let result = ChainKeyDelegationProofBatchInstallResult {
            installed_count: 0,
            first_failure: Some(IssuerProofInstallError::Transport(
                InternalError::platform_failure(),
            )),
        };
        let err = result
            .into_explicit_result(p(2))
            .expect_err("transport failure must reject explicit provisioning");

        assert_eq!(
            err.public_error().code(),
            crate::diagnostics::codes::PLATFORM_UNAVAILABLE.raw_code()
        );
    }

    #[test]
    fn explicit_provisioning_preserves_issuer_application_error() {
        let rejected = Error::from_registered(crate::diagnostics::codes::AUTH_CERT_EXPIRED);
        let failure = issuer_install_response(Err(rejected))
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

        assert_eq!(err.public_error(), rejected);
    }

    #[test]
    fn renewal_install_failures_preserve_exact_and_public_causes() {
        let transport = IssuerProofInstallError::Transport(InternalError::platform_failure())
            .into_renewal_error();
        assert_eq!(transport.code(), codes::PLATFORM_FAILED);
        assert_eq!(transport.public_code(), Some(codes::PLATFORM_UNAVAILABLE));

        let invalid_response = IssuerProofInstallError::InvalidResponse.into_renewal_error();
        assert_eq!(invalid_response.code(), codes::CODEC_FAILED);
        assert_eq!(
            invalid_response.public_code(),
            Some(codes::CONTROL_PLANE_STATE_INVALID)
        );

        let request_encoding = IssuerProofInstallError::RequestEncoding.into_renewal_error();
        assert_eq!(request_encoding.code(), codes::CODEC_FAILED);
        assert_eq!(
            request_encoding.public_code(),
            Some(codes::CONTROL_PLANE_STATE_INVALID)
        );

        let rejected = Error::from_registered(crate::diagnostics::codes::AUTH_CERT_EXPIRED);
        let public = IssuerProofInstallError::RejectedByIssuer(rejected).into_renewal_error();
        assert_eq!(public.public_error(), rejected);
    }
}
