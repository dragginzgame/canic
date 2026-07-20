//! Module: workflow::rpc::request::handler::replay
//!
//! Responsibility: gate root-capability execution with replay protection.
//! Does not own: capability authorization, capability execution, or replay record schema.
//! Boundary: adapts workflow capability metadata into replay ops and cached responses.

use super::{
    MAX_ROOT_REPLAY_ENTRIES, MAX_ROOT_REPLAY_ENTRIES_PER_CALLER, MAX_ROOT_TTL_NS,
    REPLAY_PAYLOAD_HASH_DOMAIN, REPLAY_PURGE_SCAN_LIMIT, RootCapability, RootContext,
    RootReplayInput,
};
use crate::{
    InternalError,
    cdk::types::Principal,
    dto::rpc::Response,
    ids::CanisterRole,
    model::replay::{CommandKind, ExternalEffectDescriptor, OperationId, RecoveryReason},
    ops::{
        ic::IcOps,
        replay::{
            self as replay_ops, ReplayCommitError, ReplayDecodeError, ReplayFinalizeError,
            ReplayReserveError,
            guard::secs_to_ns,
            guard::{ReplayDecision, ReplayGuardError, ReplayPending, RootReplayGuardInput},
            receipt::ReplayReceiptStoreError,
        },
        runtime::metrics::replay::{
            ReplayMetricOperation, ReplayMetricOutcome, ReplayMetricReason, ReplayMetrics,
        },
        runtime::metrics::root_capability::{
            RootCapabilityMetricKey, RootCapabilityMetricOutcome, RootCapabilityMetrics,
        },
    },
    workflow::{cost_guard::CostGuardWorkflow, rpc::RpcWorkflowError},
};
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
    let (replay_input, decision) = evaluate_replay(ctx, capability)?;
    match decision {
        ReplayDecision::Fresh(pending) => {
            ReplayMetrics::record(
                ReplayMetricOperation::Check,
                ReplayMetricOutcome::Completed,
                ReplayMetricReason::Fresh,
            );
            replay_ops::reserve_root_replay(
                &pending,
                MAX_ROOT_REPLAY_ENTRIES,
                MAX_ROOT_REPLAY_ENTRIES_PER_CALLER,
            )
            .map_err(map_replay_reserve_error)?;
            ReplayMetrics::record(
                ReplayMetricOperation::Reserve,
                ReplayMetricOutcome::Completed,
                ReplayMetricReason::Ok,
            );
            crate::perf!("reserve_fresh");
            RootCapabilityMetrics::record_replay(
                replay_input.descriptor.key,
                RootCapabilityMetricOutcome::Accepted,
            );
            Ok(ReplayPreflight::Fresh(pending))
        }
        other => map_existing_replay_decision(ctx, replay_input, other),
    }
}

fn evaluate_replay(
    ctx: &RootContext,
    capability: &RootCapability,
) -> Result<(RootReplayInput, ReplayDecision), InternalError> {
    let replay_input = capability.replay_input().ok_or_else(|| {
        ReplayMetrics::record(
            ReplayMetricOperation::Check,
            ReplayMetricOutcome::Failed,
            ReplayMetricReason::MissingMetadata,
        );
        RpcWorkflowError::MissingReplayMetadata(capability.descriptor().name)
    })?;
    crate::perf!("prepare_replay_input");

    let decision = replay::evaluate_root_replay(
        ctx,
        replay_input.descriptor.command_kind,
        OperationId::from_bytes(replay_input.metadata.request_id),
        replay_input.metadata.ttl_ns,
        replay_input.payload_hash,
    )
    .map_err(|err| map_replay_guard_error(replay_input.descriptor.key, err))?;
    crate::perf!("evaluate_replay");

    Ok((replay_input, decision))
}

