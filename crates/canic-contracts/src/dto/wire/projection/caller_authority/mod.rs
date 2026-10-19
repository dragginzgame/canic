//! Passive caller authority transport projection.
//!
//! Bound the Candid type table to the selectors used by this operation; replies
//! decode only their required variants and fields. This avoids pulling unrelated
//! role operations, release receipts, and recursive registry payloads into each
//! transport. Compatibility is checked against the canonical role contracts.

use crate::dto::{
    caller_authority::{
        CallerAuthorityCommand, CallerAuthorityPublication, CallerAuthorityReceipt,
        CallerAuthorityStatus,
    },
    role::{OperationReceipt, OperationStatusRequest},
};
use ic_platform::candid::contract::CandidType;
use serde::Deserialize;

/// Bounded Command wire projection for caller authority.

#[derive(CandidType)]
#[candid_path("ic_platform::candid::contract")]
pub enum Command {
    CallerAuthority(CallerAuthorityCommand),
    ReleaseApplicationStartup(CallerAuthorityPublication),
}

/// Bounded CommandResponse wire projection for caller authority.
#[derive(CandidType, Deserialize)]
#[candid_path("ic_platform::candid::contract")]
pub enum CommandResponse {
    CallerAuthority(Box<CallerAuthorityReceipt>),
    OperationAccepted(OperationReceipt),
}

/// Bounded StatusRequest wire projection for caller authority.
#[derive(CandidType)]
#[candid_path("ic_platform::candid::contract")]
pub enum StatusRequest {
    CallerAuthority(OperationStatusRequest),
}

/// Bounded StatusResponse wire projection for caller authority.
#[derive(CandidType, Deserialize)]
#[candid_path("ic_platform::candid::contract")]
pub enum StatusResponse {
    CallerAuthority(CallerAuthorityStatus),
}
