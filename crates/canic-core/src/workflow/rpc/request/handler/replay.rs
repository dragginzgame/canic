use super::{
    MAX_ROOT_REPLAY_ENTRIES, MAX_ROOT_TTL_SECONDS, REPLAY_PAYLOAD_HASH_DOMAIN,
    REPLAY_PURGE_SCAN_LIMIT, RootCapability, RootContext,
};
use crate::{
    InternalError,
    dto::rpc::{Response, RootCapabilityRequest},
    ops::{
        replay::{
            guard::{ReplayDecision, ReplayGuardError, ReplayPending, RootReplayGuardInput},
            slot as replay_slot,
        },
        runtime::metrics::root_capability::{
            RootCapabilityMetricEvent, RootCapabilityMetricKey, RootCapabilityMetrics,
        },
    },
    storage::stable::replay::RootReplayRecord,
    workflow::rpc::RpcWorkflowError,
};
#[cfg(test)]
use crate::{
    cdk::types::Principal, ops::replay::key as replay_key, storage::stable::replay::ReplaySlotKey,
};
use candid::encode_one;
use sha2::{Digest, Sha256};

/// check_replay
///
/// Run replay guard and convert pure replay outcomes into workflow results.
pub(super) fn check_replay(
    ctx: &RootContext,
    capability: &RootCapability,
) -> Result<ReplayPending, InternalError> {
    let capability_key = capability.metric_key();
    let metadata = capability
        .metadata()
        .ok_or_else(|| RpcWorkflowError::MissingReplayMetadata(capability.capability_name()))?;
    let payload_hash = capability.payload_hash()?;

    let decision =
        replay::evaluate_root_replay(ctx, metadata.request_id, metadata.ttl_seconds, payload_hash)
            .map_err(|err| map_replay_guard_error(capability_key, err))?;

    match decision {
        ReplayDecision::Fresh(pending) => {
            RootCapabilityMetrics::record(
                capability_key,
                RootCapabilityMetricEvent::ReplayAccepted,
            );
            Ok(pending)
        }
        ReplayDecision::DuplicateSame => {
            RootCapabilityMetrics::record(
                capability_key,
                RootCapabilityMetricEvent::ReplayDuplicateSame,
            );
            Err(RpcWorkflowError::ReplayDuplicateSame(capability.capability_name()).into())
        }
        ReplayDecision::DuplicateConflict => {
            RootCapabilityMetrics::record(
                capability_key,
                RootCapabilityMetricEvent::ReplayDuplicateConflict,
            );
            Err(RpcWorkflowError::ReplayConflict(capability.capability_name()).into())
        }
        ReplayDecision::Expired => {
            RootCapabilityMetrics::record(capability_key, RootCapabilityMetricEvent::ReplayExpired);
            Err(RpcWorkflowError::ReplayExpired(capability.capability_name()).into())
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
            RootCapabilityMetrics::record(
                capability_key,
                RootCapabilityMetricEvent::ReplayTtlExceeded,
            );
            RpcWorkflowError::InvalidReplayTtl {
                ttl_seconds,
                max_ttl_seconds,
            }
            .into()
        }
    }
}

/// commit_replay
///
/// Persist a replay record after successful capability execution.
pub(super) fn commit_replay(
    pending: ReplayPending,
    response: &Response,
) -> Result<(), InternalError> {
    if replay_slot::root_slot_len() >= MAX_ROOT_REPLAY_ENTRIES {
        return Err(RpcWorkflowError::ReplayStoreCapacityReached(MAX_ROOT_REPLAY_ENTRIES).into());
    }

    let response_candid = encode_one(response)
        .map_err(|err| RpcWorkflowError::ReplayEncodeFailed(err.to_string()))?;

    replay_slot::upsert_root_slot(
        pending.slot_key,
        RootReplayRecord {
            payload_hash: pending.payload_hash,
            issued_at: pending.issued_at,
            expires_at: pending.expires_at,
            response_candid,
        },
    );

    Ok(())
}

/// hash_capability_payload
///
/// Compute replay payload hash for canonical capability request bytes.
pub(super) fn hash_capability_payload(
    payload: &RootCapabilityRequest,
) -> Result<[u8; 32], InternalError> {
    let bytes = encode_one(payload).map_err(|err| {
        RpcWorkflowError::ReplayEncodeFailed(format!("canonical payload encode failed: {err}"))
    })?;
    Ok(hash_domain_separated(REPLAY_PAYLOAD_HASH_DOMAIN, &bytes))
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

/// replay_slot_key_legacy
///
/// Build legacy root replay slot key (test helper passthrough).
#[cfg(test)]
pub(super) fn replay_slot_key_legacy(
    caller: Principal,
    subnet_id: Principal,
    request_id: [u8; 32],
) -> ReplaySlotKey {
    replay_key::legacy_root_slot_key(caller, subnet_id, request_id)
}

/// hash_domain_separated
///
/// Build deterministic domain-separated hash values for replay payloads.
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
            subnet_id: ctx.subnet_id,
            request_id,
            ttl_seconds,
            payload_hash,
            now: ctx.now,
            max_ttl_seconds: MAX_ROOT_TTL_SECONDS,
            purge_scan_limit: REPLAY_PURGE_SCAN_LIMIT,
        })
    }
}
