use crate::types::Account;
use candid::{CandidType, Nat};
use serde::{Deserialize, Serialize};

///
/// icrc-ledger-types has a conflict so we can just copy and paste here for now
/// @todo
///

/// The arguments for the
/// [ICRC-2 `allowance`](https://github.com/dfinity/ICRC-1/blob/main/standards/ICRC-2/README.md#icrc2_allowance)
/// endpoint.
#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AllowanceArgs {
    pub account: Account,
    pub spender: Account,
}

/// The `Allowance` response type for the
/// [ICRC-2 `allowance`](https://github.com/dfinity/ICRC-1/blob/main/standards/ICRC-2/README.md#icrc2_allowance)
/// endpoint.
#[derive(CandidType, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Allowance {
    pub allowance: Nat,
    #[serde(default)]
    pub expires_at: Option<u64>,
}
