//! Passive protocol classifications; runtime decisions remain with Core.

use candid::CandidType;
use serde::{Deserialize, Serialize};

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum IcpRefillTrigger {
    Automatic { sequence: u64 },
    Manual,
}

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[remain::sorted]
pub enum IcpRefillStatus {
    Completed,
    Failed,
    InvalidTransaction,
    NotifyProcessing,
    Refunded,
    Requested,
    TransactionTooOld,
    Transferred,
}

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[remain::sorted]
pub enum IcpRefillErrorCode {
    BadFee,
    CyclesSentOverflow,
    Duplicate,
    InvalidLedgerBlockIndex,
    InvalidTransaction,
    LedgerTransferFailed,
    NotifyFailed,
    NotifyMaxAttempts,
    Processing,
    Refunded,
    TransactionTooOld,
    TransferWindowStale,
}
