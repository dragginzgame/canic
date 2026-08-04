//! Module: ops::ic::mgmt::cycles
//!
//! Responsibility: expose management-canister cycle balance and deposit calls.
//! Does not own: funding policy, cost guard admission, or cycle accounting records.
//! Boundary: `MgmtOps` extension for cycle-related management calls.

use super::*;
use crate::ops::cost_guard::CostGuardPermit;

impl MgmtOps {
    /// Return the exact current-Subnet execution cost of a zero-cycle deposit call.
    pub fn deposit_cycles_call_cost(canister_pid: Principal) -> Result<u128, InternalError> {
        MgmtInfra::deposit_cycles_call_cost(canister_pid).map_err(|err| OpsError::from(err).into())
    }

    /// Deposits cycles after a cost guard has reserved value-transfer quota and cycles.
    pub async fn deposit_cycles_with_permit(
        _permit: &CostGuardPermit,
        canister_pid: Principal,
        cycles: u128,
    ) -> Result<(), InternalError> {
        management_call(
            ManagementCallMetricOperation::DepositCycles,
            MgmtInfra::deposit_cycles(canister_pid, cycles),
        )
        .await?;

        SystemMetrics::increment(SystemMetricKind::DepositCycles);

        Ok(())
    }
    /// Gets a canister's cycle balance (expensive: calls mgmt canister).
    pub async fn get_cycles(canister_pid: Principal) -> Result<Cycles, InternalError> {
        let cycles = management_call(
            ManagementCallMetricOperation::GetCycles,
            MgmtInfra::get_cycles(canister_pid),
        )
        .await?;

        Ok(cycles)
    }
}
