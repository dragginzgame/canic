use super::replay::icp_refill_command_kind;
use crate::{
    InternalError, InternalErrorOrigin,
    ops::{
        cost_guard::{CostGuardOps, CostGuardPermit, CostGuardRequest},
        ic::{IcOps, mgmt::MgmtOps},
        replay::receipt::ReplayReceiptToken,
    },
    replay_policy::CostClass,
    storage::stable::icp_refill::IcpRefillRecord,
    workflow::ic::icp_refill::replay::operation_id_display,
    workflow::prelude::*,
};

pub(super) const ICP_REFILL_VALUE_TRANSFER_QUOTA_WINDOW_SECONDS: u64 = 60;
pub(super) const MAX_ICP_REFILL_VALUE_TRANSFER_OPERATIONS_PER_WINDOW: u64 = 60;
pub(super) const ICP_REFILL_VALUE_TRANSFER_CYCLE_RESERVATION_CYCLES: u128 = 1_000_000_000;
pub(super) const MIN_ICP_REFILL_CYCLES_AFTER_RESERVATION: u128 = 1_000_000_000;

pub(super) fn reserve_icp_refill_cost_guard_if_needed(
    token: &ReplayReceiptToken,
    record: &IcpRefillRecord,
    cost_permit: &mut Option<CostGuardPermit>,
) -> Result<(), InternalError> {
    if cost_permit.is_some() {
        return Ok(());
    }

    let permit = CostGuardOps::reserve(icp_refill_cost_guard_request(
        token,
        IcOps::canister_self(),
        MgmtOps::canister_cycle_balance().to_u128(),
        IcOps::now_secs(),
    ))?;
    log_icp_refill_cost_guard_reserved(record);
    *cost_permit = Some(permit);
    Ok(())
}

pub(super) fn require_icp_refill_cost_permit(
    cost_permit: Option<&CostGuardPermit>,
) -> Result<&CostGuardPermit, InternalError> {
    cost_permit.ok_or_else(|| {
        InternalError::invariant(
            InternalErrorOrigin::Workflow,
            "ICP refill external effect crossed without value-transfer cost permit",
        )
    })
}

pub(super) fn icp_refill_cost_guard_request(
    token: &ReplayReceiptToken,
    payer: Principal,
    current_cycle_balance: u128,
    now_secs: u64,
) -> CostGuardRequest {
    CostGuardRequest {
        cost_class: CostClass::ValueTransfer,
        command_kind: icp_refill_command_kind(),
        quota_subject: token.receipt().actor.effective_principal,
        payer,
        now_secs,
        quota_window_secs: ICP_REFILL_VALUE_TRANSFER_QUOTA_WINDOW_SECONDS,
        max_operations_per_window: MAX_ICP_REFILL_VALUE_TRANSFER_OPERATIONS_PER_WINDOW,
        current_cycle_balance,
        cycle_reservation_cycles: ICP_REFILL_VALUE_TRANSFER_CYCLE_RESERVATION_CYCLES,
        min_cycles_after_reservation: MIN_ICP_REFILL_CYCLES_AFTER_RESERVATION,
    }
}

pub(super) fn complete_icp_refill_cost_guard(cost_permit: Option<&CostGuardPermit>) {
    let Some(cost_permit) = cost_permit else {
        return;
    };
    if let Err(err) = CostGuardOps::complete(cost_permit, IcOps::now_secs()) {
        crate::log!(
            crate::log::Topic::Cycles,
            Error,
            "icp refill value-transfer cost guard completion failed reservation_id={}: {}",
            cost_permit.reservation_id,
            err
        );
    }
}

pub(super) fn recover_icp_refill_cost_guard(cost_permit: Option<&CostGuardPermit>) {
    let Some(cost_permit) = cost_permit else {
        return;
    };
    if let Err(err) = CostGuardOps::recover(cost_permit, IcOps::now_secs()) {
        crate::log!(
            crate::log::Topic::Cycles,
            Error,
            "icp refill value-transfer cost guard recovery failed reservation_id={}: {}",
            cost_permit.reservation_id,
            err
        );
    }
}

fn log_icp_refill_cost_guard_reserved(record: &IcpRefillRecord) {
    crate::log!(
        crate::log::Topic::Cycles,
        Info,
        "icp refill value-transfer cost guard reserved command_kind={} operation_id={} record_id={} source={} target={} amount_e8s={}",
        super::ICP_REFILL_REPLAY_COMMAND_KIND,
        operation_id_display(record.operation_id),
        record.id,
        record.source_canister,
        record.target_canister,
        record.amount_e8s
    );
}
