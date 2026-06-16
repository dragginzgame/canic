use super::{
    MAX_NOTIFY_ATTEMPTS, RateQueryMode, TX_WINDOW_NANOS,
    cost_guard::{require_icp_refill_cost_permit, reserve_icp_refill_cost_guard_if_needed},
    prepare_context,
    replay::{
        finish_icp_refill_replay, mark_icp_refill_notify_effect, mark_icp_refill_recovery_required,
        mark_icp_refill_transfer_effect,
    },
};
use crate::{
    InternalError,
    cdk::{candid::Nat, icrc_ledger_types::icrc1::transfer::TransferError, types::Principal},
    dto::icp_refill::{IcpRefillErrorCode, IcpRefillRequest, IcpRefillResponse, IcpRefillStatus},
    ids::CanisterRole,
    infra::ic::icp_refill::{NotifyTopUpArg, NotifyTopUpError},
    ops::{
        cost_guard::CostGuardPermit,
        ic::{IcOps, icp_refill::IcpRefillOps},
        replay::receipt::{ReplayReceiptToken, abort_reserved_receipt},
        runtime::cycles_funding::CyclesFundingLedgerOps,
        storage::{
            children::CanisterChildrenOps,
            icp_refill::{IcpRefillRecordCreateInput, IcpRefillRecordOps},
        },
    },
    storage::stable::icp_refill::IcpRefillRecord,
};

pub(super) async fn execute_fresh_manual_refill(
    request: IcpRefillRequest,
    operation_id: [u8; 32],
    token: &ReplayReceiptToken,
) -> Result<IcpRefillResponse, InternalError> {
    let mut cost_permit = None;
    let record =
        match execute_manual_refill_record(request, operation_id, token, &mut cost_permit).await {
            Ok(record) => record,
            Err(err) => {
                super::cost_guard::recover_icp_refill_cost_guard(cost_permit.as_ref());
                abort_reserved_receipt(token);
                return Err(err);
            }
        };
    let response = IcpRefillRecordOps::to_response(&record);

    if let Err(err) = finish_icp_refill_replay(token, &record, &response, cost_permit.as_ref()) {
        abort_reserved_receipt(token);
        return Err(err);
    }

    Ok(response)
}

async fn execute_manual_refill_record(
    request: IcpRefillRequest,
    operation_id: [u8; 32],
    token: &ReplayReceiptToken,
    cost_permit: &mut Option<CostGuardPermit>,
) -> Result<IcpRefillRecord, InternalError> {
    if let Some(record) = IcpRefillRecordOps::find_by_operation_id(operation_id) {
        IcpRefillRecordOps::validate_retry_request_matches_record(&request, &record)?;
        return advance_record(record, token, cost_permit).await;
    }

    let context = prepare_context(&request, RateQueryMode::WhenGateConfigured).await?;
    let cmc_account =
        IcpRefillOps::cmc_topup_account(context.cmc_canister_id, request.target_canister)?;
    let record = IcpRefillRecordOps::create_or_get(IcpRefillRecordCreateInput {
        operation_id,
        source_canister: request.source_canister,
        source_subaccount: request.source_subaccount,
        target_canister: request.target_canister,
        ledger_canister_id: context.ledger_canister_id,
        cmc_canister_id: context.cmc_canister_id,
        cmc_to_account_owner: cmc_account.owner,
        cmc_to_account_subaccount: cmc_account.subaccount,
        amount_e8s: request.amount_e8s,
        fee_e8s: context.fee_e8s,
        memo: IcpRefillOps::topup_memo(),
        created_at_time_ns: context.created_at_time_ns,
        now_ns: IcOps::now_nanos(),
    })?;

    advance_record(record, token, cost_permit).await
}

async fn transfer_record(
    record: IcpRefillRecord,
    token: &ReplayReceiptToken,
    cost_permit: &mut Option<CostGuardPermit>,
) -> Result<IcpRefillRecord, InternalError> {
    let to = IcpRefillOps::cmc_topup_account(record.cmc_canister_id, record.target_canister)?;
    let transfer_arg = IcpRefillOps::transfer_arg(
        record.source_subaccount,
        to,
        record.amount_e8s,
        record.fee_e8s,
        record.memo.clone(),
        record.created_at_time_ns,
    );

    reserve_icp_refill_cost_guard_if_needed(token, &record, cost_permit)?;
    let cost_permit = require_icp_refill_cost_permit(cost_permit.as_ref())?;
    mark_icp_refill_transfer_effect(token, &record);

    match IcpRefillOps::icrc1_transfer(cost_permit, record.ledger_canister_id, transfer_arg).await {
        Err(err) => {
            mark_icp_refill_recovery_required(token, &record, "ledger_transfer", &err);
            Err(err)
        }
        Ok(Ok(block_index)) => {
            let block_index = match IcpRefillOps::checked_block_index(block_index) {
                Ok(block_index) => block_index,
                Err(err) => {
                    return IcpRefillRecordOps::mark_transfer_failed(
                        record.id,
                        IcpRefillErrorCode::InvalidLedgerBlockIndex,
                        err.to_string(),
                        IcOps::now_nanos(),
                    );
                }
            };
            IcpRefillRecordOps::mark_transferred(record.id, block_index, IcOps::now_nanos())
        }
        Ok(Err(err)) => apply_transfer_error(record.id, err),
    }
}

