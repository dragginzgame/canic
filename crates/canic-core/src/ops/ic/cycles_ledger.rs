//! Module: ops::ic::cycles_ledger
//!
//! Responsibility: expose approved Cycles Ledger pool-refill calls.
//! Does not own: refill policy, stable creation journals, or inventory mutation.
//! Boundary: effect calls require a workflow-owned cost-guard permit.

use crate::{
    InternalError,
    cdk::types::{Cycles, Principal},
    infra::ic::{
        IcInfraError,
        cycles_ledger::{
            CyclesLedgerCreateCanisterError, CyclesLedgerCreateCanisterSuccess, CyclesLedgerInfra,
        },
    },
    ops::{OpsError, cost_guard::CostGuardPermit},
};
use candid::Nat;

/// Operations facade for the IC-mainnet Cycles Ledger.
pub struct CyclesLedgerOps;

impl CyclesLedgerOps {
    #[must_use]
    pub fn canister_id() -> Principal {
        CyclesLedgerInfra::canister_id()
    }

    pub async fn create_canister(
        _permit: &CostGuardPermit,
        root: Principal,
        subnet: Principal,
        amount: Cycles,
        created_at_time: u64,
    ) -> Result<
        Result<CyclesLedgerCreateCanisterSuccess, CyclesLedgerCreateCanisterError>,
        InternalError,
    > {
        map_infra(CyclesLedgerInfra::create_canister(root, subnet, amount, created_at_time).await)
    }

    pub async fn balance_of(root: Principal) -> Result<Cycles, InternalError> {
        let balance = map_infra(CyclesLedgerInfra::balance_of(root).await)?;
        Self::checked_cycles(balance)
    }

    pub async fn fee() -> Result<Cycles, InternalError> {
        let fee = map_infra(CyclesLedgerInfra::fee().await)?;
        Self::checked_cycles(fee)
    }

    pub fn checked_block_index(value: Nat) -> Result<u64, InternalError> {
        map_infra(CyclesLedgerInfra::checked_block_index(value))
    }

    pub fn checked_cycles(value: Nat) -> Result<Cycles, InternalError> {
        map_infra(CyclesLedgerInfra::checked_cycles(value))
    }
}

fn map_infra<T>(result: Result<T, IcInfraError>) -> Result<T, InternalError> {
    result.map_err(OpsError::from).map_err(InternalError::from)
}
