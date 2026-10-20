//! Passive capacity management transport projection.
//!
//! Bound the Candid type table to the selectors used by this operation; replies
//! decode only their required variants and fields. This avoids pulling unrelated
//! role operations, release receipts, and recursive registry payloads into each
//! transport. Compatibility is checked against the canonical role contracts.

use crate::dto::canister::{CanisterInspectionRequest, CanisterStatusResponse};
use ic_platform::candid::contract::CandidType;
use serde::Deserialize;

/// Bounded RootRequest wire projection for capacity management.

#[derive(CandidType)]
#[candid_path("ic_platform::candid::contract")]
pub enum RootRequest {
    InspectCanister(CanisterInspectionRequest),
}

/// Bounded RootResponse wire projection for capacity management.
#[derive(CandidType, Deserialize)]
#[candid_path("ic_platform::candid::contract")]
pub enum RootResponse {
    InspectCanister(Box<CanisterStatusResponse>),
    InspectionReserveRequired(crate::dto::canister::CanisterInspectionReserveResponse),
}
