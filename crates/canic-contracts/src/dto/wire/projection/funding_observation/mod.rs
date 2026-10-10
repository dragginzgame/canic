//! Passive funding observation transport projection.
//!
//! Bound the Candid type table to the selectors used by this operation; replies
//! decode only their required variants and fields. This avoids pulling unrelated
//! role operations, release receipts, and recursive registry payloads into each
//! transport. Compatibility is checked against the canonical role contracts.

use crate::dto::{
    canister::{CanisterInspectionRequest, CanisterStatusResponse},
    observability::{CanisterObservabilityResponse, FleetCanisterObservabilityRequest},
};
use candid::CandidType;
use serde::Deserialize;

/// Bounded Command wire projection for funding observation.

#[derive(CandidType)]
pub enum Command {
    InspectCanister(CanisterInspectionRequest),
    ObserveCanister(FleetCanisterObservabilityRequest),
}

/// Bounded Response wire projection for funding observation.
#[derive(CandidType, Deserialize)]
pub enum Response {
    InspectCanister(Box<CanisterStatusResponse>),
    InspectionReserveRequired(crate::dto::canister::CanisterInspectionReserveResponse),
    ObserveCanister(CanisterObservabilityResponse),
}
