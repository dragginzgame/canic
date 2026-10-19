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
use ic_platform::candid::contract::CandidType;
use serde::Deserialize;

/// Bounded CoordinatorRequest wire projection for capacity inventory.

#[derive(CandidType)]
#[candid_path("ic_platform::candid::contract")]
pub enum CoordinatorRequest {
    Registry,
}

/// Bounded CoordinatorResponse wire projection for capacity inventory.
#[derive(CandidType, Deserialize)]
#[candid_path("ic_platform::candid::contract")]
pub enum CoordinatorResponse {
    Registry(Box<FleetRegistry>),
}

/// Bounded RootRequest wire projection for capacity inventory.
#[derive(CandidType)]
#[candid_path("ic_platform::candid::contract")]
pub enum RootRequest {
    Pool(CanisterPoolStatusRequest),
}

/// Bounded RootResponse wire projection for capacity inventory.
#[derive(CandidType, Deserialize)]
#[candid_path("ic_platform::candid::contract")]
pub enum RootResponse {
    Pool(Box<CanisterPoolResponse>),
}
