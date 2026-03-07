use crate::{
    cdk::types::Principal,
    dto::{
        auth::{
            AttestationKeySet, DelegatedToken, DelegatedTokenClaims, DelegationCert,
            DelegationProof, DelegationProvisionResponse, DelegationProvisionTargetKind,
            DelegationRequest, RoleAttestationRequest, SignedRoleAttestation,
        },
        error::Error,
        rpc::{
            Request as RootCapabilityRequest, Response as RootCapabilityResponse,
            RootRequestMetadata,
        },
    },
    error::InternalErrorClass,
    log,
    log::Topic,
    ops::{
        auth::{DelegatedTokenOps, DelegatedTokenOpsError},
        config::ConfigOps,
        ic::IcOps,
        rpc::RpcOps,
        runtime::env::EnvOps,
        runtime::metrics::auth::{
            record_attestation_epoch_rejected, record_attestation_refresh_failed,
            record_attestation_unknown_key_id, record_attestation_verify_failed,
            record_signer_mint_without_proof,
        },
        storage::auth::DelegationStateOps,
    },
    protocol,
    workflow::rpc::request::handler::RootResponseWorkflow,
};
use sha2::{Digest, Sha256};
use std::{
    future::Future,
    sync::atomic::{AtomicU64, Ordering},
};

///
/// DelegationApi
///
/// Requires auth.delegated_tokens.enabled = true in config.
///

pub struct DelegationApi;

const DEFAULT_ROOT_REQUEST_TTL_SECONDS: u64 = 300;
static ROOT_REQUEST_NONCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug)]
enum RoleAttestationVerifyFlowError {
    Initial(DelegatedTokenOpsError),
    Refresh {
        trigger: DelegatedTokenOpsError,
        source: crate::InternalError,
    },
    PostRefresh(DelegatedTokenOpsError),
}

impl DelegationApi {
    const DELEGATED_TOKENS_DISABLED: &str =
        "delegated token auth disabled; set auth.delegated_tokens.enabled=true in canic.toml";

    fn map_delegation_error(err: crate::InternalError) -> Error {
        match err.class() {
            InternalErrorClass::Infra | InternalErrorClass::Ops | InternalErrorClass::Workflow => {
                Error::internal(err.to_string())
            }
            _ => Error::from(err),
        }
    }

    /// Full delegation proof verification (structure + signature).
    ///
    /// Purely local verification; does not read certified data or require a
    /// query context.
    pub fn verify_delegation_proof(
        proof: &DelegationProof,
        authority_pid: Principal,
    ) -> Result<(), Error> {
        DelegatedTokenOps::verify_delegation_proof(proof, authority_pid)
            .map_err(Self::map_delegation_error)
    }

    pub async fn sign_token(
        claims: DelegatedTokenClaims,
        proof: DelegationProof,
    ) -> Result<DelegatedToken, Error> {
        DelegatedTokenOps::sign_token(claims, proof)
            .await
            .map_err(Self::map_delegation_error)
    }

    /// Full delegated token verification (structure + signature).
    ///
    /// Purely local verification; does not read certified data or require a
    /// query context.
    pub fn verify_token(
        token: &DelegatedToken,
        authority_pid: Principal,
        now_secs: u64,
    ) -> Result<(), Error> {
        DelegatedTokenOps::verify_token(token, authority_pid, now_secs, IcOps::canister_self())
            .map(|_| ())
            .map_err(Self::map_delegation_error)
    }

    /// Verify a delegated token and return verified contents.
    ///
    /// This is intended for application-layer session construction.
    /// It performs full verification and returns verified claims and cert.
    pub fn verify_token_verified(
        token: &DelegatedToken,
        authority_pid: Principal,
        now_secs: u64,
    ) -> Result<(DelegatedTokenClaims, DelegationCert), Error> {
        DelegatedTokenOps::verify_token(token, authority_pid, now_secs, IcOps::canister_self())
            .map(|verified| (verified.claims, verified.cert))
            .map_err(Self::map_delegation_error)
    }

