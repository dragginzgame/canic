use super::{
    LEGACY_REPLAY_NONCE, LEGACY_REPLAY_SLOT_KEY_DOMAIN, MAX_ROOT_REPLAY_ENTRIES,
    MAX_ROOT_TTL_SECONDS, REPLAY_PAYLOAD_HASH_DOMAIN, REPLAY_PURGE_SCAN_LIMIT, ReplayDecision,
    ReplayPending, RootCapability, RootContext,
};
use crate::{
    InternalError,
    cdk::types::Principal,
    dto::rpc::{Response, RootCapabilityRequest},
    ops::{
        runtime::metrics::root_capability::{
            RootCapabilityMetricEvent, RootCapabilityMetricKey, RootCapabilityMetrics,
        },
        storage::replay::{ReplayService, RootReplayOps},
    },
    storage::stable::replay::{ReplaySlotKey, RootReplayRecord},
    workflow::rpc::RpcWorkflowError,
};
use candid::{decode_one, encode_one};
use sha2::{Digest, Sha256};

pub(super) fn check_replay(
    ctx: &RootContext,
    capability: &RootCapability,
) -> Result<ReplayDecision, InternalError> {
    let capability_key = capability.metric_key();

    let metadata = capability
        .metadata()
        .ok_or_else(|| RpcWorkflowError::MissingReplayMetadata(capability.capability_name()))?;

    if metadata.ttl_seconds == 0 || metadata.ttl_seconds > MAX_ROOT_TTL_SECONDS {
        RootCapabilityMetrics::record(capability_key, RootCapabilityMetricEvent::ReplayTtlExceeded);
        return Err(RpcWorkflowError::InvalidReplayTtl {
            ttl_seconds: metadata.ttl_seconds,
            max_ttl_seconds: MAX_ROOT_TTL_SECONDS,
        }
        .into());
    }

    let payload_hash = capability.payload_hash()?;
    let slot_key = replay_slot_key(ctx.caller, ctx.self_pid, metadata.request_id);
    let legacy_slot_key = replay_slot_key_legacy(ctx.caller, ctx.subnet_id, metadata.request_id);

    if let Some(existing) = RootReplayOps::get(slot_key) {
        return resolve_existing_replay(
            capability.capability_name(),
            capability_key,
            ctx.now,
            payload_hash,
            existing,
        );
    }

    // Compatibility path for 0.11-era keys during replay key migration.
    if legacy_slot_key != slot_key
        && let Some(existing) = RootReplayOps::get(legacy_slot_key)
    {
        return resolve_existing_replay(
            capability.capability_name(),
            capability_key,
            ctx.now,
            payload_hash,
            existing,
        );
    }

    let _ = RootReplayOps::purge_expired(ctx.now, REPLAY_PURGE_SCAN_LIMIT);

    let issued_at = ctx.now;
    let expires_at = issued_at.saturating_add(metadata.ttl_seconds);
    RootCapabilityMetrics::record(capability_key, RootCapabilityMetricEvent::ReplayAccepted);

    Ok(ReplayDecision::Pending(ReplayPending {
        slot_key,
        payload_hash,
        issued_at,
        expires_at,
    }))
}

pub(super) fn resolve_existing_replay(
    capability_name: &'static str,
    capability_key: RootCapabilityMetricKey,
    now: u64,
    payload_hash: [u8; 32],
    existing: RootReplayRecord,
) -> Result<ReplayDecision, InternalError> {
    if now > existing.expires_at {
        RootCapabilityMetrics::record(capability_key, RootCapabilityMetricEvent::ReplayExpired);
        return Err(RpcWorkflowError::ReplayExpired(capability_name).into());
    }

    if existing.payload_hash != payload_hash {
        RootCapabilityMetrics::record(
            capability_key,
            RootCapabilityMetricEvent::ReplayDuplicateConflict,
        );
        return Err(RpcWorkflowError::ReplayConflict(capability_name).into());
    }

    let response = decode_one::<Response>(&existing.response_candid)
        .map_err(|err| RpcWorkflowError::ReplayDecodeFailed(err.to_string()))?;
    RootCapabilityMetrics::record(
        capability_key,
        RootCapabilityMetricEvent::ReplayDuplicateSame,
    );

    Ok(ReplayDecision::Cached(response))
}

pub(super) fn commit_replay(
    pending: ReplayPending,
    response: &Response,
) -> Result<(), InternalError> {
    if RootReplayOps::len() >= MAX_ROOT_REPLAY_ENTRIES {
        return Err(RpcWorkflowError::ReplayStoreCapacityReached(MAX_ROOT_REPLAY_ENTRIES).into());
    }

    let response_candid = encode_one(response)
        .map_err(|err| RpcWorkflowError::ReplayEncodeFailed(err.to_string()))?;

    RootReplayOps::upsert(
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

pub(super) fn hash_capability_payload(
    payload: &RootCapabilityRequest,
) -> Result<[u8; 32], InternalError> {
    let bytes = encode_one(payload).map_err(|err| {
        RpcWorkflowError::ReplayEncodeFailed(format!("canonical payload encode failed: {err}"))
    })?;
    Ok(hash_domain_separated(REPLAY_PAYLOAD_HASH_DOMAIN, &bytes))
}

pub(super) fn replay_slot_key(
    caller: Principal,
    target_canister: Principal,
    request_id: [u8; 32],
) -> ReplaySlotKey {
    RootReplayOps::slot_key(
        caller,
        target_canister,
        ReplayService::Root,
        &request_id,
        LEGACY_REPLAY_NONCE,
    )
}

pub(super) fn replay_slot_key_legacy(
    caller: Principal,
    subnet_id: Principal,
    request_id: [u8; 32],
) -> ReplaySlotKey {
    let mut hasher = Sha256::new();
    hasher.update((LEGACY_REPLAY_SLOT_KEY_DOMAIN.len() as u64).to_be_bytes());
    hasher.update(LEGACY_REPLAY_SLOT_KEY_DOMAIN);
    hasher.update(caller.as_slice());
    hasher.update(subnet_id.as_slice());
    hasher.update(request_id);
    ReplaySlotKey(hasher.finalize().into())
}

pub(super) fn hash_domain_separated(domain: &[u8], payload: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update((domain.len() as u64).to_be_bytes());
    hasher.update(domain);
    hasher.update((payload.len() as u64).to_be_bytes());
    hasher.update(payload);
    hasher.finalize().into()
}
