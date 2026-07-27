//! Module: dto::fleet_subnet_root
//!
//! Responsibility: carry protected fresh-install Fleet Subnet Root authority.
//! Does not own: validation, persistence, topology compilation, or lifecycle effects.
//! Boundary: the root lifecycle adapter passes init authority to workflow; controller queries
//! return the exact persisted authority without granting mutation rights.

use crate::ids::{FleetSubnetRootBinding, FleetSubnetRootReleaseSet};
use candid::CandidType;
use serde::Deserialize;

///
/// FleetSubnetRootAuthority
///
/// Exact immutable root binding, initial release set, and installed module identity.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct FleetSubnetRootAuthority {
    pub binding: FleetSubnetRootBinding,
    pub initial_release_set: FleetSubnetRootReleaseSet,
    pub expected_module_hash: [u8; 32],
}

///
/// FleetSubnetRootInitArgs
///
/// Fresh-install authority plus the reinstall-local activation operation identity.
///

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct FleetSubnetRootInitArgs {
    pub authority: FleetSubnetRootAuthority,
    pub install_id: [u8; 32],
}
