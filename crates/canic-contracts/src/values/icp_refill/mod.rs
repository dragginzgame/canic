//! Passive protocol classifications; runtime decisions remain with Core.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum IcpRefillTrigger {
    Automatic { sequence: u64 },
    Manual,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[remain::sorted]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
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

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[remain::sorted]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
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