    /// Canonical shard-initiated delegation request (user_shard -> root).
    ///
    /// Caller must match shard_pid and be registered to the subnet.
    pub async fn request_delegation(
        request: DelegationRequest,
    ) -> Result<DelegationProvisionResponse, Error> {
        let request = with_root_request_metadata(request);
        let response =
            RootResponseWorkflow::response(RootCapabilityRequest::IssueDelegation(request))
                .await
                .map_err(Self::map_delegation_error)?;

        match response {
            RootCapabilityResponse::DelegationIssued(response) => Ok(response),
            _ => Err(Error::internal(
                "invalid root response type for delegation request",
            )),
        }
    }

    pub async fn request_role_attestation(
        request: RoleAttestationRequest,
    ) -> Result<SignedRoleAttestation, Error> {
        let request = with_root_attestation_request_metadata(request);
        let response =
            RootResponseWorkflow::response(RootCapabilityRequest::IssueRoleAttestation(request))
                .await
                .map_err(Self::map_delegation_error)?;

        match response {
            RootCapabilityResponse::RoleAttestationIssued(response) => Ok(response),
            _ => Err(Error::internal(
                "invalid root response type for role attestation request",
            )),
        }
    }

    pub async fn attestation_key_set() -> Result<AttestationKeySet, Error> {
        DelegatedTokenOps::attestation_key_set()
            .await
            .map_err(Self::map_delegation_error)
    }

    pub fn replace_attestation_key_set(key_set: AttestationKeySet) {
        DelegatedTokenOps::replace_attestation_key_set(key_set);
    }

    pub async fn verify_role_attestation(
        attestation: &SignedRoleAttestation,
        min_accepted_epoch: u64,
    ) -> Result<(), Error> {
        let configured_min_accepted_epoch = ConfigOps::role_attestation_config()
            .map_err(Error::from)?
            .min_accepted_epoch_by_role
            .get(attestation.payload.role.as_str())
            .copied();
        let min_accepted_epoch =
            resolve_min_accepted_epoch(min_accepted_epoch, configured_min_accepted_epoch);

        let caller = IcOps::msg_caller();
        let self_pid = IcOps::canister_self();
        let now_secs = IcOps::now_secs();
        let verifier_subnet = Some(EnvOps::subnet_pid().map_err(Error::from)?);
        let root_pid = EnvOps::root_pid().map_err(Error::from)?;

        let verify = || {
            DelegatedTokenOps::verify_role_attestation_cached(
                attestation,
                caller,
                self_pid,
                verifier_subnet,
                now_secs,
                min_accepted_epoch,
            )
            .map(|_| ())
        };
        let refresh = || async {
            let key_set: AttestationKeySet =
                RpcOps::call_rpc_result(root_pid, protocol::CANIC_ATTESTATION_KEY_SET, ()).await?;
            DelegatedTokenOps::replace_attestation_key_set(key_set);
            Ok(())
        };

        match verify_role_attestation_with_single_refresh(verify, refresh).await {
            Ok(()) => Ok(()),
            Err(RoleAttestationVerifyFlowError::Initial(err)) => {
                record_attestation_verifier_rejection(&err);
                log_attestation_verifier_rejection(&err, attestation, caller, self_pid, "cached");
                Err(Self::map_delegation_error(err.into()))
            }
            Err(RoleAttestationVerifyFlowError::Refresh { trigger, source }) => {
                record_attestation_verifier_rejection(&trigger);
                log_attestation_verifier_rejection(
                    &trigger,
                    attestation,
                    caller,
                    self_pid,
                    "cache_miss_refresh",
                );
                record_attestation_refresh_failed();
                log!(
                    Topic::Auth,
                    Warn,
                    "role attestation refresh failed local={} caller={} key_id={} error={}",
                    self_pid,
                    caller,
                    attestation.key_id,
                    source
                );
                Err(Self::map_delegation_error(source))
            }
            Err(RoleAttestationVerifyFlowError::PostRefresh(err)) => {
                record_attestation_verifier_rejection(&err);
                log_attestation_verifier_rejection(
                    &err,
                    attestation,
                    caller,
                    self_pid,
                    "post_refresh",
                );
                Err(Self::map_delegation_error(err.into()))
            }
        }
    }

