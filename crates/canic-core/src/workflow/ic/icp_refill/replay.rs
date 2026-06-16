use super::{
    ICP_REFILL_REPLAY_COMMAND_KIND, ICP_REFILL_REPLAY_RESPONSE_SCHEMA_VERSION,
    cost_guard::{complete_icp_refill_cost_guard, recover_icp_refill_cost_guard},
};
use crate::{
    InternalError, InternalErrorOrigin,
    dto::{
        error::Error,
        icp_refill::{IcpRefillMode, IcpRefillRequest, IcpRefillResponse},
    },
    ops::{
        ic::IcOps,
        replay::{
            model::{
                CommandKind, ExternalEffectDescriptor, OperationId, RecoveryReason, ReplayActor,
                ReplayPayloadHasher, ReplayReceipt,
            },
            receipt::{
                ReplayReceiptDecision, ReplayReceiptReserveInput, ReplayReceiptStoreError,
                ReplayReceiptToken, abort_uncommitted_receipt, commit_receipt_response,
                mark_external_effect_in_flight, mark_recovery_required, reserve_or_replay_receipt,
            },
        },
        storage::icp_refill::IcpRefillRecordOps,
    },
    storage::stable::icp_refill::IcpRefillRecord,
    workflow::prelude::*,
};
use candid::{decode_one, encode_one};

///
/// IcpRefillReplayReservation
///

#[derive(Debug)]
pub(super) enum IcpRefillReplayReservation {
    Fresh {
        operation_id: [u8; 32],
        token: Box<ReplayReceiptToken>,
    },
    Replay(IcpRefillResponse),
}

pub(super) fn icp_refill_replay_reserve_input(
    request: &IcpRefillRequest,
    caller: Principal,
    now_ns: u64,
) -> ReplayReceiptReserveInput {
    let command_kind = icp_refill_command_kind();
    let actor = icp_refill_replay_actor(caller);
    let payload_hash = icp_refill_payload_hash(&command_kind, &actor, request);

    ReplayReceiptReserveInput::new(
        command_kind,
        icp_refill_operation_id(request),
        actor,
        payload_hash,
        now_ns,
    )
}

pub(super) fn reserve_icp_refill_replay(
    input: ReplayReceiptReserveInput,
) -> Result<IcpRefillReplayReservation, InternalError> {
    let operation_id = input.operation_id.into_bytes();
    match reserve_or_replay_receipt(input).map_err(map_icp_refill_replay_store_error)? {
        ReplayReceiptDecision::Fresh(token) => Ok(IcpRefillReplayReservation::Fresh {
            operation_id,
            token: Box::new(token),
        }),
        ReplayReceiptDecision::ReturnCommitted(receipt) => {
            decode_icp_refill_replay_response(&receipt).map(IcpRefillReplayReservation::Replay)
        }
        ReplayReceiptDecision::OperationInProgress => {
            log_icp_refill_replay_conflict(operation_id, "operation_in_progress");
            Err(InternalError::public(Error::conflict(
                "ICP refill request is already in progress; retry later with the same operation id",
            )))
        }
        ReplayReceiptDecision::ActorMismatch => {
            log_icp_refill_replay_conflict(operation_id, "actor_mismatch");
            Err(InternalError::public(Error::conflict(
                "ICP refill operation id was reused by a different caller",
            )))
        }
        ReplayReceiptDecision::PayloadMismatch => {
            log_icp_refill_replay_conflict(operation_id, "payload_mismatch");
            Err(InternalError::public(Error::conflict(
                "ICP refill operation id was reused with a different payload",
            )))
        }
        ReplayReceiptDecision::Expired => {
            log_icp_refill_replay_conflict(operation_id, "expired");
            Err(InternalError::public(Error::conflict(
                "ICP refill replay receipt expired; retry with a new operation id",
            )))
        }
        ReplayReceiptDecision::RecoveryRequired(reason) => {
            log_icp_refill_replay_conflict(operation_id, "recovery_required");
            Err(InternalError::public(Error::conflict(format!(
                "ICP refill request requires recovery before replay: {reason:?}"
            ))))
        }
        ReplayReceiptDecision::TerminalFailed {
            error_code,
            error_bytes,
            error_bytes_truncated,
        } => {
            log_icp_refill_replay_conflict(operation_id, "terminal_failed");
            Err(InternalError::public(Error::conflict(format!(
                "ICP refill request previously failed: {error_code:?}; error_bytes_len={}; truncated={error_bytes_truncated}",
                error_bytes.len()
            ))))
        }
        ReplayReceiptDecision::PendingActorQuotaExceeded { max_pending, .. } => {
            log_icp_refill_replay_conflict(operation_id, "pending_actor_quota_exceeded");
            Err(InternalError::public(Error::exhausted(format!(
                "ICP refill pending replay receipt quota exceeded for caller; max_pending={max_pending}"
            ))))
        }
        ReplayReceiptDecision::PendingCommandQuotaExceeded { max_pending, .. } => {
            log_icp_refill_replay_conflict(operation_id, "pending_command_quota_exceeded");
            Err(InternalError::public(Error::exhausted(format!(
                "ICP refill pending replay receipt quota exceeded for command kind; max_pending={max_pending}"
            ))))
        }
    }
}

