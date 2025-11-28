pub use icrc_ledger_types::icrc2::allowance::Allowance;

use crate::types::Account;
use candid::CandidType;
use serde::{Deserialize, Serialize};

///
/// AllowanceArgs
/// wrapped to use the canic Account
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AllowanceArgs {
    pub account: Account,
    pub spender: Account,
}