fn map_existing_replay_decision(
    ctx: &RootContext,
    replay_input: RootReplayInput,
    decision: ReplayDecision,
) -> Result<ReplayPreflight, InternalError> {
    match decision {
        ReplayDecision::Fresh(_) => unreachable!("fresh replay decisions are handled by callers"),
        ReplayDecision::DuplicateSame(cached) => {
            crate::perf!("decode_cached");
            ReplayMetrics::record(
                ReplayMetricOperation::Check,
                ReplayMetricOutcome::Completed,
                ReplayMetricReason::Duplicate,
            );
            RootCapabilityMetrics::record_replay(
                replay_input.descriptor.key,
                RootCapabilityMetricOutcome::DuplicateSame,
            );
            decode_replay_response(&cached.response_bytes).map(ReplayPreflight::Cached)
        }
        ReplayDecision::InFlight => {
            crate::perf!("duplicate_in_flight");
            ReplayMetrics::record(
                ReplayMetricOperation::Check,
                ReplayMetricOutcome::Failed,
                ReplayMetricReason::InFlight,
            );
            RootCapabilityMetrics::record_replay(
                replay_input.descriptor.key,
                RootCapabilityMetricOutcome::DuplicateSame,
            );
            Err(RpcWorkflowError::ReplayDuplicateSame(replay_input.descriptor.name).into())
        }
        ReplayDecision::RecoveryRequired {
            pending,
            reason:
                reason @ (RecoveryReason::CostSettlementFailed | RecoveryReason::ResponseCommitFailed),
        } => {
            let response = recover_staged_response(ctx, &pending, reason)?;
            ReplayMetrics::record(
                ReplayMetricOperation::Check,
                ReplayMetricOutcome::Completed,
                ReplayMetricReason::Duplicate,
            );
            RootCapabilityMetrics::record_replay(
                replay_input.descriptor.key,
                RootCapabilityMetricOutcome::DuplicateSame,
            );
            Ok(ReplayPreflight::Cached(response))
        }
        ReplayDecision::RecoveryRequired { .. } => {
            ReplayMetrics::record(
                ReplayMetricOperation::Check,
                ReplayMetricOutcome::Failed,
                ReplayMetricReason::InFlight,
            );
            Err(RpcWorkflowError::ReplayDuplicateSame(replay_input.descriptor.name).into())
        }
        ReplayDecision::DuplicateConflict => {
            crate::perf!("duplicate_conflict");
            ReplayMetrics::record(
                ReplayMetricOperation::Check,
                ReplayMetricOutcome::Failed,
                ReplayMetricReason::Conflict,
            );
            RootCapabilityMetrics::record_replay(
                replay_input.descriptor.key,
                RootCapabilityMetricOutcome::DuplicateConflict,
            );
            Err(RpcWorkflowError::ReplayConflict(replay_input.descriptor.name).into())
        }
        ReplayDecision::Expired => {
            crate::perf!("replay_expired");
            ReplayMetrics::record(
                ReplayMetricOperation::Check,
                ReplayMetricOutcome::Failed,
                ReplayMetricReason::Expired,
            );
            RootCapabilityMetrics::record_replay(
                replay_input.descriptor.key,
                RootCapabilityMetricOutcome::Expired,
            );
            Err(RpcWorkflowError::ReplayExpired(replay_input.descriptor.name).into())
        }
        ReplayDecision::DecodeFailed(message) => Err(map_replay_decode_error(
            ReplayDecodeError::DecodeFailed(message),
        )),
    }
}

pub(super) fn recover_staged_response(
    ctx: &RootContext,
    pending: &ReplayPending,
    reason: RecoveryReason,
) -> Result<Response, InternalError> {
    let cost_settled = reason == RecoveryReason::CostSettlementFailed;
    if cost_settled {
        let settlement = replay_ops::root_replay_cost_guard_settlement(pending)
            .map_err(map_replay_store_error)?;
        CostGuardWorkflow::complete_replay_settlement(&settlement, ctx.now)?;
    }
    let receipt = match replay_ops::commit_staged_root_replay_response(pending, secs_to_ns(ctx.now))
    {
        Ok(receipt) => receipt,
        Err(err) => {
            let mut err = map_replay_store_error(err);
            if cost_settled
                && let Err(recovery_err) = replay_ops::mark_root_replay_recovery_required(
                    pending,
                    RecoveryReason::ResponseCommitFailed,
                    secs_to_ns(ctx.now),
                )
                .map_err(map_replay_store_error)
            {
                err = err.with_diagnostic_context(format!(
                    "root replay response recovery marker failed: {recovery_err}"
                ));
            }
            return Err(err);
        }
    };
    let response_bytes = receipt.response_bytes.ok_or_else(|| {
        map_replay_decode_error(ReplayDecodeError::DecodeFailed(
            "recovered root replay receipt is missing response bytes".to_string(),
        ))
    })?;
    decode_replay_response(&response_bytes)
}

