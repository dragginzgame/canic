//! Passive component operation transport projection.
//!
//! Bound the Candid type table to the selectors used by this operation; replies
//! decode only their required variants and fields. This avoids pulling unrelated
//! role operations, release receipts, and recursive registry payloads into each
//! transport. Compatibility is checked against the canonical role contracts.

use crate::dto::{
    component_registry::RootComponentAllocationRequest,
    fleet_subnet_root::FleetSubnetRootAuthority,
    pool::{CanisterPoolResponse, CanisterPoolStatusRequest},
    role::OperationStatusRequest,
    root::RootOperationStatusResponse,
};
use candid::CandidType;
use serde::Deserialize;

/// Bounded RootRead wire projection for component operation.

#[derive(CandidType, Deserialize)]
pub enum RootRead {
    FleetAuthority,
    Operation(OperationStatusRequest),
    Pool(CanisterPoolStatusRequest),
}

/// Bounded RootResponse wire projection for component operation.
#[derive(CandidType, Deserialize)]
pub enum RootResponse {
    FleetAuthority(Box<FleetSubnetRootAuthority>),
    Operation(Box<RootOperationStatusResponse>),
    Pool(Box<CanisterPoolResponse>),
}

/// Bounded RootCommand wire projection for component operation.
#[derive(CandidType, Deserialize)]
pub enum RootCommand {
    ProvisionComponent(RootComponentAllocationRequest),
}
