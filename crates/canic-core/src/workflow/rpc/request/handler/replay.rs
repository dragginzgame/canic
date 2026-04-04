use super::{
    MAX_ROOT_REPLAY_ENTRIES, MAX_ROOT_TTL_SECONDS, REPLAY_PAYLOAD_HASH_DOMAIN,
    REPLAY_PURGE_SCAN_LIMIT, RootCapability, RootContext,
};
use crate::{
    InternalError,
    cdk::types::Principal,
    dto::rpc::Response,
    ids::CanisterRole,
    ops::{
        replay::{
            self as replay_ops, ReplayCommitError, ReplayReserveError,
            guard::{ReplayDecision, ReplayGuardError, ReplayPending, RootReplayGuardInput},
        },
        runtime::metrics::root_capability::{
            RootCapabilityMetricKey, RootCapabilityMetricOutcome, RootCapabilityMetrics,
        },
    },
    workflow::rpc::RpcWorkflowError,
};
#[cfg(test)]
use crate::{ops::replay::key as replay_key, storage::stable::replay::ReplaySlotKey};
use candid::decode_one;
use sha2::{Digest, Sha256};

/// ReplayPreflight
///
/// Workflow replay gate result used to branch execute-vs-cache behavior.
#[derive(Debug)]
pub(super) enum ReplayPreflight {
    Fresh(ReplayPending),
    Cached(Response),
}

/// check_replay
///
/// Run replay guard and convert pure replay outcomes into workflow results.
pub(super) fn check_replay(
    ctx: &RootContext,
    capability: &RootCapability,
) -> Result<ReplayPreflight, InternalError> {
    let capability_key = capability.metric_key();
    let capability_name = capability.capability_name();
    let metadata = capability
        .metadata()
        .ok_or(RpcWorkflowError::MissingReplayMetadata(capability_name))?;
    let payload_hash = capability.payload_hash();
    crate::perf!("prepare_replay_input");

    let decision =
        replay::evaluate_root_replay(ctx, metadata.request_id, metadata.ttl_seconds, payload_hash)
            .map_err(|err| map_replay_guard_error(capability_key, err))?;
    crate::perf!("evaluate_replay");

    match decision {
        ReplayDecision::Fresh(pending) => {
            replay_ops::reserve_root_replay(pending, MAX_ROOT_REPLAY_ENTRIES)
                .map_err(map_replay_reserve_error)?;
            crate::perf!("reserve_fresh");
            RootCapabilityMetrics::record_replay(
                capability_key,
                RootCapabilityMetricOutcome::Accepted,
            );
            Ok(ReplayPreflight::Fresh(pending))
        }
        ReplayDecision::DuplicateSame(cached) => {
            crate::perf!("decode_cached");
            RootCapabilityMetrics::record_replay(
                capability_key,
                RootCapabilityMetricOutcome::DuplicateSame,
            );
            decode_replay_response(&cached.response_candid).map(ReplayPreflight::Cached)
        }
        ReplayDecision::InFlight => {
            crate::perf!("duplicate_in_flight");
            RootCapabilityMetrics::record_replay(
                capability_key,
                RootCapabilityMetricOutcome::DuplicateSame,
            );
            Err(RpcWorkflowError::ReplayDuplicateSame(capability_name).into())
        }
        ReplayDecision::DuplicateConflict => {
            crate::perf!("duplicate_conflict");
            RootCapabilityMetrics::record_replay(
                capability_key,
                RootCapabilityMetricOutcome::DuplicateConflict,
            );
            Err(RpcWorkflowError::ReplayConflict(capability_name).into())
        }
        ReplayDecision::Expired => {
            crate::perf!("replay_expired");
            RootCapabilityMetrics::record_replay(
                capability_key,
                RootCapabilityMetricOutcome::Expired,
            );
            Err(RpcWorkflowError::ReplayExpired(capability_name).into())
        }
    }
}

/// map_replay_guard_error
///
/// Convert guard-level infra errors into workflow replay failures.
fn map_replay_guard_error(
    capability_key: RootCapabilityMetricKey,
    err: ReplayGuardError,
) -> InternalError {
    match err {
        ReplayGuardError::InvalidTtl {
            ttl_seconds,
            max_ttl_seconds,
        } => {
            RootCapabilityMetrics::record_replay(
                capability_key,
                RootCapabilityMetricOutcome::TtlExceeded,
            );
            RpcWorkflowError::InvalidReplayTtl {
                ttl_seconds,
                max_ttl_seconds,
            }
            .into()
        }
    }
}

/// map_replay_reserve_error
///
/// Convert ops replay-reservation failures into workflow replay errors.
fn map_replay_reserve_error(err: ReplayReserveError) -> InternalError {
    match err {
        ReplayReserveError::CapacityReached { max_entries } => {
            RpcWorkflowError::ReplayStoreCapacityReached(max_entries).into()
        }
    }
}

/// map_replay_commit_error
///
/// Convert ops replay-commit failures into workflow replay errors.
fn map_replay_commit_error(err: ReplayCommitError) -> InternalError {
    match err {
        ReplayCommitError::EncodeFailed(message) => {
            RpcWorkflowError::ReplayEncodeFailed(message).into()
        }
    }
}

/// decode_replay_response
///
/// Decode cached replay payload bytes back into canonical root responses.
fn decode_replay_response(bytes: &[u8]) -> Result<Response, InternalError> {
    decode_one(bytes).map_err(|err| RpcWorkflowError::ReplayDecodeFailed(err.to_string()).into())
}

