//! Bounded reserve query/reply schema. Decode only the exact next-target quote;
//! unrelated Root observations remain outside this transport schema.

use crate::dto::canister::{CanisterInspectionRequest, CanisterInspectionReserveResponse};
use candid::CandidType;
use serde::Deserialize;

/// Controller-owned observation request for the exact next inspection target.

#[derive(CandidType)]
pub enum InspectionReserveRequest {
    InspectionReserve(CanisterInspectionRequest),
}

/// Indicative reserve evidence; this query cannot authorize a paid effect.
#[derive(CandidType, Deserialize)]
pub enum InspectionReserveResponse {
    InspectionReserve(CanisterInspectionReserveResponse),
}
