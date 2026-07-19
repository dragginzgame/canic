//! Module: infra::ic::mgmt::cycles
//!
//! Responsibility: perform raw management canister cycle operations.
//! Does not own: funding policy, threshold decisions, or workflow retries.
//! Boundary: extends `MgmtInfra` with cycle-related management calls.

use crate::{
    cdk::{
        self,
        candid::{Nat, Principal},
        types::Cycles,
    },
    infra::{InfraError, ic::call::Call},
};

use super::{MgmtInfra, MgmtInfraError, types::InfraCanisterIdRecord};

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
        checked_canister_cycles(canister_pid, status.cycles)
            .map_err(|err| InfraError::from(crate::infra::ic::IcInfraError::from(err)))
    }
}

fn checked_canister_cycles(canister_pid: Principal, value: Nat) -> Result<Cycles, MgmtInfraError> {
    Cycles::try_from(value.clone()).map_err(|_| MgmtInfraError::CanisterCyclesOverflow {
        canister_pid,
        value,
    })
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_canister_cycles_preserves_overflow_identity_and_value() {
        let canister_pid = Principal::from_slice(&[7]);
        let value = Nat::parse(b"340282366920938463463374607431768211456")
            .expect("u128 max plus one is valid Nat");

        let err = checked_canister_cycles(canister_pid, value.clone())
            .expect_err("oversized status balance must fail closed");

        assert!(matches!(
            err,
            MgmtInfraError::CanisterCyclesOverflow {
                canister_pid: actual_canister_pid,
                value: actual_value,
            } if actual_canister_pid == canister_pid && actual_value == value
        ));
    }
}
