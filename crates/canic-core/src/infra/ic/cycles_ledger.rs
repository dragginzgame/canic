//! Module: infra::ic::cycles_ledger
//!
//! Responsibility: perform raw IC-mainnet Cycles Ledger pool-refill calls.
//! Does not own: refill policy, durable retry state, or pool inventory mutation.
//! Boundary: ops invokes this adapter only after workflow has persisted exact effect authority.

use crate::{
    cdk::types::Cycles,
    infra::ic::{IcInfraError, call::Call, known::CYCLES_LEDGER_CANISTER},
};
use candid::{CandidType, Nat, Principal};
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error as ThisError;

/// Exact Cycles Ledger canister-creation request.
#[derive(CandidType)]
pub struct CyclesLedgerCreateCanisterArgs {
    pub from_subaccount: Option<[u8; 32]>,
    pub created_at_time: Option<u64>,
    pub amount: Nat,
    pub creation_args: Option<CyclesLedgerCmcCreateCanisterArgs>,
}

/// CMC arguments nested in a Cycles Ledger creation request.
#[derive(CandidType)]
pub struct CyclesLedgerCmcCreateCanisterArgs {
    pub settings: Option<CyclesLedgerCanisterSettings>,
    pub subnet_selection: Option<CyclesLedgerSubnetSelection>,
}

/// Settings applied by the CMC while creating the empty pool Canister.
#[derive(CandidType)]
pub struct CyclesLedgerCanisterSettings {
    pub controllers: Option<Vec<Principal>>,
    pub compute_allocation: Option<Nat>,
    pub memory_allocation: Option<Nat>,
    pub freezing_threshold: Option<Nat>,
    pub reserved_cycles_limit: Option<Nat>,
}

/// Exact physical Subnet selection used for one pool refill.
#[derive(CandidType)]
pub enum CyclesLedgerSubnetSelection {
    Subnet { subnet: Principal },
}

/// Successful Cycles Ledger creation evidence.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CyclesLedgerCreateCanisterSuccess {
    pub block_id: Nat,
    pub canister_id: Principal,
}

/// Lossless Cycles Ledger create-canister error surface.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CyclesLedgerCreateCanisterError {
    InsufficientFunds {
        balance: Nat,
    },
    TooOld,
    CreatedInFuture {
        ledger_time: u64,
    },
    TemporarilyUnavailable,
    Duplicate {
        duplicate_of: Nat,
        canister_id: Option<Principal>,
    },
    FailedToCreate {
        fee_block: Option<Nat>,
        refund_block: Option<Nat>,
        error: String,
    },
    GenericError {
        message: String,
        error_code: Nat,
    },
}

impl fmt::Display for CyclesLedgerCreateCanisterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InsufficientFunds { balance } => {
                write!(
                    formatter,
                    "Cycles Ledger balance is insufficient: {balance}"
                )
            }
            Self::TooOld => formatter.write_str("creation request is older than the ledger window"),
            Self::CreatedInFuture { ledger_time } => {
                write!(
                    formatter,
                    "creation timestamp is ahead of ledger time {ledger_time}"
                )
            }
            Self::TemporarilyUnavailable => {
                formatter.write_str("Cycles Ledger is temporarily unavailable")
            }
            Self::Duplicate {
                duplicate_of,
                canister_id,
            } => write!(
                formatter,
                "creation duplicates block {duplicate_of} with canister {canister_id:?}"
            ),
            Self::FailedToCreate { error, .. } => write!(formatter, "CMC creation failed: {error}"),
            Self::GenericError {
                message,
                error_code,
            } => write!(formatter, "Cycles Ledger error {error_code}: {message}"),
        }
    }
}

/// Typed local conversion failures around the Cycles Ledger boundary.
#[derive(Debug, ThisError)]
pub enum CyclesLedgerInfraError {
    #[error("Cycles Ledger value {value} exceeds the Canic cycles range")]
    CyclesOverflow { value: Nat },

    #[error("Cycles Ledger block index {value} exceeds u64")]
    BlockIndexOverflow { value: Nat },
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
        root: Principal,
        subnet: Principal,
        amount: Cycles,
        created_at_time: u64,
    ) -> Result<
        Result<CyclesLedgerCreateCanisterSuccess, CyclesLedgerCreateCanisterError>,
        IcInfraError,
    > {
        Call::unbounded_wait(*CYCLES_LEDGER_CANISTER, "create_canister")
            .with_arg(CyclesLedgerCreateCanisterArgs {
                from_subaccount: None,
                created_at_time: Some(created_at_time),
                amount: Nat::from(amount.to_u128()),
                creation_args: Some(CyclesLedgerCmcCreateCanisterArgs {
                    settings: Some(CyclesLedgerCanisterSettings {
                        controllers: Some(vec![root]),
                        compute_allocation: None,
                        memory_allocation: None,
                        freezing_threshold: None,
                        reserved_cycles_limit: None,
                    }),
                    subnet_selection: Some(CyclesLedgerSubnetSelection::Subnet { subnet }),
                }),
            })?
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
