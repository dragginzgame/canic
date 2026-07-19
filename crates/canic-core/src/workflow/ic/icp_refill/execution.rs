//! Module: workflow::ic::icp_refill::execution
//!
//! Responsibility: execute ICP ledger transfers and CMC top-up notifications.
//! Does not own: endpoint authorization, stable record schemas, or pure policy.
//! Boundary: orchestrates ops/storage/replay after request preflight.

use crate::{
    InternalError,
    cdk::{candid::Nat, types::Principal},
    domain::{
        icp_refill::{IcpRefillErrorCode, IcpRefillStatus},
        policy::pure::icp_refill::IcpRefillPolicyViolation,
    },
    dto::icp_refill::{IcpRefillRequest, IcpRefillResponse},
    ids::CanisterRole,
    infra::ic::icp_refill::{NotifyTopUpArg, NotifyTopUpError, TransferError},
    ops::{
        cost_guard::CostGuardPermit,
        ic::{IcOps, icp_refill::IcpRefillOps},
        replay::receipt::{ReplayReceiptToken, validate_receipt_token},
        runtime::cycles_funding::CyclesFundingLedgerOps,
        storage::{
            children::CanisterChildrenOps,
            icp_refill::{
                IcpRefillOperationCreateInput, IcpRefillRecordOpsError, IcpRefillStoreOps,
            },
        },
    },
    view::icp_refill::IcpRefillOperation,
    workflow::ic::icp_refill::{
        MAX_NOTIFY_ATTEMPTS, RateQueryMode, TX_WINDOW_NANOS,
        cost_guard::{
            recover_icp_refill_cost_guard, require_icp_refill_cost_permit,
            reserve_icp_refill_cost_guard_if_needed,
        },
        policy_denied, prepare_context,
        replay::{
            finish_icp_refill_replay, map_icp_refill_replay_store_error,
            mark_icp_refill_notify_effect, mark_icp_refill_transfer_effect,
            preserve_icp_refill_recovery_required,
        },
    },
    workflow::replay::abort_reserved_receipt_after_failure,
};

pub(super) async fn execute_fresh_manual_refill(
    request: IcpRefillRequest,
    operation_id: [u8; 32],
    token: &ReplayReceiptToken,
) -> Result<IcpRefillResponse, InternalError> {
    let mut cost_permit = None;
    let operation =
        match execute_manual_refill_operation(request, operation_id, token, &mut cost_permit).await
        {
            Ok(operation) => operation,
            Err(err) => {
                if let Err(recovery_error) = recover_icp_refill_cost_guard(cost_permit.as_ref()) {
                    return Err(err.with_diagnostic_context(format!(
                        "ICP refill cost guard recovery failed: {recovery_error}"
                    )));
                }
                return Err(abort_reserved_receipt_after_failure(
                    token,
                    err,
                    "ICP refill replay reservation cleanup failed",
                ));
            }
        };
    let response = IcpRefillStoreOps::to_response(&operation);

    if let Err(err) = finish_icp_refill_replay(token, &operation, &response, cost_permit.as_ref()) {
        return Err(abort_reserved_receipt_after_failure(
            token,
            err,
            "ICP refill replay reservation cleanup failed",
        ));
    }

    Ok(response)
}

