use super::*;

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