    pub async fn store_proof(
        proof: DelegationProof,
        kind: DelegationProvisionTargetKind,
    ) -> Result<(), Error> {
        let cfg = ConfigOps::delegated_tokens_config().map_err(Error::from)?;
        if !cfg.enabled {
            return Err(Error::forbidden(Self::DELEGATED_TOKENS_DISABLED));
        }

        let root_pid = EnvOps::root_pid().map_err(Error::from)?;
        let caller = IcOps::msg_caller();
        if caller != root_pid {
            return Err(Error::forbidden(
                "delegation proof store requires root caller",
            ));
        }

        DelegatedTokenOps::cache_public_keys_for_cert(&proof.cert)
            .await
            .map_err(Self::map_delegation_error)?;
        if let Err(err) = DelegatedTokenOps::verify_delegation_proof(&proof, root_pid) {
            let local = IcOps::canister_self();
            log!(
                Topic::Auth,
                Warn,
                "delegation proof rejected kind={:?} local={} shard={} issued_at={} expires_at={} error={}",
                kind,
                local,
                proof.cert.shard_pid,
                proof.cert.issued_at,
                proof.cert.expires_at,
                err
            );
            return Err(Self::map_delegation_error(err));
        }

        DelegationStateOps::set_proof_from_dto(proof);
        let local = IcOps::canister_self();
        let stored = DelegationStateOps::proof_dto()
            .ok_or_else(|| Error::invariant("delegation proof missing after store"))?;
        log!(
            Topic::Auth,
            Info,
            "delegation proof stored kind={:?} local={} shard={} issued_at={} expires_at={}",
            kind,
            local,
            stored.cert.shard_pid,
            stored.cert.issued_at,
            stored.cert.expires_at
        );

        Ok(())
    }

    pub fn require_proof() -> Result<DelegationProof, Error> {
        let cfg = ConfigOps::delegated_tokens_config().map_err(Error::from)?;
        if !cfg.enabled {
            return Err(Error::forbidden(Self::DELEGATED_TOKENS_DISABLED));
        }

        DelegationStateOps::proof_dto().ok_or_else(|| {
            record_signer_mint_without_proof();
            Error::not_found("delegation proof not set")
        })
    }
}

fn with_root_request_metadata(mut request: DelegationRequest) -> DelegationRequest {
    if request.metadata.is_none() {
        request.metadata = Some(new_request_metadata());
    }
    request
}

fn with_root_attestation_request_metadata(
    mut request: RoleAttestationRequest,
) -> RoleAttestationRequest {
    if request.metadata.is_none() {
        request.metadata = Some(new_request_metadata());
    }
    request
}

fn new_request_metadata() -> RootRequestMetadata {
    RootRequestMetadata {
        request_id: generate_request_id(),
        ttl_seconds: DEFAULT_ROOT_REQUEST_TTL_SECONDS,
    }
}

fn generate_request_id() -> [u8; 32] {
    if let Ok(bytes) = crate::utils::rand::random_bytes(32)
        && bytes.len() == 32
    {
        let mut out = [0u8; 32];
        out.copy_from_slice(&bytes);
        return out;
    }

    let nonce = ROOT_REQUEST_NONCE.fetch_add(1, Ordering::Relaxed);
    let now = IcOps::now_secs();
    let caller = IcOps::msg_caller();
    let canister = IcOps::canister_self();

    let mut hasher = Sha256::new();
    hasher.update(now.to_be_bytes());
    hasher.update(nonce.to_be_bytes());
    hasher.update(caller.as_slice());
    hasher.update(canister.as_slice());
    hasher.finalize().into()
}

