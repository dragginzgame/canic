//! Shared bounded reservation for Root and Store deletion-cycle transfers.

use canic_core::{
    cdk::types::Principal,
    control_plane_support::{
        error::InternalError,
        model::replay::CommandKind,
        ops::{
            cost_guard::{CostGuardPermit, CostGuardRequest},
            ic::IcOps,
        },
        workflow::cost_guard::{CostGuardWorkflow, map_cost_guard_reserve_error},
    },
    replay_policy::CostClass,
};

pub(super) fn reserve(
    command_kind: CommandKind,
    quota_subject: Principal,
    maximum_transfer: u128,
    retained_cycles: u128,
    current_cycle_balance: u128,
) -> Result<CostGuardPermit, InternalError> {
    CostGuardWorkflow::reserve(CostGuardRequest {
        cost_class: CostClass::ValueTransfer,
        command_kind,
        quota_subject,
        payer: IcOps::canister_self(),
        now_secs: IcOps::now_secs(),
        quota_window_secs: 60,
        max_operations_per_window: 60,
        current_cycle_balance,
        cycle_reservation_cycles: maximum_transfer,
        min_cycles_after_reservation: retained_cycles,
    })
    .map_err(map_cost_guard_reserve_error)
}
