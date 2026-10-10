//! Passive cycle conversion transport projection.
//!
//! Bound the Candid type table to the selectors used by this operation; replies
//! decode only their required variants and fields. This avoids pulling unrelated
//! role operations, release receipts, and recursive registry payloads into each
//! transport. Compatibility is checked against the canonical role contracts.

use crate::dto::icp_refill::IcpRefillResponse;
use candid::CandidType;
use serde::Deserialize;

/// Bounded RootOperationStatusResponse wire projection for cycle conversion.
#[derive(CandidType, Deserialize)]
pub enum RootOperationStatusResponse {
    RefillCycles(IcpRefillResponse),
}

/// Bounded RootStatusResponse wire projection for cycle conversion.
#[derive(CandidType, Deserialize)]
pub enum RootStatusResponse {
    Operation(RootOperationStatusResponse),
}