async fn advance_record(
    record: IcpRefillRecord,
    token: &ReplayReceiptToken,
    cost_permit: &mut Option<CostGuardPermit>,
) -> Result<IcpRefillRecord, InternalError> {
    let record = match record.status {
        IcpRefillStatus::Requested => {
            transfer_unless_window_stale(record, token, cost_permit).await?
        }
        IcpRefillStatus::Transferred | IcpRefillStatus::NotifyProcessing => record,
        IcpRefillStatus::Failed if IcpRefillRecordOps::can_retry_notify(&record) => record,
        IcpRefillStatus::Failed if IcpRefillRecordOps::can_retry_bad_fee(&record) => {
            transfer_unless_window_stale(record, token, cost_permit).await?
        }
        IcpRefillStatus::Completed
        | IcpRefillStatus::Failed
        | IcpRefillStatus::InvalidTransaction
        | IcpRefillStatus::Refunded
        | IcpRefillStatus::TransactionTooOld => return Ok(record),
    };

    if IcpRefillRecordOps::should_notify(&record) {
        notify_record(record, token, cost_permit).await
    } else {
        Ok(record)
    }
}

async fn transfer_unless_window_stale(
    record: IcpRefillRecord,
    token: &ReplayReceiptToken,
    cost_permit: &mut Option<CostGuardPermit>,
) -> Result<IcpRefillRecord, InternalError> {
    let now_ns = IcOps::now_nanos();
    if IcpRefillRecordOps::transfer_window_stale(&record, now_ns, TX_WINDOW_NANOS) {
        IcpRefillRecordOps::mark_transfer_window_stale(record.id, now_ns)
    } else {
        transfer_record(record, token, cost_permit).await
    }
}

async fn notify_record(
    record: IcpRefillRecord,
    token: &ReplayReceiptToken,
    cost_permit: &mut Option<CostGuardPermit>,
) -> Result<IcpRefillRecord, InternalError> {
    let Some(block_index) = record.ledger_block_index else {
        return IcpRefillRecordOps::mark_notify_failed(
            record.id,
            "notify_top_up cannot run before ledger block is recorded".to_string(),
            IcOps::now_nanos(),
        );
    };

    let record = IcpRefillRecordOps::mark_notify_attempt_started(record.id, IcOps::now_nanos())?;
    let args = NotifyTopUpArg {
        block_index,
        canister_id: record.target_canister,
    };

    reserve_icp_refill_cost_guard_if_needed(token, &record, cost_permit)?;
    let cost_permit = require_icp_refill_cost_permit(cost_permit.as_ref())?;
    mark_icp_refill_notify_effect(token, &record);

    match IcpRefillOps::notify_top_up(cost_permit, record.cmc_canister_id, args).await {
        Ok(Ok(cycles_sent)) => {
            let record =
                IcpRefillRecordOps::mark_completed(record.id, cycles_sent, IcOps::now_nanos())?;
            record_direct_child_refill_grant(&record, IcOps::now_secs());
            Ok(record)
        }
        Ok(Err(err)) => apply_notify_error(record.id, record.notify_attempts, err),
        Err(err) => {
            mark_icp_refill_recovery_required(token, &record, "cmc_notify_top_up", &err);
            Err(err)
        }
    }
}