async fn verify_role_attestation_with_single_refresh<Verify, Refresh, RefreshFuture>(
    mut verify: Verify,
    mut refresh: Refresh,
) -> Result<(), RoleAttestationVerifyFlowError>
where
    Verify: FnMut() -> Result<(), DelegatedTokenOpsError>,
    Refresh: FnMut() -> RefreshFuture,
    RefreshFuture: Future<Output = Result<(), crate::InternalError>>,
{
    match verify() {
        Ok(()) => Ok(()),
        Err(err @ DelegatedTokenOpsError::AttestationUnknownKeyId { .. }) => {
            refresh()
                .await
                .map_err(|source| RoleAttestationVerifyFlowError::Refresh {
                    trigger: err,
                    source,
                })?;
            verify().map_err(RoleAttestationVerifyFlowError::PostRefresh)
        }
        Err(err) => Err(RoleAttestationVerifyFlowError::Initial(err)),
    }
}

fn resolve_min_accepted_epoch(explicit: u64, configured: Option<u64>) -> u64 {
    if explicit > 0 {
        explicit
    } else {
        configured.unwrap_or(0)
    }
}

fn record_attestation_verifier_rejection(err: &DelegatedTokenOpsError) {
    record_attestation_verify_failed();
    match err {
        DelegatedTokenOpsError::AttestationUnknownKeyId { .. } => {
            record_attestation_unknown_key_id();
        }
        DelegatedTokenOpsError::AttestationEpochRejected { .. } => {
            record_attestation_epoch_rejected();
        }
        _ => {}
    }
}

