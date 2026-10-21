//! Passive subnet information transport projection.
//!
//! Bound the Candid type table to the selectors used by this operation; replies
//! decode only their required variants and fields. This avoids pulling unrelated
//! role operations, release receipts, and recursive registry payloads into each
//! transport. Compatibility is checked against the canonical role contracts.

use crate::dto::{
    fleet_registry::{FleetRegistry, FleetRegistryManifest, FleetRegistryVersion},
    fleet_subnet_root::FleetSubnetRootCanisterSummary,
};
use ic_platform::candid::contract::CandidType;

/// Bounded CoordinatorStatusRequestFragment wire projection for subnet information.

#[derive(CandidType)]
#[candid_path("ic_platform::candid::contract")]
pub enum CoordinatorStatusRequestFragment {
    Registry,
    RegistryManifest,
    RegistryVersion,
}

/// Bounded CoordinatorStatusResponseFragment wire projection for subnet information.
#[derive(CandidType, serde::Deserialize)]
#[candid_path("ic_platform::candid::contract")]
pub enum CoordinatorStatusResponseFragment {
    Registry(Box<FleetRegistry>),
    RegistryManifest(FleetRegistryManifest),
    RegistryVersion(FleetRegistryVersion),
}

/// Bounded RootStatusRequestFragment wire projection for subnet information.
#[derive(CandidType)]
#[candid_path("ic_platform::candid::contract")]
pub enum RootStatusRequestFragment {
    Inventory,
}

/// Bounded RootStatusResponseFragment wire projection for subnet information.
#[derive(CandidType, serde::Deserialize)]
#[candid_path("ic_platform::candid::contract")]
pub enum RootStatusResponseFragment {
    Inventory(FleetSubnetRootCanisterSummary),
}
