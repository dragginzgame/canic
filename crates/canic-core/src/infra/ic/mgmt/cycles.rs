//! Module: infra::ic::mgmt::cycles
//!
//! Responsibility: perform raw management canister cycle operations.
//! Does not own: funding policy, threshold decisions, or workflow retries.
//! Boundary: extends `MgmtInfra` with cycle-related management calls.

use crate::{
    cdk::{self, candid::Principal, types::Cycles},
    infra::{InfraError, ic::call::Call},
};

use super::{MgmtInfra, types::InfraCanisterIdRecord};

impl MgmtInfra {
    /// Return the local canister's cycle balance.
    #[must_use]
    pub fn canister_cycle_balance() -> Cycles {
        cdk::api::canister_cycle_balance().into()
    }

    /// Deposit cycles into a canister through the management canister.
    pub async fn deposit_cycles(canister_pid: Principal, cycles: u128) -> Result<(), InfraError> {
        let args = InfraCanisterIdRecord {
            canister_id: canister_pid,
        };
        Call::bounded_wait(Principal::management_canister(), "deposit_cycles")
            .with_arg(args)?
            .with_cycles(cycles)
            .execute()
            .await?;

        Ok(())
    }

    /// Get a canister's cycle balance by querying canister status.
    pub async fn get_cycles(canister_pid: Principal) -> Result<Cycles, InfraError> {
        let status = Self::canister_status(canister_pid).await?;
        Ok(status.cycles.into())
    }
}
