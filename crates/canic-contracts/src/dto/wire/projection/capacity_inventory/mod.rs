//! Passive capacity inventory transport projection.
//!
//! Bound the Candid type table to the selectors used by this operation; replies
//! decode only their required variants and fields. This avoids pulling unrelated
//! role operations, release receipts, and recursive registry payloads into each
//! transport. Compatibility is checked against the canonical role contracts.

use crate::dto::{
    fleet_registry::FleetRegistry,
    pool::{CanisterPoolResponse, CanisterPoolStatusRequest},
};
use candid::CandidType;
use serde::Deserialize;

/// Bounded CoordinatorRequest wire projection for capacity inventory.

#[derive(CandidType)]
pub enum CoordinatorRequest {
    Registry,
}

/// Bounded CoordinatorResponse wire projection for capacity inventory.
#[derive(CandidType, Deserialize)]
pub enum CoordinatorResponse {
    Registry(Box<FleetRegistry>),
}

/// Bounded RootRequest wire projection for capacity inventory.
#[derive(CandidType)]
pub enum RootRequest {
    Pool(CanisterPoolStatusRequest),
}

/// Bounded RootResponse wire projection for capacity inventory.
#[derive(CandidType, Deserialize)]
pub enum RootResponse {
    Pool(Box<CanisterPoolResponse>),
}