pub(super) fn apply_transfer_error(
    record_id: u64,
    err: TransferError,
) -> Result<IcpRefillRecord, InternalError> {
    match err {
        TransferError::BadFee { expected_fee } => {
            let expected_fee_e8s =
                match super::checked_nat_u64("bad_fee.expected_fee", expected_fee) {
                    Ok(expected_fee_e8s) => expected_fee_e8s,
                    Err(err) => {
                        return IcpRefillRecordOps::mark_transfer_failed(
                            record_id,
                            IcpRefillErrorCode::BadFee,
                            err.to_string(),
                            IcOps::now_nanos(),
                        );
                    }
                };
            IcpRefillRecordOps::mark_bad_fee(
                record_id,
                expected_fee_e8s,
                format!("bad fee; expected {expected_fee_e8s}"),
                IcOps::now_nanos(),
            )
        }
        TransferError::Duplicate { duplicate_of } => {
            let duplicate_of = match IcpRefillOps::checked_block_index(duplicate_of) {
                Ok(block_index) => block_index,
                Err(err) => {
                    return IcpRefillRecordOps::mark_transfer_failed(
                        record_id,
                        IcpRefillErrorCode::InvalidLedgerBlockIndex,
                        err.to_string(),
                        IcOps::now_nanos(),
                    );
                }
            };
            IcpRefillRecordOps::mark_duplicate_transferred(
                record_id,
                duplicate_of,
                IcOps::now_nanos(),
            )
        }
        TransferError::TooOld => {
            IcpRefillRecordOps::mark_transfer_window_stale(record_id, IcOps::now_nanos())
        }
        other => IcpRefillRecordOps::mark_transfer_failed(
            record_id,
            IcpRefillErrorCode::LedgerTransferFailed,
            other.to_string(),
            IcOps::now_nanos(),
        ),
    }
}

pub(super) fn apply_notify_error(
    record_id: u64,
    notify_attempts: u32,
    err: NotifyTopUpError,
) -> Result<IcpRefillRecord, InternalError> {
    match err {
        NotifyTopUpError::Refunded {
            block_index,
            reason,
        } => IcpRefillRecordOps::mark_refunded(record_id, block_index, reason, IcOps::now_nanos()),
        NotifyTopUpError::InvalidTransaction(reason) => {
            IcpRefillRecordOps::mark_invalid_transaction(record_id, reason, IcOps::now_nanos())
        }
        NotifyTopUpError::Processing => mark_notify_processing(record_id, notify_attempts),
        NotifyTopUpError::TransactionTooOld(min_block_index) => {
            IcpRefillRecordOps::mark_transaction_too_old(
                record_id,
                Some(min_block_index),
                IcOps::now_nanos(),
            )
        }
        NotifyTopUpError::Other {
            error_code,
            error_message,
        } => mark_retryable_notify_failure(
            record_id,
            notify_attempts,
            format!("notify_top_up error {error_code}: {error_message}"),
        ),
    }
}

pub(super) fn mark_notify_processing(
    record_id: u64,
    notify_attempts: u32,
) -> Result<IcpRefillRecord, InternalError> {
    if notify_attempts >= MAX_NOTIFY_ATTEMPTS {
        IcpRefillRecordOps::mark_notify_max_attempts(
            record_id,
            "notify_top_up returned Processing after max attempts".to_string(),
            IcOps::now_nanos(),
        )
    } else {
        IcpRefillRecordOps::mark_notify_processing(record_id, IcOps::now_nanos())
    }
}

pub(super) fn mark_retryable_notify_failure(
    record_id: u64,
    notify_attempts: u32,
    error_message: String,
) -> Result<IcpRefillRecord, InternalError> {
    if notify_attempts >= MAX_NOTIFY_ATTEMPTS {
        IcpRefillRecordOps::mark_notify_max_attempts(record_id, error_message, IcOps::now_nanos())
    } else {
        IcpRefillRecordOps::mark_notify_failed(record_id, error_message, IcOps::now_nanos())
    }
}

fn record_direct_child_refill_grant(record: &IcpRefillRecord, now_secs: u64) {
    let Some(cycles_sent) = record.cycles_sent.as_ref() else {
        return;
    };
    let Some((_child_role, parent_pid)) = CanisterChildrenOps::role_parent(record.target_canister)
    else {
        return;
    };
    let Some((child, cycles)) = direct_child_refill_grant(record, cycles_sent, parent_pid) else {
        return;
    };

    CyclesFundingLedgerOps::record_child_grant(child, cycles, now_secs);
}

pub(super) fn direct_child_refill_grant(
    record: &IcpRefillRecord,
    cycles_sent: &Nat,
    parent_pid: Option<Principal>,
) -> Option<(Principal, u128)> {
    if !direct_child_refill_parent_matches(parent_pid, record.source_canister) {
        return None;
    }

    Some((
        record.target_canister,
        IcpRefillRecordOps::nat_to_u128_saturating(cycles_sent),
    ))
}

pub(super) fn direct_child_refill_role(
    target_canister: Principal,
    source_canister: Principal,
) -> Option<CanisterRole> {
    let (role, parent_pid) = CanisterChildrenOps::role_parent(target_canister)?;
    if direct_child_refill_parent_matches(parent_pid, source_canister) {
        Some(role)
    } else {
        None
    }
}

fn direct_child_refill_parent_matches(
    parent_pid: Option<Principal>,
    source_canister: Principal,
) -> bool {
    parent_pid == Some(source_canister)
}
