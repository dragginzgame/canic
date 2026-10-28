//! Module: infra::ic::cycles_ledger
//!
//! Responsibility: perform raw IC-mainnet Cycles Ledger pool-refill calls.
//! Does not own: refill policy, durable retry state, or pool inventory mutation.
//! Boundary: ops invokes this adapter only after workflow has persisted exact effect authority.

use crate::{
    dto::fleet_registry::FleetLedgerTransferIntent,
    infra::ic::{IcInfraError, call::Call, known::CYCLES_LEDGER_CANISTER},
};
use canic_contracts::cycles::Cycles;
use ic_platform::{
    candid::contract::Nat,
    cycles_ledger_types::{self as ledger, CreateCanisterError, CreateCanisterSuccess},
    icrc_ledger_types::TransferArg,
    principal::ExternalPrincipal as Principal,
};
use thiserror::Error as ThisError;

pub use ic_platform::icrc_ledger_types::{Account, TransferError};

/// Typed local conversion failures around the Cycles Ledger boundary.
#[derive(Debug, ThisError)]
pub enum CyclesLedgerInfraError {
    #[error("Cycles Ledger value {value} exceeds the Canic cycles range")]
    CyclesOverflow { value: Nat },

    #[error("Cycles Ledger block index {value} exceeds u64")]
    BlockIndexOverflow { value: Nat },

    #[error("retirement Ledger balance {balance} cannot cover transfer fee {fee}")]
    InvalidTransfer { balance: u128, fee: u128 },
}

/// Raw Cycles Ledger adapter.
pub struct CyclesLedgerInfra;

impl CyclesLedgerInfra {
    /// Return the canonical IC-mainnet Cycles Ledger principal.
    #[must_use]
    pub fn canister_id() -> Principal {
        *CYCLES_LEDGER_CANISTER
    }

    /// Ask the Cycles Ledger to create one root-controlled Canister on an exact Subnet.
    pub async fn create_canister(
        controllers: Vec<Principal>,
        subnet: Principal,
        amount: Cycles,
        created_at_time: u64,
    ) -> Result<Result<CreateCanisterSuccess, CreateCanisterError>, IcInfraError> {
        Call::unbounded_wait(*CYCLES_LEDGER_CANISTER, "create_canister")
            .with_arg(ledger::CreateCanisterArgs {
                from_subaccount: None,
                created_at_time: Some(created_at_time),
                amount: Nat::from(amount.to_u128()),
                creation_args: Some(ledger::CmcCreateCanisterArgs {
                    settings: Some(ledger::CanisterSettings {
                        controllers: Some(controllers),
                        compute_allocation: None,
                        memory_allocation: None,
                        freezing_threshold: None,
                        reserved_cycles_limit: None,
                        wasm_memory_limit: None,
                        wasm_memory_threshold: None,
                        log_visibility: None,
                        environment_variables: None,
                    }),
                    subnet_selection: Some(ledger::SubnetSelection::Subnet { subnet }),
                }),
            })?
            .execute()
            .await?
            .candid()
    }

    /// Execute the exact retirement transfer whose intent the caller has persisted.
    pub async fn transfer(
        intent: &FleetLedgerTransferIntent,
    ) -> Result<Result<Nat, TransferError>, IcInfraError> {
        let amount = intent
            .balance_before
            .checked_sub(intent.fee)
            .filter(|amount| *amount > 0)
            .ok_or_else(|| {
                IcInfraError::from(CyclesLedgerInfraError::InvalidTransfer {
                    balance: intent.balance_before,
                    fee: intent.fee,
                })
            })?;
        Call::unbounded_wait(*CYCLES_LEDGER_CANISTER, "icrc1_transfer")
            .with_arg(TransferArg {
                amount: amount.into(),
                created_at_time: Some(intent.created_at_time),
                fee: Some(intent.fee.into()),
                from_subaccount: None,
                memo: Some(intent.memo.to_vec().into()),
                to: Account {
                    owner: intent.destination,
                    subaccount: None,
                },
            })?
            .execute()
            .await?
            .candid()
    }

    /// Observe the exact Root-owned funding account before a paid creation effect.
    pub async fn balance_of(root: Principal) -> Result<Nat, IcInfraError> {
        Call::unbounded_wait(*CYCLES_LEDGER_CANISTER, "icrc1_balance_of")
            .with_arg(Account {
                owner: root,
                subaccount: None,
            })?
            .execute()
            .await?
            .candid()
    }

    /// Observe the current Ledger fee charged in addition to a create-canister amount.
    pub async fn fee() -> Result<Nat, IcInfraError> {
        Call::unbounded_wait(*CYCLES_LEDGER_CANISTER, "icrc1_fee")
            .with_arg(())?
            .execute()
            .await?
            .candid()
    }

    /// Convert a ledger block index into the bounded stable representation.
    pub fn checked_block_index(value: Nat) -> Result<u64, IcInfraError> {
        u64::try_from(value.0.clone())
            .map_err(|_| CyclesLedgerInfraError::BlockIndexOverflow { value }.into())
    }

    /// Convert a ledger balance into the bounded Canic cycles representation.
    pub fn checked_cycles(value: Nat) -> Result<Cycles, IcInfraError> {
        u128::try_from(value.0.clone())
            .map(Cycles::new)
            .map_err(|_| CyclesLedgerInfraError::CyclesOverflow { value }.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn canonical_cycles_ledger_principal_is_frozen() {
        assert_eq!(
            CyclesLedgerInfra::canister_id().to_text(),
            "um5iw-rqaaa-aaaaq-qaaba-cai"
        );
    }

    #[test]
    fn bounded_ledger_values_reject_overflow() {
        let too_large =
            Nat::from_str("340282366920938463463374607431768211456").expect("valid u128 plus one");
        assert!(CyclesLedgerInfra::checked_cycles(too_large).is_err());

        let too_large = Nat::from_str("18446744073709551616").expect("valid u64 plus one");
        assert!(CyclesLedgerInfra::checked_block_index(too_large).is_err());
    }
}
