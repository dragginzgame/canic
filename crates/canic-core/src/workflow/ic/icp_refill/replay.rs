//! Module: workflow::ic::icp_refill::replay
//!
//! Responsibility: bind ICP refill requests and effects to shared replay receipts.
//! Does not own: ledger/CMC execution, stable records, or cost guard accounting.
//! Boundary: maps generic replay ops into ICP refill workflow decisions.

use crate::{
    InternalError, InternalErrorOrigin,
    cdk::types::Principal,
    domain::icp_refill::IcpRefillMode,
    dto::{
        error::Error,
        icp_refill::{IcpRefillRequest, IcpRefillResponse},
    },
    model::replay::{
        CommandKind, ExternalEffectDescriptor, OperationId, RecoveryReason, ReplayActor,
        ReplayPayloadHasher, ReplayReceipt,
    },
    ops::{
        ic::IcOps,
        replay::{
            self as replay_ops, ICP_REFILL_REPLAY_RESPONSE_SCHEMA_VERSION,
            receipt::{
                ReplayReceiptDecision, ReplayReceiptReserveInput, ReplayReceiptStoreError,
                ReplayReceiptToken, abort_uncommitted_receipt, commit_staged_receipt_response,
                mark_external_effect_in_flight, mark_recovery_required,
                replay_cost_guard_settlement, reserve_or_replay_receipt, stage_receipt_response,
            },
        },
        storage::icp_refill::IcpRefillStoreOps,
    },
    view::icp_refill::IcpRefillOperation,
    workflow::{
        cost_guard::CostGuardWorkflow,
        ic::icp_refill::{
            ICP_REFILL_REPLAY_COMMAND_KIND,
            cost_guard::{complete_icp_refill_cost_guard, recover_icp_refill_cost_guard},
        },
        replay::mark_recovery_required_after_failure,
    },
};

