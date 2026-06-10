use crate::{
    cdk::types::Principal,
    config::schema::RoleAttestationConfig,
    dto::{
        auth::{InternalInvocationProofRequest, SignedInternalInvocationProofV1},
        error::Error,
    },
    ids::CanisterRole,
    ops::{config::ConfigOps, ic::IcOps, runtime::env::EnvOps},
};
use std::{cell::RefCell, collections::BTreeMap};

const ONE_SECOND_NS: u64 = 1_000_000_000;
const INTERNAL_CALL_PROOF_REFRESH_MARGIN_MAX_NS: u64 = 30_000_000_000;

thread_local! {
    static INTERNAL_INVOCATION_PROOF_CACHE:
        RefCell<BTreeMap<InternalInvocationProofCacheKey, SignedInternalInvocationProofV1>> =
        const { RefCell::new(BTreeMap::new()) };
}

///
/// InternalInvocationProofCacheKey
///

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct InternalInvocationProofCacheKey {
    root_pid: Principal,
    attestation_key_name: String,
    subject: Principal,
    role: CanisterRole,
    subnet_id: Option<Principal>,
    audience: Principal,
    audience_method: String,
    ttl_ns: u64,
}

pub(super) async fn internal_invocation_proof_for_request(
    request: InternalInvocationProofRequest,
) -> Result<SignedInternalInvocationProofV1, Error> {
    let cfg = ConfigOps::role_attestation_config().map_err(Error::from)?;
    let root_pid = EnvOps::root_pid().map_err(Error::from)?;
    let now_ns = IcOps::now_nanos();

    if let Some(proof) = cached_internal_invocation_proof(&request, &cfg, root_pid, now_ns) {
        return Ok(proof);
    }

    fresh_internal_invocation_proof_for_request_with_context(request, cfg, root_pid, now_ns).await
}

pub(super) async fn fresh_internal_invocation_proof_for_request(
    request: InternalInvocationProofRequest,
) -> Result<SignedInternalInvocationProofV1, Error> {
    let cfg = ConfigOps::role_attestation_config().map_err(Error::from)?;
    let root_pid = EnvOps::root_pid().map_err(Error::from)?;
    let now_ns = IcOps::now_nanos();
    fresh_internal_invocation_proof_for_request_with_context(request, cfg, root_pid, now_ns).await
}

async fn fresh_internal_invocation_proof_for_request_with_context(
    request: InternalInvocationProofRequest,
    cfg: RoleAttestationConfig,
    root_pid: Principal,
    now_ns: u64,
) -> Result<SignedInternalInvocationProofV1, Error> {
    let proof =
        crate::api::auth::AuthApi::request_internal_invocation_proof(request.clone()).await?;
    cache_internal_invocation_proof(&request, &cfg, root_pid, now_ns, proof.clone());
    Ok(proof)
}

fn internal_invocation_proof_cache_key(
    request: &InternalInvocationProofRequest,
    cfg: &RoleAttestationConfig,
    root_pid: Principal,
) -> InternalInvocationProofCacheKey {
    InternalInvocationProofCacheKey {
        root_pid,
        attestation_key_name: cfg.ecdsa_key_name.clone(),
        subject: request.subject,
        role: request.role.clone(),
        subnet_id: request.subnet_id,
        audience: request.audience,
        audience_method: request.audience_method.clone(),
        ttl_ns: request.ttl_ns,
    }
}

pub(super) fn cached_internal_invocation_proof(
    request: &InternalInvocationProofRequest,
    cfg: &RoleAttestationConfig,
    root_pid: Principal,
    now_ns: u64,
) -> Option<SignedInternalInvocationProofV1> {
    let key = internal_invocation_proof_cache_key(request, cfg, root_pid);
    let min_accepted_epoch = cfg
        .min_accepted_epoch_by_role
        .get(request.role.as_str())
        .copied()
        .unwrap_or(0);

    INTERNAL_INVOCATION_PROOF_CACHE.with_borrow_mut(|cache| {
        let proof = cache.get(&key)?;
        if internal_invocation_proof_is_reusable(proof, request, now_ns, min_accepted_epoch) {
            Some(proof.clone())
        } else {
            cache.remove(&key);
            None
        }
    })
}

pub(super) fn cache_internal_invocation_proof(
    request: &InternalInvocationProofRequest,
    cfg: &RoleAttestationConfig,
    root_pid: Principal,
    now_ns: u64,
    proof: SignedInternalInvocationProofV1,
) {
    let min_accepted_epoch = cfg
        .min_accepted_epoch_by_role
        .get(request.role.as_str())
        .copied()
        .unwrap_or(0);
    if !internal_invocation_proof_is_reusable(&proof, request, now_ns, min_accepted_epoch) {
        return;
    }

    let key = internal_invocation_proof_cache_key(request, cfg, root_pid);
    INTERNAL_INVOCATION_PROOF_CACHE.with_borrow_mut(|cache| {
        cache.insert(key, proof);
    });
}

pub(super) fn invalidate_internal_invocation_proof(
    request: &InternalInvocationProofRequest,
) -> Result<(), Error> {
    let cfg = ConfigOps::role_attestation_config().map_err(Error::from)?;
    let root_pid = EnvOps::root_pid().map_err(Error::from)?;
    let key = internal_invocation_proof_cache_key(request, &cfg, root_pid);
    INTERNAL_INVOCATION_PROOF_CACHE.with_borrow_mut(|cache| {
        cache.remove(&key);
    });
    Ok(())
}

fn internal_invocation_proof_is_reusable(
    proof: &SignedInternalInvocationProofV1,
    request: &InternalInvocationProofRequest,
    now_ns: u64,
    min_accepted_epoch: u64,
) -> bool {
    let payload = &proof.payload;
    if payload.expires_at_ns <= payload.issued_at_ns || now_ns < payload.issued_at_ns {
        return false;
    }

    payload.subject == request.subject
        && payload.role == request.role
        && payload.subnet_id == request.subnet_id
        && payload.audience == request.audience
        && payload.audience_method == request.audience_method
        && payload.epoch >= min_accepted_epoch
        && now_ns.saturating_add(internal_invocation_proof_refresh_margin_ns(proof))
            < payload.expires_at_ns
}

pub(super) fn internal_invocation_proof_refresh_margin_ns(
    proof: &SignedInternalInvocationProofV1,
) -> u64 {
    proof
        .payload
        .expires_at_ns
        .saturating_sub(proof.payload.issued_at_ns)
        .saturating_div(5)
        .clamp(ONE_SECOND_NS, INTERNAL_CALL_PROOF_REFRESH_MARGIN_MAX_NS)
}

#[cfg(test)]
pub(super) fn clear_internal_invocation_proof_cache() {
    INTERNAL_INVOCATION_PROOF_CACHE.with_borrow_mut(BTreeMap::clear);
}
