//! Passive release coordinator funding transport projection.
//!
//! Bound the Candid type table to the selectors used by this operation; replies
//! decode only their required variants and fields. This avoids pulling unrelated
//! role operations, release receipts, and recursive registry payloads into each
//! transport. Compatibility is checked against the canonical role contracts.

use crate::dto::fleet_coordinator::CoordinatorFundingStatusResponse;
use candid::CandidType;
use serde::Deserialize;

/// Bounded Request wire projection for release coordinator funding.

#[derive(CandidType)]
pub enum Request {
    Funding,
}

/// Bounded Response wire projection for release coordinator funding.
#[derive(CandidType, Deserialize)]
pub enum Response {
    Funding(Box<CoordinatorFundingStatusResponse>),
}
