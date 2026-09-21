//! Module: fleet_ensure::ops::startup_funding::observation::registry
//!
//! Responsibility: read and validate current registry evidence for funding observations.
//! Boundary: query-only, invocation-local evidence; neither a funding approval nor a cache.

#[cfg(test)]
mod tests;

use crate::{
    canister_protocol::query_with_candid,
    fleet_ensure::view::startup_funding::{
        StartupFundingPlacement, StartupFundingRegistry, StartupRootInventory,
        StartupUsageUnavailable,
    },
    icp::IcpCli,
};
use candid::{CandidType, Deserialize, Principal};
use canic_control_plane::dto::fleet_coordinator::{
    CoordinatorObservabilityRequest, CoordinatorObservabilityResponse, CoordinatorRegistryRequest,
    CoordinatorRegistryResponse,
};
use canic_core::{
    control_plane_support::{config::ComponentTopology, ops::fleet_registry::FleetRegistryOps},
    dto::{
        fleet_registry::{FleetRegistry, FleetRegistryVersion, FleetSubnetRootStatus},
        fleet_subnet_root::FleetSubnetRootCanisterSummary,
    },
    protocol,
};
use std::path::Path;

#[derive(CandidType)]
enum RootRequest {
    Inventory,
}

#[derive(CandidType, Deserialize)]
enum RootResponse {
    Inventory(FleetSubnetRootCanisterSummary),
}

/// Require the selected Root's active mirror to match the independently read Coordinator head.
pub fn observe_root(
    icp: &IcpCli,
    candid: &Path,
    root: Principal,
    registry: &StartupFundingRegistry,
) -> Result<StartupRootInventory, StartupUsageUnavailable> {
    let RootResponse::Inventory(summary) = query_with_candid(
        icp,
        candid,
        root,
        protocol::CANIC_ROOT_STATUS,
        &RootRequest::Inventory,
    )
    .map_err(|_| StartupUsageUnavailable::ObservationFailed)?;
    match_root(root, registry, &summary)?;
    Ok(StartupRootInventory {
        workloads: summary.component_canisters,
    })
}

fn match_root(
    root: Principal,
    registry: &StartupFundingRegistry,
    summary: &FleetSubnetRootCanisterSummary,
) -> Result<(), StartupUsageUnavailable> {
    let placement = registry
        .roots
        .get(&root)
        .ok_or(StartupUsageUnavailable::AuthorityMismatch)?;
    if summary.fleet_subnet_root != root || summary.placement_subnet != placement.placement_subnet {
        return Err(StartupUsageUnavailable::AuthorityMismatch);
    }
    if !placement.active || summary.status != FleetSubnetRootStatus::Active {
        return Err(StartupUsageUnavailable::PolicyTransition);
    }
    match_head(registry, &summary.fleet_registry)
}

/// Read one canonical snapshot from the selected Coordinator before child accounting.
pub fn observe(
    icp: &IcpCli,
    candid: &Path,
    coordinator: Principal,
    topology: &ComponentTopology,
) -> Result<StartupFundingRegistry, StartupUsageUnavailable> {
    let CoordinatorRegistryResponse::Registry(registry) = query_with_candid(
        icp,
        candid,
        coordinator,
        protocol::CANIC_COORDINATOR_REGISTRY,
        &CoordinatorRegistryRequest::Registry,
    )
    .map_err(|_| StartupUsageUnavailable::ObservationFailed)?;
    project(coordinator, topology, registry)
}

/// Fence a collected preview against a changed or unavailable Coordinator head.
pub fn recheck(
    icp: &IcpCli,
    candid: &Path,
    registry: &StartupFundingRegistry,
) -> Result<(), StartupUsageUnavailable> {
    let response = query_with_candid(
        icp,
        candid,
        registry.authority.binding.coordinator,
        protocol::CANIC_OBSERVABILITY,
        &CoordinatorObservabilityRequest::RegistryVersion,
    )
    .map_err(|_| StartupUsageUnavailable::ObservationFailed)?;
    let CoordinatorObservabilityResponse::RegistryVersion(version) = response else {
        return Err(StartupUsageUnavailable::ObservationFailed);
    };
    match_head(registry, &version)
}

fn match_head(
    registry: &StartupFundingRegistry,
    version: &FleetRegistryVersion,
) -> Result<(), StartupUsageUnavailable> {
    let expected = FleetRegistryVersion {
        authority: registry.authority.clone(),
        revision: registry.revision,
        content_hash: registry.content_hash,
    };
    if *version != expected {
        return Err(StartupUsageUnavailable::PolicyTransition);
    }
    Ok(())
}

fn project(
    coordinator: Principal,
    topology: &ComponentTopology,
    registry: FleetRegistry,
) -> Result<StartupFundingRegistry, StartupUsageUnavailable> {
    if registry.authority.binding.coordinator != coordinator {
        return Err(StartupUsageUnavailable::AuthorityMismatch);
    }
    // Version construction validates canonical ordering, unique placements, topology,
    // admission authority and the complete bounded encoding before hashing it.
    let version = FleetRegistryOps::version(&registry.authority, topology, &registry)
        .map_err(|_| StartupUsageUnavailable::AuthorityMismatch)?;
    let roots = registry
        .fleet_subnet_roots
        .into_iter()
        .map(|root| {
            (
                root.fleet_subnet_root,
                StartupFundingPlacement {
                    active: root.status == FleetSubnetRootStatus::Active,
                    placement_subnet: root.placement_subnet,
                    release_set: root.active_release_set,
                    component_admissions: root.component_admissions,
                    component_topology_digest: root.component_topology_digest,
                    limits: root.limits,
                    funding: root.funding,
                },
            )
        })
        .collect();
    Ok(StartupFundingRegistry {
        authority: version.authority,
        revision: version.revision,
        content_hash: version.content_hash,
        roots,
    })
}