///
/// IcpRefillReplayReservation
///
/// Replay reservation outcome for one ICP refill request.
/// Owned by ICP refill workflow and mapped into execution or cached response paths.
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
        ReplayReceiptDecision::RecoveryRequired {
            token,
            reason:
                reason @ (RecoveryReason::CostSettlementFailed | RecoveryReason::ResponseCommitFailed),
        } => recover_icp_refill_response(&token, reason).map(IcpRefillReplayReservation::Replay),
        ReplayReceiptDecision::RecoveryRequired { reason, .. } => {
            log_icp_refill_replay_conflict(operation_id, "recovery_required");
            Err(InternalError::public(Error::conflict(format!(
                "ICP refill request requires recovery before replay: {reason:?}"
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
    operation: &IcpRefillOperation,
    response: &IcpRefillResponse,
    cost_permit: Option<&crate::ops::cost_guard::CostGuardPermit>,
) -> Result<(), InternalError> {
    if IcpRefillStoreOps::is_resumable(operation) {
        recover_icp_refill_cost_guard(cost_permit)?;
        log_icp_refill_resumable_abort(operation);
        abort_uncommitted_receipt(token).map_err(map_icp_refill_replay_store_error)?;
        return Ok(());
    }

    let response_bytes = match encode_icp_refill_replay_response(response) {
        Ok(response_bytes) => response_bytes,
        Err(err) => {
            return Err(preserve_icp_refill_response_failure(
                token,
                cost_permit,
                err,
            ));
        }
    };

    if let Err(err) = stage_receipt_response(
        token,
        ICP_REFILL_REPLAY_RESPONSE_SCHEMA_VERSION,
        response_bytes,
        IcOps::now_nanos(),
    ) {
        return Err(preserve_icp_refill_response_failure(
            token,
            cost_permit,
            map_icp_refill_replay_store_error(err),
        ));
    }

    if let Err(err) = complete_icp_refill_cost_guard(cost_permit) {
        if let Err(recovery_err) = mark_recovery_required(
            token,
            RecoveryReason::CostSettlementFailed,
            IcOps::now_nanos(),
        )
        .map_err(map_icp_refill_replay_store_error)
        {
            return Err(err.with_diagnostic_context(format!(
                "ICP refill replay recovery marker failed: {recovery_err}"
            )));
        }
        return Err(err);
    }
    if let Err(err) = commit_staged_receipt_response(token, IcOps::now_nanos()) {
        let mut err = map_icp_refill_replay_store_error(err);
        if let Err(recovery_err) = mark_recovery_required(
            token,
            RecoveryReason::ResponseCommitFailed,
            IcOps::now_nanos(),
        )
        .map_err(map_icp_refill_replay_store_error)
        {
            err = err.with_diagnostic_context(format!(
                "ICP refill replay recovery marker failed: {recovery_err}"
            ));
        }
        return Err(err);
    }
    log_icp_refill_commit(operation);
    Ok(())
}

fn preserve_icp_refill_response_failure(
    token: &ReplayReceiptToken,
    cost_permit: Option<&crate::ops::cost_guard::CostGuardPermit>,
    mut err: InternalError,
) -> InternalError {
    let reason = match complete_icp_refill_cost_guard(cost_permit) {
        Ok(()) => RecoveryReason::ResponseCommitFailed,
        Err(settlement_err) => {
            err = err.with_diagnostic_context(format!(
                "ICP refill cost settlement also failed: {settlement_err}"
            ));
            RecoveryReason::CostSettlementFailed
        }
    };
    if let Err(recovery_err) = mark_recovery_required(token, reason, IcOps::now_nanos())
        .map_err(map_icp_refill_replay_store_error)
    {
        err = err.with_diagnostic_context(format!(
            "ICP refill replay recovery marker failed: {recovery_err}"
        ));
    }
    err
}

fn recover_icp_refill_response(
    token: &ReplayReceiptToken,
    reason: RecoveryReason,
) -> Result<IcpRefillResponse, InternalError> {
    let cost_settled = reason == RecoveryReason::CostSettlementFailed;
    if cost_settled {
        let settlement =
            replay_cost_guard_settlement(token).map_err(map_icp_refill_replay_store_error)?;
        CostGuardWorkflow::complete_replay_settlement(&settlement, IcOps::now_secs())?;
    }
    let receipt = match commit_staged_receipt_response(token, IcOps::now_nanos()) {
        Ok(receipt) => receipt,
        Err(err) => {
            let mut err = map_icp_refill_replay_store_error(err);
            if cost_settled
                && let Err(recovery_err) = mark_recovery_required(
                    token,
                    RecoveryReason::ResponseCommitFailed,
                    IcOps::now_nanos(),
                )
                .map_err(map_icp_refill_replay_store_error)
            {
                err = err.with_diagnostic_context(format!(
                    "ICP refill response recovery marker failed: {recovery_err}"
                ));
            }
            return Err(err);
        }
    };
    decode_icp_refill_replay_response(&receipt)
}

pub(super) fn mark_icp_refill_transfer_effect(
    token: &ReplayReceiptToken,
    operation: &IcpRefillOperation,
) -> Result<(), InternalError> {
    mark_external_effect_in_flight(
        token,
        ExternalEffectDescriptor::IcpTransfer {
            operation_id: OperationId::from_bytes(operation.operation_id),
        },
        IcOps::now_nanos(),
    )
    .map_err(map_icp_refill_replay_store_error)?;
    crate::log!(
        crate::log::Topic::Cycles,
        Info,
        "icp refill replay effect marked effect=ledger_transfer command_kind={} operation_id={} record_id={} source={} target={} amount_e8s={}",
        ICP_REFILL_REPLAY_COMMAND_KIND,
        operation_id_display(operation.operation_id),
        operation.id,
        operation.source_canister,
        operation.target_canister,
        operation.amount_e8s
    );
    Ok(())
}

pub(super) fn mark_icp_refill_notify_effect(
    token: &ReplayReceiptToken,
    operation: &IcpRefillOperation,
) -> Result<(), InternalError> {
    mark_external_effect_in_flight(
        token,
        ExternalEffectDescriptor::ManagementCall {
            canister: operation.cmc_canister_id,
            method: "notify_top_up".to_string(),
        },
        IcOps::now_nanos(),
    )
    .map_err(map_icp_refill_replay_store_error)?;
    crate::log!(
        crate::log::Topic::Cycles,
        Info,
        "icp refill replay effect marked effect=cmc_notify_top_up command_kind={} operation_id={} record_id={} source={} target={} amount_e8s={}",
        ICP_REFILL_REPLAY_COMMAND_KIND,
        operation_id_display(operation.operation_id),
        operation.id,
        operation.source_canister,
        operation.target_canister,
        operation.amount_e8s
    );
    Ok(())
}

pub(super) fn preserve_icp_refill_recovery_required(
    token: &ReplayReceiptToken,
    operation: &IcpRefillOperation,
    effect: &'static str,
    err: InternalError,
) -> InternalError {
    let (error_class, error_origin) = err.log_fields();
    let err = mark_recovery_required_after_failure(
        token,
        RecoveryReason::ExternalEffectStatusUnknown,
        IcOps::now_nanos(),
        err,
        "ICP refill replay recovery marker failed",
    );
    crate::log!(
        crate::log::Topic::Cycles,
        Error,
        "icp refill replay recovery required effect={} command_kind={} operation_id={} record_id={} source={} target={} amount_e8s={} error_class={} error_origin={}",
        effect,
        ICP_REFILL_REPLAY_COMMAND_KIND,
        operation_id_display(operation.operation_id),
        operation.id,
        operation.source_canister,
        operation.target_canister,
        operation.amount_e8s,
        error_class,
        error_origin
    );
    err
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

pub(super) fn log_icp_refill_resumable_abort(operation: &IcpRefillOperation) {
    crate::log!(
        crate::log::Topic::Cycles,
        Info,
        "icp refill replay receipt aborted for resumable record command_kind={} operation_id={} record_id={} source={} target={} amount_e8s={} status={:?}",
        ICP_REFILL_REPLAY_COMMAND_KIND,
        operation_id_display(operation.operation_id),
        operation.id,
        operation.source_canister,
        operation.target_canister,
        operation.amount_e8s,
        operation.status
    );
}

pub(super) fn operation_id_display(operation_id: [u8; 32]) -> String {
    OperationId::from_bytes(operation_id).to_string()
}

pub(super) fn log_icp_refill_commit(operation: &IcpRefillOperation) {
    crate::log!(
        crate::log::Topic::Cycles,
        Ok,
        "icp refill replay response committed command_kind={} operation_id={} record_id={} source={} target={} amount_e8s={} status={:?}",
        ICP_REFILL_REPLAY_COMMAND_KIND,
        operation_id_display(operation.operation_id),
        operation.id,
        operation.source_canister,
        operation.target_canister,
        operation.amount_e8s,
        operation.status
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
    replay_ops::encode_icp_refill_replay_response(response).map_err(|err| match err {
        replay_ops::ReplayCommitError::EncodeFailed(message) => InternalError::workflow(
            InternalErrorOrigin::Workflow,
            format!("failed to encode ICP refill replay response: {message}"),
        ),
    })
}

fn decode_icp_refill_replay_response(
    receipt: &ReplayReceipt,
) -> Result<IcpRefillResponse, InternalError> {
    replay_ops::decode_icp_refill_replay_response(receipt).map_err(|err| match err {
        replay_ops::ReplayDecodeError::DecodeFailed(message) => {
            InternalError::workflow(InternalErrorOrigin::Workflow, message)
        }
    })
}

pub(super) fn map_icp_refill_replay_store_error(err: ReplayReceiptStoreError) -> InternalError {
    match err {
        ReplayReceiptStoreError::ReceiptMissing => InternalError::workflow(
            InternalErrorOrigin::Workflow,
            "ICP refill replay receipt is missing",
        ),
        ReplayReceiptStoreError::ReceiptDecodeFailed(message) => InternalError::workflow(
            InternalErrorOrigin::Workflow,
            format!("failed to decode ICP refill replay receipt: {message}"),
        ),
        ReplayReceiptStoreError::ReceiptTokenMismatch => InternalError::workflow(
            InternalErrorOrigin::Workflow,
            "ICP refill replay receipt token is stale",
        ),
        ReplayReceiptStoreError::StagedResponseMissing => InternalError::workflow(
            InternalErrorOrigin::Workflow,
            "ICP refill replay receipt is missing staged response data",
        ),
        ReplayReceiptStoreError::CostGuardSettlementMissing => InternalError::workflow(
            InternalErrorOrigin::Workflow,
            "ICP refill replay receipt is missing cost guard settlement identity",
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