fn log_attestation_verifier_rejection(
    err: &DelegatedTokenOpsError,
    attestation: &SignedRoleAttestation,
    caller: Principal,
    self_pid: Principal,
    phase: &str,
) {
    log!(
        Topic::Auth,
        Warn,
        "role attestation rejected phase={} local={} caller={} subject={} role={} key_id={} audience={:?} subnet={:?} issued_at={} expires_at={} epoch={} error={}",
        phase,
        self_pid,
        caller,
        attestation.payload.subject,
        attestation.payload.role,
        attestation.key_id,
        attestation.payload.audience,
        attestation.payload.subnet_id,
        attestation.payload.issued_at,
        attestation.payload.expires_at,
        attestation.payload.epoch,
        err
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InternalErrorOrigin;
    use futures::executor::block_on;
    use std::cell::Cell;

    #[test]
    fn verify_role_attestation_with_single_refresh_accepts_without_refresh() {
        let verify_calls = Cell::new(0usize);
        let refresh_calls = Cell::new(0usize);

        let result = block_on(verify_role_attestation_with_single_refresh(
            || {
                verify_calls.set(verify_calls.get() + 1);
                Ok(())
            },
            || {
                refresh_calls.set(refresh_calls.get() + 1);
                std::future::ready(Ok(()))
            },
        ));

        assert!(result.is_ok());
        assert_eq!(verify_calls.get(), 1, "verify must run exactly once");
        assert_eq!(refresh_calls.get(), 0, "refresh must not run");
    }

    #[test]
    fn verify_role_attestation_with_single_refresh_retries_once_on_unknown_key() {
        let verify_calls = Cell::new(0usize);
        let refresh_calls = Cell::new(0usize);

        let result = block_on(verify_role_attestation_with_single_refresh(
            || {
                let attempt = verify_calls.get();
                verify_calls.set(attempt + 1);
                if attempt == 0 {
                    Err(DelegatedTokenOpsError::AttestationUnknownKeyId { key_id: 7 })
                } else {
                    Ok(())
                }
            },
            || {
                refresh_calls.set(refresh_calls.get() + 1);
                std::future::ready(Ok(()))
            },
        ));

        assert!(result.is_ok());
        assert_eq!(verify_calls.get(), 2, "verify must run exactly twice");
        assert_eq!(refresh_calls.get(), 1, "refresh must run exactly once");
    }

    #[test]
    fn verify_role_attestation_with_single_refresh_fails_closed_on_refresh_error() {
        let verify_calls = Cell::new(0usize);
        let refresh_calls = Cell::new(0usize);

        let result = block_on(verify_role_attestation_with_single_refresh(
            || {
                verify_calls.set(verify_calls.get() + 1);
                Err(DelegatedTokenOpsError::AttestationUnknownKeyId { key_id: 9 })
            },
            || {
                refresh_calls.set(refresh_calls.get() + 1);
                std::future::ready(Err(crate::InternalError::infra(
                    InternalErrorOrigin::Infra,
                    "refresh failed",
                )))
            },
        ));

        match result {
            Err(RoleAttestationVerifyFlowError::Refresh {
                trigger: DelegatedTokenOpsError::AttestationUnknownKeyId { key_id },
                ..
            }) => assert_eq!(key_id, 9),
            other => panic!("expected refresh failure for unknown key, got: {other:?}"),
        }

        assert_eq!(
            verify_calls.get(),
            1,
            "verify must not retry after refresh failure"
        );
        assert_eq!(refresh_calls.get(), 1, "refresh must run once");
    }

    #[test]
    fn verify_role_attestation_with_single_refresh_does_not_refresh_on_non_unknown_error() {
        let verify_calls = Cell::new(0usize);
        let refresh_calls = Cell::new(0usize);

        let result = block_on(verify_role_attestation_with_single_refresh(
            || {
                verify_calls.set(verify_calls.get() + 1);
                Err(DelegatedTokenOpsError::AttestationEpochRejected {
                    epoch: 1,
                    min_accepted_epoch: 2,
                })
            },
            || {
                refresh_calls.set(refresh_calls.get() + 1);
                std::future::ready(Ok(()))
            },
        ));

        match result {
            Err(RoleAttestationVerifyFlowError::Initial(
                DelegatedTokenOpsError::AttestationEpochRejected {
                    epoch,
                    min_accepted_epoch,
                },
            )) => {
                assert_eq!(epoch, 1);
                assert_eq!(min_accepted_epoch, 2);
            }
            other => panic!("expected initial epoch rejection, got: {other:?}"),
        }

        assert_eq!(verify_calls.get(), 1, "verify must run once");
        assert_eq!(refresh_calls.get(), 0, "refresh must not run");
    }

    #[test]
    fn verify_role_attestation_with_single_refresh_only_attempts_one_refresh() {
        let verify_calls = Cell::new(0usize);
        let refresh_calls = Cell::new(0usize);

        let result = block_on(verify_role_attestation_with_single_refresh(
            || {
                let attempt = verify_calls.get();
                verify_calls.set(attempt + 1);
                if attempt == 0 {
                    Err(DelegatedTokenOpsError::AttestationUnknownKeyId { key_id: 5 })
                } else {
                    Err(DelegatedTokenOpsError::AttestationUnknownKeyId { key_id: 6 })
                }
            },
            || {
                refresh_calls.set(refresh_calls.get() + 1);
                std::future::ready(Ok(()))
            },
        ));

        match result {
            Err(RoleAttestationVerifyFlowError::PostRefresh(
                DelegatedTokenOpsError::AttestationUnknownKeyId { key_id },
            )) => assert_eq!(key_id, 6),
            other => panic!("expected post-refresh unknown-key rejection, got: {other:?}"),
        }

        assert_eq!(verify_calls.get(), 2, "verify must run exactly twice");
        assert_eq!(refresh_calls.get(), 1, "refresh must run exactly once");
    }

    #[test]
    fn resolve_min_accepted_epoch_prefers_explicit_argument() {
        assert_eq!(resolve_min_accepted_epoch(7, Some(3)), 7);
        assert_eq!(resolve_min_accepted_epoch(5, None), 5);
    }

    #[test]
    fn resolve_min_accepted_epoch_falls_back_to_config_or_zero() {
        assert_eq!(resolve_min_accepted_epoch(0, Some(4)), 4);
        assert_eq!(resolve_min_accepted_epoch(0, None), 0);
    }
}
