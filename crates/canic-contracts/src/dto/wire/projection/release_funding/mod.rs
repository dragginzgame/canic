//! Passive release funding transport projection.
//!
//! Bound the Candid type table to the selectors used by this operation; replies
//! decode only their required variants and fields. This avoids pulling unrelated
//! role operations, release receipts, and recursive registry payloads into each
//! transport. Compatibility is checked against the canonical role contracts.

use crate::dto::root::RootFundingReleaseResponse;
use candid::CandidType;
use serde::Deserialize;

/// Bounded Request wire projection for release funding.

#[derive(CandidType)]
pub enum Request {
    FundingRelease(Option<u64>),
}

/// Bounded Response wire projection for release funding.
#[derive(CandidType, Deserialize)]
pub enum Response {
    FundingRelease(Box<RootFundingReleaseResponse>),
}
