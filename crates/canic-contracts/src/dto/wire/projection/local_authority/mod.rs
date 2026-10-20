//! Bounded Root authority observation for local Fleet installation checks.
//!
//! Decode only the observed authority; unrelated Root status graphs do not
//! participate in controller and installation verification.

use crate::dto::fleet_subnet_root::FleetSubnetRootAuthority;
use ic_platform::candid::contract::CandidType;
use serde::Deserialize;

/// Observe the complete current Root authority.
#[derive(CandidType)]
#[candid_path("ic_platform::candid::contract")]
pub enum Request {
    FleetAuthority,
}

/// The Root-owned authority observation.
#[derive(CandidType, Deserialize)]
#[candid_path("ic_platform::candid::contract")]
pub enum Response {
    FleetAuthority(Box<FleetSubnetRootAuthority>),
}
