//! Bounded reserve query/reply schema. Decode only the exact next-target quote;
//! unrelated Root observations remain outside this transport schema.

use crate::dto::canister::{CanisterInspectionRequest, CanisterInspectionReserveResponse};
use ic_platform::candid::contract::CandidType;
use serde::Deserialize;

/// Controller-owned observation request for the exact next inspection target.

#[derive(CandidType)]
#[candid_path("ic_platform::candid::contract")]
pub enum InspectionReserveRequest {
    InspectionReserve(CanisterInspectionRequest),
}

/// Indicative reserve evidence; this query cannot authorize a paid effect.
#[derive(CandidType, Deserialize)]
#[candid_path("ic_platform::candid::contract")]
pub enum InspectionReserveResponse {
    InspectionReserve(CanisterInspectionReserveResponse),
}
