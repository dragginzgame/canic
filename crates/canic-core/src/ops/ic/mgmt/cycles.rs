//! Module: ops::ic::mgmt::cycles
//!
//! Responsibility: expose management-canister cycle balance and deposit calls.
//! Does not own: funding policy, cost guard admission, or cycle accounting records.
//! Boundary: `MgmtOps` extension for cycle-related management calls.

use super::*;
use crate::ops::cost_guard::CostGuardPermit;

impl MgmtOps {
    /// Returns the local canister's cycle balance (cheap).
    #[must_use]
    pub fn canister_cycle_balance() -> Cycles {
        MgmtInfra::canister_cycle_balance()
    }

    /// Deposits cycles into a canister and records metrics.
    pub async fn deposit_cycles(
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

    /// Deposits cycles after a cost guard has reserved value-transfer quota and cycles.
    pub async fn deposit_cycles_with_permit(
        _permit: &CostGuardPermit,
        canister_pid: Principal,
        cycles: u128,
    ) -> Result<(), InternalError> {
        Self::deposit_cycles(canister_pid, cycles).await
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