pub(super) fn finish_icp_refill_replay(
    token: &ReplayReceiptToken,
    record: &IcpRefillRecord,
    response: &IcpRefillResponse,
    cost_permit: Option<&crate::ops::cost_guard::CostGuardPermit>,
) -> Result<(), InternalError> {
    if IcpRefillRecordOps::is_resumable(record) {
        recover_icp_refill_cost_guard(cost_permit);
        log_icp_refill_resumable_abort(record);
        abort_uncommitted_receipt(token);
        return Ok(());
    }

    let response_bytes = match encode_icp_refill_replay_response(response) {
        Ok(response_bytes) => response_bytes,
        Err(err) => {
            recover_icp_refill_cost_guard(cost_permit);
            mark_recovery_required(
                token,
                RecoveryReason::ResponseCommitFailed,
                IcOps::now_nanos(),
            );
            return Err(err);
        }
    };

    commit_receipt_response(
        token,
        ICP_REFILL_REPLAY_RESPONSE_SCHEMA_VERSION,
        response_bytes,
        IcOps::now_nanos(),
    );
    complete_icp_refill_cost_guard(cost_permit);
    log_icp_refill_commit(record);
    Ok(())
}

pub(super) fn mark_icp_refill_transfer_effect(
    token: &ReplayReceiptToken,
    record: &IcpRefillRecord,
) {
    mark_external_effect_in_flight(
        token,
        ExternalEffectDescriptor::IcpTransfer {
            operation_id: OperationId::from_bytes(record.operation_id),
        },
        IcOps::now_nanos(),
    );
    crate::log!(
        crate::log::Topic::Cycles,
        Info,
        "icp refill replay effect marked effect=ledger_transfer command_kind={} operation_id={} record_id={} source={} target={} amount_e8s={}",
        ICP_REFILL_REPLAY_COMMAND_KIND,
        operation_id_display(record.operation_id),
        record.id,
        record.source_canister,
        record.target_canister,
        record.amount_e8s
    );
}

pub(super) fn mark_icp_refill_notify_effect(token: &ReplayReceiptToken, record: &IcpRefillRecord) {
    mark_external_effect_in_flight(
        token,
        ExternalEffectDescriptor::ManagementCall {
            canister: record.cmc_canister_id,
            method: "notify_top_up".to_string(),
        },
        IcOps::now_nanos(),
    );
    crate::log!(
        crate::log::Topic::Cycles,
        Info,
        "icp refill replay effect marked effect=cmc_notify_top_up command_kind={} operation_id={} record_id={} source={} target={} amount_e8s={}",
        ICP_REFILL_REPLAY_COMMAND_KIND,
        operation_id_display(record.operation_id),
        record.id,
        record.source_canister,
        record.target_canister,
        record.amount_e8s
    );
}

pub(super) fn mark_icp_refill_recovery_required(
    token: &ReplayReceiptToken,
    record: &IcpRefillRecord,
    effect: &'static str,
    err: &InternalError,
) {
    let (error_class, error_origin) = err.log_fields();
    mark_recovery_required(
        token,
        RecoveryReason::ExternalEffectStatusUnknown,
        IcOps::now_nanos(),
    );
    crate::log!(
        crate::log::Topic::Cycles,
        Error,
        "icp refill replay recovery required effect={} command_kind={} operation_id={} record_id={} source={} target={} amount_e8s={} error_class={} error_origin={}",
        effect,
        ICP_REFILL_REPLAY_COMMAND_KIND,
        operation_id_display(record.operation_id),
        record.id,
        record.source_canister,
        record.target_canister,
        record.amount_e8s,
        error_class,
        error_origin
    );
}

pub(super) fn log_icp_refill_fresh_reservation(request: &IcpRefillRequest) {
    crate::log!(
        crate::log::Topic::Cycles,
        Info,
        "icp refill replay receipt reserved command_kind={} operation_id={} source={} target={} amount_e8s={}",
        ICP_REFILL_REPLAY_COMMAND_KIND,
        operation_id_display(request.operation_id),
        request.source_canister,
        request.target_canister,
        request.amount_e8s
    );
}

pub(super) fn log_icp_refill_committed_replay(response: &IcpRefillResponse) {
    crate::log!(
        crate::log::Topic::Cycles,
        Info,
        "icp refill committed replay returned command_kind={} operation_id={} status={:?}",
        ICP_REFILL_REPLAY_COMMAND_KIND,
        operation_id_display(response.operation_id),
        response.status
    );
}

