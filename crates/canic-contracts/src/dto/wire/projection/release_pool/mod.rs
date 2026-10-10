//! Passive release pool transport projection.
//!
//! Bound the Candid type table to the selectors used by this operation; replies
//! decode only their required variants and fields. This avoids pulling unrelated
//! role operations, release receipts, and recursive registry payloads into each
//! transport. Compatibility is checked against the canonical role contracts.

use crate::dto::root::RootPoolReleaseResponse;
use candid::CandidType;
use serde::Deserialize;

/// Bounded Request wire projection for release pool.

#[derive(CandidType)]
pub enum Request {
    PoolRelease,
}

/// Bounded Response wire projection for release pool.
#[derive(CandidType, Deserialize)]
pub enum Response {
    PoolRelease(Box<RootPoolReleaseResponse>),
}
