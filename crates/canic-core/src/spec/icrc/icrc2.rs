use crate::cdk::types::Account;
use candid::CandidType;
use serde::{Deserialize, Serialize};

///
/// Allowance
///

#[derive(CandidType, Deserialize)]
pub struct Allowance {
    pub allowance: u64,
    pub expires_at: Option<u64>,
}

///
/// AllowanceArgs
/// wrapped to use the canic Account
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AllowanceArgs {
    pub account: Account,
    pub spender: Account,
}

///
/// TransferFromArgs
/// transfer_from() arguments
///

#[derive(CandidType, Serialize)]
pub struct TransferFromArgs {
    pub from: Account,
    pub to: Account,
    pub amount: u64,
    pub memo: Option<Vec<u8>>,
    pub created_at_time: Option<u64>,
}

///
/// TransferFromResult
///

#[derive(CandidType, Deserialize)]
pub enum TransferFromResult {
    #[serde(rename = "Ok")]
    Ok(u64), // Transaction index
    #[serde(rename = "Err")]
    Err(TransferFromError),
}

///
/// TransferFromError
///

#[derive(CandidType, Debug, Deserialize)]
pub enum TransferFromError {
    BadFee { expected_fee: u64 },
    BadBurn { min_burn_amount: u64 },
    InsufficientFunds { balance: u64 },
    InsufficientAllowance { allowance: u64 },
    TooOld,
    CreatedInFuture { ledger_time: u64 },
    Duplicate { duplicate_of: u64 },
    TemporarilyUnavailable,
    GenericError { error_code: u64, message: String },
}
