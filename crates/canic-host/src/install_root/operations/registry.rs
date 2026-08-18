//! Module: install_root::operations::registry
//!
//! Responsibility: read one complete live Fleet Registry evidence set during installation.
//! Does not own: expected Registry derivation, validation, mutation, or journal transitions.
//! Boundary: every workflow receives the same snapshot, manifest, and version projection.

use super::query_with_arg;
use crate::{
    fleet_install_plan::PersistedFleetInstallPlan, icp::IcpCli,
    protocol_binding::ResolvedProtocolBinding,
};
use candid::Principal;
use canic_control_plane::dto::fleet_coordinator::{
    CoordinatorStatusRequest, CoordinatorStatusResponse,
};
use canic_core::{
    dto::fleet_registry::{FleetRegistry, FleetRegistryManifest, FleetRegistryVersion},
    ids::{FleetCoordinatorBinding, FleetRegistryAuthority},
    protocol,
};

pub(in crate::install_root) fn fleet_registry_authority(
    fleet_install_plan: &PersistedFleetInstallPlan,
    coordinator: Principal,
) -> FleetRegistryAuthority {
    FleetRegistryAuthority {
        binding: FleetCoordinatorBinding {
            fleet: fleet_install_plan.plan.fleet.clone(),
            coordinator_subnet: fleet_install_plan.plan.coordinator.coordinator_subnet,
            coordinator,
        },
        epoch: 1,
    }
}

pub(in crate::install_root) struct LiveRegistryEvidence {
    pub(in crate::install_root) registry: FleetRegistry,
    pub(in crate::install_root) manifest: FleetRegistryManifest,
    pub(in crate::install_root) version: FleetRegistryVersion,
}

pub(in crate::install_root) fn query_live_registry(
    icp: &IcpCli,
    binding: &ResolvedProtocolBinding,
    coordinator: Principal,
) -> Result<LiveRegistryEvidence, Box<dyn std::error::Error>> {
    let registry = query_with_arg::<_, CoordinatorStatusResponse>(
        icp,
        binding,
        coordinator,
        protocol::CANIC_STATUS,
        &CoordinatorStatusRequest::Registry,
    )?;
    let manifest = query_with_arg::<_, CoordinatorStatusResponse>(
        icp,
        binding,
        coordinator,
        protocol::CANIC_STATUS,
        &CoordinatorStatusRequest::RegistryManifest,
    )?;
    let version = query_with_arg::<_, CoordinatorStatusResponse>(
        icp,
        binding,
        coordinator,
        protocol::CANIC_STATUS,
        &CoordinatorStatusRequest::RegistryVersion,
    )?;
    Ok(LiveRegistryEvidence {
        registry: match registry {
            CoordinatorStatusResponse::Registry(registry) => registry,
            _ => return Err("Coordinator returned an unrelated Registry response".into()),
        },
        manifest: match manifest {
            CoordinatorStatusResponse::RegistryManifest(manifest) => manifest,
            _ => return Err("Coordinator returned an unrelated Registry manifest response".into()),
        },
        version: match version {
            CoordinatorStatusResponse::RegistryVersion(version) => version,
            _ => return Err("Coordinator returned an unrelated Registry version response".into()),
        },
    })
}
