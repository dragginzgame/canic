//! Passive startup binding transport projection.
//!
//! Bound the Candid type table to the selectors used by this operation; replies
//! decode only their required variants and fields. This avoids pulling unrelated
//! role operations, release receipts, and recursive registry payloads into each
//! transport. Compatibility is checked against the canonical role contracts.

use crate::dto::root::RootOperationStatusResponse;
use candid::CandidType;
use serde::Deserialize;

/// Bounded Response wire projection for startup binding.
#[derive(CandidType, Deserialize)]
pub enum Response {
    Operation(Box<RootOperationStatusResponse>),
}
