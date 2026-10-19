//! Passive release receipts transport projection.
//!
//! Bound the Candid type table to the selectors used by this operation; replies
//! decode only their required variants and fields. This avoids pulling unrelated
//! role operations, release receipts, and recursive registry payloads into each
//! transport. Compatibility is checked against the canonical role contracts.

use crate::dto::release_receipts::ReplayReleaseResponse;
use ic_platform::candid::contract::CandidType;
use serde::Deserialize;

/// Bounded Request wire projection for release receipts.

#[derive(CandidType)]
#[candid_path("ic_platform::candid::contract")]
pub enum Request {
    ReplayRelease(Option<[u8; 32]>),
}

/// Bounded Response wire projection for release receipts.
#[derive(CandidType, Deserialize)]
#[candid_path("ic_platform::candid::contract")]
pub enum Response {
    ReplayRelease(ReplayReleaseResponse),
}
