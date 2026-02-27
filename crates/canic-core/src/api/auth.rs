use crate::{
    cdk::types::Principal,
    dto::{
        auth::{
            DelegatedToken, DelegatedTokenClaims, DelegationCert, DelegationProof,
            DelegationProvisionResponse, DelegationProvisionTargetKind, DelegationRequest,
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
        auth::DelegatedTokenOps, config::ConfigOps, ic::IcOps, runtime::env::EnvOps,
        runtime::metrics::auth::record_signer_mint_without_proof,
        storage::auth::DelegationStateOps,
    },
    workflow::rpc::request::handler::RootResponseWorkflow,
};
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicU64, Ordering};

///
/// DelegationApi
///
/// Requires auth.delegated_tokens.enabled = true in config.
///

pub struct DelegationApi;

const DEFAULT_ROOT_REQUEST_TTL_SECONDS: u64 = 300;
static ROOT_REQUEST_NONCE: AtomicU64 = AtomicU64::new(1);

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
