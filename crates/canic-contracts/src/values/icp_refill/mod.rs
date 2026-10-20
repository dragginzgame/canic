//! Passive protocol classifications; runtime decisions remain with Core.

use ic_platform::candid::contract::CandidType;
use serde::{Deserialize, Serialize};

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[candid_path("ic_platform::candid::contract")]
pub enum IcpRefillTrigger {
    Automatic { sequence: u64 },
    Manual,
}

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[candid_path("ic_platform::candid::contract")]
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
#[candid_path("ic_platform::candid::contract")]
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