/// map_replay_guard_error
///
/// Convert guard-level infra errors into workflow replay failures.
fn map_replay_guard_error(
    capability_key: RootCapabilityMetricKey,
    err: ReplayGuardError,
) -> InternalError {
    match err {
        ReplayGuardError::InvalidTtl { ttl_ns, max_ttl_ns } => {
            ReplayMetrics::record(
                ReplayMetricOperation::Check,
                ReplayMetricOutcome::Failed,
                ReplayMetricReason::InvalidTtl,
            );
            RootCapabilityMetrics::record_replay(
                capability_key,
                RootCapabilityMetricOutcome::TtlExceeded,
            );
            RpcWorkflowError::InvalidReplayTtl { ttl_ns, max_ttl_ns }.into()
        }
        ReplayGuardError::TtlOverflow { now_ns, ttl_ns } => {
            ReplayMetrics::record(
                ReplayMetricOperation::Check,
                ReplayMetricOutcome::Failed,
                ReplayMetricReason::InvalidTtl,
            );
            RootCapabilityMetrics::record_replay(
                capability_key,
                RootCapabilityMetricOutcome::TtlExceeded,
            );
            RpcWorkflowError::ReplayTtlOverflow { now_ns, ttl_ns }.into()
        }
        ReplayGuardError::ReceiptDecodeFailed(message) => {
            map_replay_decode_error(ReplayDecodeError::DecodeFailed(message))
        }
    }
}

/// map_replay_reserve_error
///
/// Convert ops replay-reservation failures into workflow replay errors.
fn map_replay_reserve_error(err: ReplayReserveError) -> InternalError {
    match err {
        ReplayReserveError::CapacityReached { max_entries } => {
            ReplayMetrics::record(
                ReplayMetricOperation::Reserve,
                ReplayMetricOutcome::Failed,
                ReplayMetricReason::Capacity,
            );
            RpcWorkflowError::ReplayStoreCapacityReached(max_entries).into()
        }
        ReplayReserveError::CallerCapacityReached {
            caller,
            max_entries,
        } => {
            ReplayMetrics::record(
                ReplayMetricOperation::Reserve,
                ReplayMetricOutcome::Failed,
                ReplayMetricReason::Capacity,
            );
            RpcWorkflowError::ReplayStoreCallerCapacityReached {
                caller,
                max_entries,
            }
            .into()
        }
    }
}

/// map_replay_commit_error
///
/// Convert ops replay-commit failures into workflow replay errors.
fn map_replay_commit_error(err: ReplayCommitError) -> InternalError {
    match err {
        ReplayCommitError::EncodeFailed(message) => {
            ReplayMetrics::record(
                ReplayMetricOperation::Commit,
                ReplayMetricOutcome::Failed,
                ReplayMetricReason::EncodeFailed,
            );
            RpcWorkflowError::ReplayEncodeFailed(message).into()
        }
    }
}

fn map_replay_finalize_error(err: ReplayFinalizeError) -> InternalError {
    match err {
        ReplayFinalizeError::Encode(err) => map_replay_commit_error(err),
        ReplayFinalizeError::Store(err) => map_replay_store_error(err),
    }
}

pub(super) fn map_replay_store_error(err: ReplayReceiptStoreError) -> InternalError {
    match err {
        ReplayReceiptStoreError::ReceiptMissing => map_replay_decode_error(
            ReplayDecodeError::DecodeFailed("reserved replay receipt is missing".to_string()),
        ),
        ReplayReceiptStoreError::ReceiptDecodeFailed(message) => {
            map_replay_decode_error(ReplayDecodeError::DecodeFailed(message))
        }
        ReplayReceiptStoreError::ReceiptTokenMismatch => {
            map_replay_decode_error(ReplayDecodeError::DecodeFailed(
                "replay receipt token no longer matches persisted receipt identity".to_string(),
            ))
        }
        ReplayReceiptStoreError::StagedResponseMissing => {
            map_replay_decode_error(ReplayDecodeError::DecodeFailed(
                "replay receipt is missing staged response data".to_string(),
            ))
        }
        ReplayReceiptStoreError::CostGuardSettlementMissing => {
            map_replay_decode_error(ReplayDecodeError::DecodeFailed(
                "replay receipt is missing cost guard settlement identity".to_string(),
            ))
        }
    }
}

