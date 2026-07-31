//! Module: install_root::operations::registry
//!
//! Responsibility: read one complete live Fleet Registry evidence set during installation.
//! Does not own: expected Registry derivation, validation, mutation, or journal transitions.
//! Boundary: every workflow receives the same snapshot, manifest, and version projection.

use super::query_no_arg;
use crate::icp::IcpCli;
use candid::Principal;
use canic_core::{
    dto::fleet_registry::{FleetRegistry, FleetRegistryManifest, FleetRegistryVersion},
    protocol,
};

pub(in crate::install_root) struct LiveRegistryEvidence {
    pub(in crate::install_root) registry: FleetRegistry,
    pub(in crate::install_root) manifest: FleetRegistryManifest,
    pub(in crate::install_root) version: FleetRegistryVersion,
}

pub(in crate::install_root) fn query_live_registry(
    icp: &IcpCli,
    coordinator: Principal,
) -> Result<LiveRegistryEvidence, Box<dyn std::error::Error>> {
    Ok(LiveRegistryEvidence {
        registry: query_no_arg(icp, coordinator, protocol::CANIC_FLEET_REGISTRY)?,
        manifest: query_no_arg(icp, coordinator, protocol::CANIC_FLEET_REGISTRY_MANIFEST)?,
        version: query_no_arg(icp, coordinator, protocol::CANIC_FLEET_REGISTRY_VERSION)?,
    })
}
