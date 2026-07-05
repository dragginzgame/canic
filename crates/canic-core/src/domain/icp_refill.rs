//! Module: domain::icp_refill
//!
//! Responsibility: define pure ICP refill operation value enums shared across
//! storage projections, workflow decisions, and endpoint DTOs.
//! Does not own: endpoint request/response structs, stable records, or ledger
//! execution.
//! Boundary: DTOs re-export these values to preserve the public API path while
//! internal code imports them from the domain owner.

use candid::CandidType;
use serde::{Deserialize, Serialize};

///
/// IcpRefillMode
///

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[remain::sorted]
pub enum IcpRefillMode {
    Canister,
    Fabricate,
}

///
/// IcpRefillStatus
///

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

///
/// IcpRefillErrorCode
///
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[remain::sorted]
pub enum IcpRefillErrorCode {
    BadFee,
    Duplicate,
    FabricationUnavailable,
    InvalidLedgerBlockIndex,
    InvalidTransaction,
    LedgerTransferFailed,
    NotifyFailed,
    NotifyMaxAttempts,
    Processing,
    RateGateDenied,
    Refunded,
    RequestDenied,
    TransactionTooOld,
    TransferWindowStale,
}
