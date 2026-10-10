//! Passive install history transport projection.
//!
//! Bound the Candid type table to the selectors used by this operation; replies
//! decode only their required variants and fields. This avoids pulling unrelated
//! role operations, release receipts, and recursive registry payloads into each
//! transport. Compatibility is checked against the canonical role contracts.

use crate::dto::canister::{CanisterHistoryResponse, CanisterInspectionRequest};
use candid::CandidType;
use serde::Deserialize;

/// Bounded Command wire projection for install history.

#[derive(CandidType)]
pub enum Command {
    InspectCanisterHistory(CanisterInspectionRequest),
}

/// Bounded Response wire projection for install history.
#[derive(CandidType, Deserialize)]
pub enum Response {
    InspectCanisterHistory(CanisterHistoryResponse),
}
