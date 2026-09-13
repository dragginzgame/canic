//! Shared exact Cycles Ledger creation contracts for maintained host transports.

use candid::{CandidType, Nat, Principal};
use serde::Deserialize;

#[derive(CandidType)]
pub struct CreateCanisterArgs {
    pub amount: Nat,
    pub created_at_time: Option<u64>,
    pub creation_args: Option<CmcCreateCanisterArgs>,
    pub from_subaccount: Option<[u8; 32]>,
}

#[derive(CandidType)]
pub struct CmcCreateCanisterArgs {
    pub settings: Option<CanisterSettings>,
    pub subnet_selection: Option<SubnetSelection>,
}

#[derive(CandidType)]
pub struct CanisterSettings {
    pub compute_allocation: Option<Nat>,
    pub controllers: Option<Vec<Principal>>,
    pub freezing_threshold: Option<Nat>,
    pub memory_allocation: Option<Nat>,
    pub reserved_cycles_limit: Option<Nat>,
}

#[derive(CandidType)]
pub enum SubnetSelection {
    Subnet { subnet: Principal },
}

#[derive(CandidType, Deserialize)]
pub struct CreateCanisterSuccess {
    pub block_id: Nat,
    pub canister_id: Principal,
}

#[derive(CandidType, Debug, Deserialize)]
pub enum CreateCanisterError {
    CreatedInFuture {
        ledger_time: u64,
    },
    Duplicate {
        duplicate_of: Nat,
        canister_id: Option<Principal>,
    },
    FailedToCreate {
        error: String,
        fee_block: Option<Nat>,
        refund_block: Option<Nat>,
    },
    GenericError {
        error_code: Nat,
        message: String,
    },
    InsufficientFunds {
        balance: Nat,
    },
    TemporarilyUnavailable,
    TooOld,
}
