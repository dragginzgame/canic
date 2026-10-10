//! Passive estate generation transport projection.
//!
//! Bound the Candid type table to the selectors used by this operation; replies
//! decode only their required variants and fields. This avoids pulling unrelated
//! role operations, release receipts, and recursive registry payloads into each
//! transport. Compatibility is checked against the canonical role contracts.

use crate::dto::{
    fleet_subnet_root::FleetSubnetRootAuthority,
    pool::{CanisterPoolResponse, CanisterPoolStatusRequest},
};
use serde::Deserialize;

/// Bounded RootEstateStatusRequest wire projection for estate generation.

#[derive(candid::CandidType)]
pub enum RootEstateStatusRequest {
    FleetAuthority,
    Pool(CanisterPoolStatusRequest),
}

/// Bounded RootEstateStatusResponse wire projection for estate generation.
#[derive(candid::CandidType, Deserialize)]
pub enum RootEstateStatusResponse {
    FleetAuthority(Box<FleetSubnetRootAuthority>),
    Pool(Box<CanisterPoolResponse>),
}