async fn execute_manual_refill_operation(
    request: IcpRefillRequest,
    operation_id: [u8; 32],
    token: &ReplayReceiptToken,
    cost_permit: &mut Option<CostGuardPermit>,
) -> Result<IcpRefillOperation, InternalError> {
    if let Some(operation) = IcpRefillStoreOps::find_by_operation_id(operation_id)? {
        IcpRefillStoreOps::validate_retry_request_matches_operation(&request, &operation)?;
        return advance_operation(operation, token, cost_permit).await;
    }

    let context = prepare_context(&request, RateQueryMode::WhenGateConfigured).await?;
    validate_receipt_token(token).map_err(map_icp_refill_replay_store_error)?;
    let cmc_account =
        IcpRefillOps::cmc_topup_account(context.cmc_canister_id, request.target_canister)?;
    let operation = create_or_get_operation(IcpRefillOperationCreateInput {
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

    advance_operation(operation, token, cost_permit).await
}

fn create_or_get_operation(
    input: IcpRefillOperationCreateInput,
) -> Result<IcpRefillOperation, InternalError> {
    match IcpRefillStoreOps::create_or_get(input) {
        Ok(operation) => Ok(operation),
        Err(IcpRefillRecordOpsError::ConcurrentOperation { .. }) => {
            Err(policy_denied(IcpRefillPolicyViolation::ConcurrentRefill))
        }
        Err(err) => Err(err.into()),
    }
}

async fn transfer_operation(
    operation: IcpRefillOperation,
    token: &ReplayReceiptToken,
    cost_permit: &mut Option<CostGuardPermit>,
) -> Result<IcpRefillOperation, InternalError> {
    let to = IcpRefillOps::cmc_topup_account(operation.cmc_canister_id, operation.target_canister)?;
    let transfer_arg = IcpRefillOps::transfer_arg(
        operation.source_subaccount,
        to,
        operation.amount_e8s,
        operation.fee_e8s,
        operation.memo.clone(),
        operation.created_at_time_ns,
    );

    reserve_icp_refill_cost_guard_if_needed(token, &operation, cost_permit)?;
    let cost_permit = require_icp_refill_cost_permit(cost_permit.as_ref())?;
    mark_icp_refill_transfer_effect(token, &operation)?;

    match IcpRefillOps::icrc1_transfer(cost_permit, operation.ledger_canister_id, transfer_arg)
        .await
    {
        Err(err) => Err(preserve_icp_refill_recovery_required(
            token,
            &operation,
            "ledger_transfer",
            err,
        )),
        Ok(Ok(block_index)) => {
            let block_index = match IcpRefillOps::checked_block_index(block_index) {
                Ok(block_index) => block_index,
                Err(err) => {
                    return IcpRefillStoreOps::mark_transfer_failed(
                        operation.id,
                        IcpRefillErrorCode::InvalidLedgerBlockIndex,
                        err.to_string(),
                        IcOps::now_nanos(),
                    );
                }
            };
            IcpRefillStoreOps::mark_transferred(operation.id, block_index, IcOps::now_nanos())
        }
        Ok(Err(err)) => apply_transfer_error(operation.id, err),
    }
}

async fn advance_operation(
    operation: IcpRefillOperation,
    token: &ReplayReceiptToken,
    cost_permit: &mut Option<CostGuardPermit>,
) -> Result<IcpRefillOperation, InternalError> {
    let operation = match operation.status {
        IcpRefillStatus::Requested => {
            transfer_unless_window_stale(operation, token, cost_permit).await?
        }
        IcpRefillStatus::Transferred | IcpRefillStatus::NotifyProcessing => operation,
        IcpRefillStatus::Failed if IcpRefillStoreOps::can_retry_notify(&operation) => operation,
        IcpRefillStatus::Failed if IcpRefillStoreOps::can_retry_bad_fee(&operation) => {
            transfer_unless_window_stale(operation, token, cost_permit).await?
        }
        IcpRefillStatus::Completed
        | IcpRefillStatus::Failed
        | IcpRefillStatus::InvalidTransaction
        | IcpRefillStatus::Refunded
        | IcpRefillStatus::TransactionTooOld => return Ok(operation),
    };

    if IcpRefillStoreOps::should_notify(&operation) {
        notify_operation(operation, token, cost_permit).await
    } else {
        Ok(operation)
    }
}

async fn transfer_unless_window_stale(
    operation: IcpRefillOperation,
    token: &ReplayReceiptToken,
    cost_permit: &mut Option<CostGuardPermit>,
) -> Result<IcpRefillOperation, InternalError> {
    let now_ns = IcOps::now_nanos();
    if IcpRefillStoreOps::transfer_window_stale(&operation, now_ns, TX_WINDOW_NANOS) {
        IcpRefillStoreOps::mark_transfer_window_stale(operation.id, now_ns)
    } else {
        transfer_operation(operation, token, cost_permit).await
    }
}

async fn notify_operation(
    operation: IcpRefillOperation,
    token: &ReplayReceiptToken,
    cost_permit: &mut Option<CostGuardPermit>,
) -> Result<IcpRefillOperation, InternalError> {
    let Some(block_index) = operation.ledger_block_index else {
        return IcpRefillStoreOps::mark_notify_failed(
            operation.id,
            "notify_top_up cannot run before ledger block is recorded".to_string(),
            IcOps::now_nanos(),
        );
    };

    let operation =
        IcpRefillStoreOps::mark_notify_attempt_started(operation.id, IcOps::now_nanos())?;
    let args = NotifyTopUpArg {
        block_index,
        canister_id: operation.target_canister,
    };

    reserve_icp_refill_cost_guard_if_needed(token, &operation, cost_permit)?;
    let cost_permit = require_icp_refill_cost_permit(cost_permit.as_ref())?;
    mark_icp_refill_notify_effect(token, &operation)?;

    match IcpRefillOps::notify_top_up(cost_permit, operation.cmc_canister_id, args).await {
        Ok(Ok(cycles_sent)) => {
            let (operation, cycles_sent) =
                apply_notify_success(operation.id, cycles_sent, IcOps::now_nanos())?;
            if let Some(cycles_sent) = cycles_sent {
                record_direct_child_refill_grant(&operation, cycles_sent, IcOps::now_secs());
            }
            Ok(operation)
        }
        Ok(Err(err)) => apply_notify_error(operation.id, operation.notify_attempts, err),
        Err(err) => Err(preserve_icp_refill_recovery_required(
            token,
            &operation,
            "cmc_notify_top_up",
            err,
        )),
    }
}

pub(super) fn apply_notify_success(
    record_id: u64,
    cycles_sent: Nat,
    now_ns: u64,
) -> Result<(IcpRefillOperation, Option<u128>), InternalError> {
    IcpRefillStoreOps::complete_from_notified_cycles(record_id, cycles_sent, now_ns)
}

pub(super) fn apply_transfer_error(
    record_id: u64,
    err: TransferError,
) -> Result<IcpRefillOperation, InternalError> {
    match err {
        TransferError::BadFee { expected_fee } => {
            let expected_fee_e8s = match crate::workflow::ic::icp_refill::checked_nat_u64(
                "bad_fee.expected_fee",
                expected_fee,
            ) {
                Ok(expected_fee_e8s) => expected_fee_e8s,
                Err(err) => {
                    return IcpRefillStoreOps::mark_transfer_failed(
                        record_id,
                        IcpRefillErrorCode::BadFee,
                        err.to_string(),
                        IcOps::now_nanos(),
                    );
                }
            };
            IcpRefillStoreOps::mark_bad_fee(
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
                    return IcpRefillStoreOps::mark_transfer_failed(
                        record_id,
                        IcpRefillErrorCode::InvalidLedgerBlockIndex,
                        err.to_string(),
                        IcOps::now_nanos(),
                    );
                }
            };
            IcpRefillStoreOps::mark_duplicate_transferred(
                record_id,
                duplicate_of,
                IcOps::now_nanos(),
            )
        }
        TransferError::TooOld => {
            IcpRefillStoreOps::mark_transfer_window_stale(record_id, IcOps::now_nanos())
        }
        other => IcpRefillStoreOps::mark_transfer_failed(
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
) -> Result<IcpRefillOperation, InternalError> {
    match err {
        NotifyTopUpError::Refunded {
            block_index,
            reason,
        } => IcpRefillStoreOps::mark_refunded(record_id, block_index, reason, IcOps::now_nanos()),
        NotifyTopUpError::InvalidTransaction(reason) => {
            IcpRefillStoreOps::mark_invalid_transaction(record_id, reason, IcOps::now_nanos())
        }
        NotifyTopUpError::Processing => mark_notify_processing(record_id, notify_attempts),
        NotifyTopUpError::TransactionTooOld(min_block_index) => {
            IcpRefillStoreOps::mark_transaction_too_old(
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
) -> Result<IcpRefillOperation, InternalError> {
    if notify_attempts >= MAX_NOTIFY_ATTEMPTS {
        IcpRefillStoreOps::mark_notify_max_attempts(
            record_id,
            "notify_top_up returned Processing after max attempts".to_string(),
            IcOps::now_nanos(),
        )
    } else {
        IcpRefillStoreOps::mark_notify_processing(record_id, IcOps::now_nanos())
    }
}

pub(super) fn mark_retryable_notify_failure(
    record_id: u64,
    notify_attempts: u32,
    error_message: String,
) -> Result<IcpRefillOperation, InternalError> {
    if notify_attempts >= MAX_NOTIFY_ATTEMPTS {
        IcpRefillStoreOps::mark_notify_max_attempts(record_id, error_message, IcOps::now_nanos())
    } else {
        IcpRefillStoreOps::mark_notify_failed(record_id, error_message, IcOps::now_nanos())
    }
}

fn record_direct_child_refill_grant(
    operation: &IcpRefillOperation,
    cycles_sent: u128,
    now_secs: u64,
) {
    let Some((_child_role, parent_pid)) =
        CanisterChildrenOps::role_parent(operation.target_canister)
    else {
        return;
    };
    let Some((child, cycles)) = direct_child_refill_grant(operation, cycles_sent, parent_pid)
    else {
        return;
    };

    CyclesFundingLedgerOps::record_child_grant(child, cycles, now_secs);
}

pub(super) fn direct_child_refill_grant(
    operation: &IcpRefillOperation,
    cycles_sent: u128,
    parent_pid: Option<Principal>,
) -> Option<(Principal, u128)> {
    if !direct_child_refill_parent_matches(parent_pid, operation.source_canister) {
        return None;
    }

    Some((operation.target_canister, cycles_sent))
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
