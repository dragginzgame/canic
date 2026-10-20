//! Shared exact Cycles Ledger creation contracts for maintained host transports.

use ic_platform::{
    candid::contract::{CandidType, Nat},
    principal::ExternalPrincipal as Principal,
};
use serde::Deserialize;

#[derive(CandidType)]
#[candid_path("ic_platform::candid::contract")]
pub struct CreateCanisterArgs {
    pub amount: Nat,
    pub created_at_time: Option<u64>,
    pub creation_args: Option<CmcCreateCanisterArgs>,
    pub from_subaccount: Option<[u8; 32]>,
}

#[derive(CandidType)]
#[candid_path("ic_platform::candid::contract")]
pub struct CmcCreateCanisterArgs {
    pub settings: Option<CanisterSettings>,
    pub subnet_selection: Option<SubnetSelection>,
}

#[derive(CandidType)]
#[candid_path("ic_platform::candid::contract")]
pub struct CanisterSettings {
    pub compute_allocation: Option<Nat>,
    pub controllers: Option<Vec<Principal>>,
    pub freezing_threshold: Option<Nat>,
    pub memory_allocation: Option<Nat>,
    pub reserved_cycles_limit: Option<Nat>,
}

#[derive(CandidType)]
#[candid_path("ic_platform::candid::contract")]
pub enum SubnetSelection {
    Subnet { subnet: Principal },
}

#[derive(CandidType, Deserialize)]
#[candid_path("ic_platform::candid::contract")]
pub struct CreateCanisterSuccess {
    pub block_id: Nat,
    pub canister_id: Principal,
}

#[derive(CandidType, Debug, Deserialize)]
#[candid_path("ic_platform::candid::contract")]
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