/// commit_replay
///
/// Persist a replay record after successful capability execution.
pub(super) fn commit_replay(
    pending: ReplayPending,
    response: &Response,
) -> Result<(), InternalError> {
    crate::perf!("commit_encode");
    replay_ops::commit_root_replay(pending, response).map_err(map_replay_commit_error)
}

/// abort_replay
///
/// Remove reserved replay state when capability execution fails.
pub(super) fn abort_replay(pending: ReplayPending) {
    replay_ops::abort_root_replay(pending);
    crate::perf!("abort_replay");
}

/// payload_hasher
///
/// Start a replay payload hasher with the shared domain prefix applied.
pub(super) fn payload_hasher() -> Sha256 {
    let mut hasher = Sha256::new();
    hasher.update((REPLAY_PAYLOAD_HASH_DOMAIN.len() as u64).to_be_bytes());
    hasher.update(REPLAY_PAYLOAD_HASH_DOMAIN);
    hasher
}

/// hash_u64
///
/// Append one fixed-width `u64` field to the replay payload hash.
pub(super) fn hash_u64(hasher: &mut Sha256, value: u64) {
    hasher.update(value.to_be_bytes());
}

/// hash_u128
///
/// Append one fixed-width `u128` field to the replay payload hash.
pub(super) fn hash_u128(hasher: &mut Sha256, value: u128) {
    hasher.update(value.to_be_bytes());
}

/// hash_bool
///
/// Append one boolean field to the replay payload hash.
pub(super) fn hash_bool(hasher: &mut Sha256, value: bool) {
    hasher.update([u8::from(value)]);
}

/// hash_bytes
///
/// Append one length-prefixed byte slice to the replay payload hash.
pub(super) fn hash_bytes(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}

/// hash_str
///
/// Append one UTF-8 string field to the replay payload hash.
pub(super) fn hash_str(hasher: &mut Sha256, value: &str) {
    hash_bytes(hasher, value.as_bytes());
}

/// hash_role
///
/// Append one canister role field to the replay payload hash.
pub(super) fn hash_role(hasher: &mut Sha256, role: &CanisterRole) {
    hash_str(hasher, role.as_str());
}

/// hash_principal
///
/// Append one principal field to the replay payload hash.
pub(super) fn hash_principal(hasher: &mut Sha256, principal: &Principal) {
    hash_bytes(hasher, principal.as_slice());
}

/// hash_principals
///
/// Append one principal-vector field to the replay payload hash.
pub(super) fn hash_principals(hasher: &mut Sha256, principals: &[Principal]) {
    hash_u64(hasher, principals.len() as u64);
    for principal in principals {
        hash_principal(hasher, principal);
    }
}

/// hash_strings
///
/// Append one string-vector field to the replay payload hash.
pub(super) fn hash_strings(hasher: &mut Sha256, values: &[String]) {
    hash_u64(hasher, values.len() as u64);
    for value in values {
        hash_str(hasher, value);
    }
}

/// hash_optional_principal
///
/// Append one optional principal field to the replay payload hash.
pub(super) fn hash_optional_principal(hasher: &mut Sha256, principal: Option<Principal>) {
    hash_bool(hasher, principal.is_some());
    if let Some(principal) = principal {
        hash_principal(hasher, &principal);
    }
}

/// hash_optional_bytes
///
/// Append one optional byte-vector field to the replay payload hash.
pub(super) fn hash_optional_bytes(hasher: &mut Sha256, bytes: Option<&[u8]>) {
    hash_bool(hasher, bytes.is_some());
    if let Some(bytes) = bytes {
        hash_bytes(hasher, bytes);
    }
}

/// finish_payload_hash
///
/// Finalize a prepared replay payload hash.
pub(super) fn finish_payload_hash(hasher: Sha256) -> [u8; 32] {
    hasher.finalize().into()
}

/// replay_slot_key
///
/// Build current root replay slot key (test helper passthrough).
#[cfg(test)]
pub(super) fn replay_slot_key(
    caller: Principal,
    target_canister: Principal,
    request_id: [u8; 32],
) -> ReplaySlotKey {
    replay_key::root_slot_key(caller, target_canister, request_id)
}

/// hash_domain_separated
///
/// Build deterministic domain-separated hash values for replay payloads.
#[cfg(test)]
pub(super) fn hash_domain_separated(domain: &[u8], payload: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update((domain.len() as u64).to_be_bytes());
    hasher.update(domain);
    hasher.update((payload.len() as u64).to_be_bytes());
    hasher.update(payload);
    hasher.finalize().into()
}

mod replay {
    use super::*;

    /// evaluate_root_replay
    ///
    /// Call the ops replay guard with workflow root replay context.
    pub(super) fn evaluate_root_replay(
        ctx: &RootContext,
        request_id: [u8; 32],
        ttl_seconds: u64,
        payload_hash: [u8; 32],
    ) -> Result<ReplayDecision, ReplayGuardError> {
        crate::ops::replay::guard::evaluate_root_replay(RootReplayGuardInput {
            caller: ctx.caller,
            target_canister: ctx.self_pid,
            request_id,
            ttl_seconds,
            payload_hash,
            now: ctx.now,
            max_ttl_seconds: MAX_ROOT_TTL_SECONDS,
            purge_scan_limit: REPLAY_PURGE_SCAN_LIMIT,
        })
    }
}