pub(super) fn log_icp_refill_replay_conflict(operation_id: [u8; 32], decision: &'static str) {
    crate::log!(
        crate::log::Topic::Cycles,
        Warn,
        "icp refill replay decision blocked command_kind={} operation_id={} decision={}",
        ICP_REFILL_REPLAY_COMMAND_KIND,
        operation_id_display(operation_id),
        decision
    );
}

pub(super) fn log_icp_refill_resumable_abort(record: &IcpRefillRecord) {
    crate::log!(
        crate::log::Topic::Cycles,
        Info,
        "icp refill replay receipt aborted for resumable record command_kind={} operation_id={} record_id={} source={} target={} amount_e8s={} status={:?}",
        ICP_REFILL_REPLAY_COMMAND_KIND,
        operation_id_display(record.operation_id),
        record.id,
        record.source_canister,
        record.target_canister,
        record.amount_e8s,
        record.status
    );
}

pub(super) fn operation_id_display(operation_id: [u8; 32]) -> String {
    OperationId::from_bytes(operation_id).to_string()
}

pub(super) fn log_icp_refill_commit(record: &IcpRefillRecord) {
    crate::log!(
        crate::log::Topic::Cycles,
        Ok,
        "icp refill replay response committed command_kind={} operation_id={} record_id={} source={} target={} amount_e8s={} status={:?}",
        ICP_REFILL_REPLAY_COMMAND_KIND,
        operation_id_display(record.operation_id),
        record.id,
        record.source_canister,
        record.target_canister,
        record.amount_e8s,
        record.status
    );
}

pub(super) const fn icp_refill_operation_id(request: &IcpRefillRequest) -> OperationId {
    OperationId::from_bytes(request.operation_id)
}

pub(super) fn icp_refill_command_kind() -> CommandKind {
    CommandKind::new(ICP_REFILL_REPLAY_COMMAND_KIND)
        .expect("ICP refill replay command kind is a valid static label")
}

pub(super) const fn icp_refill_replay_actor(caller: Principal) -> ReplayActor {
    ReplayActor::direct_caller(caller)
}

pub(super) fn icp_refill_payload_hash(
    command_kind: &CommandKind,
    actor: &ReplayActor,
    request: &IcpRefillRequest,
) -> [u8; 32] {
    let mut hasher = ReplayPayloadHasher::new(command_kind, actor);
    hasher.hash_str("IcpRefill");
    hasher.hash_principal(&request.source_canister);
    hash_optional_subaccount(&mut hasher, request.source_subaccount);
    hasher.hash_principal(&request.target_canister);
    hasher.hash_u64(request.amount_e8s);
    hasher.hash_str(icp_refill_mode_label(request.mode));
    hasher.finish()
}

fn encode_icp_refill_replay_response(
    response: &IcpRefillResponse,
) -> Result<Vec<u8>, InternalError> {
    encode_one(response).map_err(|err| {
        InternalError::workflow(
            InternalErrorOrigin::Workflow,
            format!("failed to encode ICP refill replay response: {err}"),
        )
    })
}

fn decode_icp_refill_replay_response(
    receipt: &ReplayReceipt,
) -> Result<IcpRefillResponse, InternalError> {
    let response_schema_version = receipt.response_schema_version.ok_or_else(|| {
        InternalError::workflow(
            InternalErrorOrigin::Workflow,
            "ICP refill replay receipt is missing response schema version",
        )
    })?;
    if response_schema_version != ICP_REFILL_REPLAY_RESPONSE_SCHEMA_VERSION {
        return Err(InternalError::workflow(
            InternalErrorOrigin::Workflow,
            format!(
                "unsupported ICP refill replay response schema version {response_schema_version}"
            ),
        ));
    }
    let response_bytes = receipt.response_bytes.as_deref().ok_or_else(|| {
        InternalError::workflow(
            InternalErrorOrigin::Workflow,
            "ICP refill replay receipt is missing response bytes",
        )
    })?;
    decode_one(response_bytes).map_err(|err| {
        InternalError::workflow(
            InternalErrorOrigin::Workflow,
            format!("failed to decode ICP refill replay response: {err}"),
        )
    })
}

fn map_icp_refill_replay_store_error(err: ReplayReceiptStoreError) -> InternalError {
    match err {
        ReplayReceiptStoreError::ReceiptDecodeFailed(message) => InternalError::workflow(
            InternalErrorOrigin::Workflow,
            format!("failed to decode ICP refill replay receipt: {message}"),
        ),
    }
}

fn hash_optional_subaccount(hasher: &mut ReplayPayloadHasher, subaccount: Option<[u8; 32]>) {
    hasher.hash_bool(subaccount.is_some());
    if let Some(subaccount) = subaccount {
        hasher.hash_bytes(&subaccount);
    }
}

const fn icp_refill_mode_label(mode: IcpRefillMode) -> &'static str {
    match mode {
        IcpRefillMode::Canister => "canister",
        IcpRefillMode::Fabricate => "fabricate",
    }
}
