//! Exact upstream ICP Ledger and CMC Candid contracts used by the mint adapter.

use candid::{CandidType, Nat};
use serde::Deserialize;

#[derive(CandidType, Deserialize)]
pub struct Tokens {
    pub e8s: u64,
}

#[derive(CandidType, Deserialize)]
pub struct Timestamp {
    pub timestamp_nanos: u64,
}

#[derive(CandidType, Deserialize)]
pub struct TransferArgs {
    pub memo: u64,
    pub amount: Tokens,
    pub fee: Tokens,
    pub from_subaccount: Option<[u8; 32]>,
    pub to: [u8; 32],
    pub created_at_time: Option<Timestamp>,
}

#[derive(CandidType, Deserialize)]
pub enum TransferError {
    BadFee { expected_fee: Tokens },
    InsufficientFunds { balance: Tokens },
    TxTooOld { allowed_window_nanos: u64 },
    TxCreatedInFuture,
    TxDuplicate { duplicate_of: u64 },
}

#[derive(CandidType, Deserialize)]
pub struct NotifyMintArgs {
    pub block_index: u64,
    pub deposit_memo: Option<Vec<u8>>,
    pub to_subaccount: Option<[u8; 32]>,
}

#[derive(CandidType, Deserialize)]
pub struct NotifyMintOk {
    pub balance: Nat,
    pub block_index: Nat,
    pub minted: Nat,
}

#[derive(CandidType, Deserialize)]
pub enum NotifyMintError {
    Refunded {
        block_index: Option<u64>,
        reason: String,
    },
    InvalidTransaction(String),
    Other {
        error_message: String,
        error_code: u64,
    },
    Processing,
    TransactionTooOld(u64),
}
