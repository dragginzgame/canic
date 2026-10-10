//! Passive observability relay transport projection.
//!
//! Bound the Candid type table to the selectors used by this operation; replies
//! decode only their required variants and fields. This avoids pulling unrelated
//! role operations, release receipts, and recursive registry payloads into each
//! transport. Compatibility is checked against the canonical role contracts.

use crate::dto::observability::{CanisterObservabilityRequest, CanisterObservabilityResponse};
use candid::CandidType;
use serde::Deserialize;

/// Bounded CanisterCommandFragment wire projection for observability relay.

#[derive(CandidType)]
pub enum CanisterCommandFragment {
    Observe(CanisterObservabilityRequest),
}

/// Bounded CanisterCommandResponseFragment wire projection for observability relay.
#[derive(CandidType, Deserialize)]
pub enum CanisterCommandResponseFragment {
    Observe(CanisterObservabilityResponse),
}
