//! Bounded Root authority observation for local Fleet installation checks.
//!
//! Decode only the observed authority; unrelated Root status graphs do not
//! participate in controller and installation verification.

use crate::dto::fleet_subnet_root::FleetSubnetRootAuthority;
use candid::CandidType;
use serde::Deserialize;

/// Observe the complete current Root authority.
#[derive(CandidType)]
pub enum Request {
    FleetAuthority,
}

/// The Root-owned authority observation.
#[derive(CandidType, Deserialize)]
pub enum Response {
    FleetAuthority(Box<FleetSubnetRootAuthority>),
}