/// map_replay_decode_error
///
/// Convert ops replay-decode failures into workflow replay errors.
fn map_replay_decode_error(err: ReplayDecodeError) -> InternalError {
    match err {
        ReplayDecodeError::DecodeFailed(message) => {
            ReplayMetrics::record(
                ReplayMetricOperation::Decode,
                ReplayMetricOutcome::Failed,
                ReplayMetricReason::DecodeFailed,
            );
            RpcWorkflowError::ReplayDecodeFailed(message).into()
        }
    }
}

/// decode_replay_response
///
/// Decode cached replay payload bytes back into canonical root responses.
fn decode_replay_response(bytes: &[u8]) -> Result<Response, InternalError> {
    match replay_ops::decode_root_replay_response(bytes) {
        Ok(response) => {
            ReplayMetrics::record(
                ReplayMetricOperation::Decode,
                ReplayMetricOutcome::Completed,
                ReplayMetricReason::Ok,
            );
            Ok(response)
        }
        Err(err) => Err(map_replay_decode_error(err)),
    }
}

/// commit_replay
///
/// Persist a replay record after successful capability execution.
pub(super) fn commit_replay(pending: &ReplayPending) -> Result<(), InternalError> {
    match replay_ops::commit_staged_root_replay_response(pending, secs_to_ns(IcOps::now_secs())) {
        Ok(_) => {
            ReplayMetrics::record(
                ReplayMetricOperation::Commit,
                ReplayMetricOutcome::Completed,
                ReplayMetricReason::Ok,
            );
            Ok(())
        }
        Err(err) => Err(map_replay_store_error(err)),
    }
}

/// abort_replay
///
/// Remove reserved replay state when capability execution fails.
pub(super) fn abort_replay(pending: ReplayPending) -> Result<(), InternalError> {
    replay_ops::abort_root_replay(pending).map_err(map_replay_store_error)?;
    ReplayMetrics::record(
        ReplayMetricOperation::Abort,
        ReplayMetricOutcome::Completed,
        ReplayMetricReason::Ok,
    );
    crate::perf!("abort_replay");
    Ok(())
}

/// Abort a pre-effect root reservation without replacing the primary failure.
#[must_use]
pub(super) fn abort_replay_after_failure(
    pending: ReplayPending,
    mut error: InternalError,
) -> InternalError {
    if let Err(cleanup_error) = abort_replay(pending) {
        error = error.with_diagnostic_context(format!(
            "root replay reservation cleanup failed: {cleanup_error}"
        ));
    }
    error
}

/// Record a root external effect together with the cost intents needed for recovery.
pub(super) fn mark_costed_external_effect_in_flight(
    pending: &ReplayPending,
    effect: ExternalEffectDescriptor,
    cost_permit: &crate::ops::cost_guard::CostGuardPermit,
) -> Result<(), InternalError> {
    replay_ops::mark_root_replay_costed_external_effect(
        pending,
        effect,
        cost_permit,
        secs_to_ns(IcOps::now_secs()),
    )
    .map_err(map_replay_store_error)
}

/// Stage the response that an accounting-only retry will commit.
pub(super) fn stage_response(
    pending: &ReplayPending,
    response: &Response,
) -> Result<(), InternalError> {
    replay_ops::stage_root_replay_response(pending, response, secs_to_ns(IcOps::now_secs()))
        .map_err(map_replay_finalize_error)
}

/// mark_recovery_required
///
/// Preserve a root replay receipt for manual recovery after uncertain execution.
pub(super) fn mark_recovery_required(
    pending: &ReplayPending,
    reason: RecoveryReason,
) -> Result<(), InternalError> {
    replay_ops::mark_root_replay_recovery_required(pending, reason, secs_to_ns(IcOps::now_secs()))
        .map_err(map_replay_store_error)
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
        command_kind: &'static str,
        operation_id: OperationId,
        ttl_ns: u64,
        payload_hash: [u8; 32],
    ) -> Result<ReplayDecision, ReplayGuardError> {
        let command_kind =
            CommandKind::new(command_kind).expect("root replay command kind constants are valid");
        crate::ops::replay::guard::evaluate_root_replay(RootReplayGuardInput {
            caller: ctx.caller,
            command_kind,
            operation_id,
            ttl_ns,
            payload_hash,
            now_ns: secs_to_ns(ctx.now),
            max_ttl_ns: MAX_ROOT_TTL_NS,
            purge_scan_limit: REPLAY_PURGE_SCAN_LIMIT,
        })
    }
}
