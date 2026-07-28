//! Module: install_root::fleet_catalog_publication
//!
//! Responsibility: validate complete terminal multi-root evidence before Fleet discovery
//! publication.
//! Does not own: Coordinator/root queries, lifecycle effects, or catalog persistence mechanics.
//! Boundary: the sole catalog writer is reachable only through this exact terminal-evidence gate.

#[cfg(test)]
mod tests;

use crate::{
    fleet_catalog::{
        CommittedFleetCatalog, FleetCatalogEntryV1, FleetCatalogError, commit_fleet_catalog_entry,
    },
    fleet_install_plan::FleetInstallPlan,
};
use std::path::Path;

use candid::Principal;
use canic_core::{
    control_plane_support::{
        config::ComponentTopology, error::InternalError, ops::fleet_registry::FleetRegistryOps,
    },
    dto::{
        fleet_registry::{
            FleetRegistrySnapshotResponse, FleetRegistryVersion, FleetSubnetRootEntry,
            FleetSubnetRootStatus,
        },
        fleet_subnet_root::FleetSubnetRootCanisterSummary,
    },
    ids::{FleetCoordinatorBinding, FleetName, FleetRegistryAuthority},
};
use thiserror::Error as ThisError;

///
/// TerminalFleetCatalogPublicationRequest
///
/// Exact live and planned authority required before publishing Fleet discovery.
///

pub(super) struct TerminalFleetCatalogPublicationRequest<'a> {
    pub project_root: &'a Path,
    pub fleet_name: FleetName,
    pub environment: &'a str,
    pub deployed_at_unix_secs: u64,
    pub fleet_install_plan: &'a FleetInstallPlan,
    pub component_topology: &'a ComponentTopology,
    pub coordinator: Principal,
    pub registry: &'a FleetRegistrySnapshotResponse,
    pub root_summaries: &'a [FleetSubnetRootCanisterSummary],
}

///
/// TerminalFleetCatalogPublicationError
///
/// Typed rejection before terminal Fleet discovery can become durable.
///

#[derive(Debug, ThisError)]
pub(super) enum TerminalFleetCatalogPublicationError {
    #[error("terminal Fleet catalog authority differs from the immutable install plan")]
    AuthorityMismatch,

    #[error(transparent)]
    Catalog(#[from] FleetCatalogError),

    #[error("terminal Fleet catalog publication time must be positive")]
    NonPositiveDeploymentTime,

    #[error("terminal Fleet Registry evidence is not exact")]
    RegistryEvidenceMismatch,

    #[error("terminal Fleet Registry validation failed: {0}")]
    RegistryValidation(#[source] InternalError),

    #[error("terminal Fleet root count or immutable root plan differs from the Registry")]
    RootSetMismatch,

    #[error("terminal evidence for Fleet Subnet Root {root} is incomplete or contradictory")]
    RootSummaryMismatch { root: Principal },
}

/// Publish one Coordinator-anchored Fleet row after exact terminal validation.
pub(super) fn publish_terminal_fleet_catalog(
    request: TerminalFleetCatalogPublicationRequest<'_>,
) -> Result<CommittedFleetCatalog, TerminalFleetCatalogPublicationError> {
    validate_terminal_authority(&request)?;
    let entry = FleetCatalogEntryV1 {
        canonical_network_id: request.fleet_install_plan.fleet.fleet.canonical_network_id,
        fleet_id: request.fleet_install_plan.fleet.fleet.fleet_id,
        fleet_name: request.fleet_name,
        app: request.fleet_install_plan.fleet.app.clone(),
        environment: request.environment.to_string(),
        deployed_at_unix_secs: request.deployed_at_unix_secs,
        coordinator_principal: request.coordinator.to_text(),
    };
    commit_fleet_catalog_entry(request.project_root, entry).map_err(Into::into)
}

fn validate_terminal_authority(
    request: &TerminalFleetCatalogPublicationRequest<'_>,
) -> Result<(), TerminalFleetCatalogPublicationError> {
    if request.deployed_at_unix_secs == 0 {
        return Err(TerminalFleetCatalogPublicationError::NonPositiveDeploymentTime);
    }
    let expected_authority = FleetRegistryAuthority {
        binding: FleetCoordinatorBinding {
            fleet: request.fleet_install_plan.fleet.clone(),
            coordinator_subnet: request.fleet_install_plan.coordinator.coordinator_subnet,
            coordinator: request.coordinator,
        },
        epoch: 1,
    };
    if request.coordinator == Principal::anonymous()
        || request.coordinator == Principal::management_canister()
        || request.registry.registry.authority != expected_authority
    {
        return Err(TerminalFleetCatalogPublicationError::AuthorityMismatch);
    }

    FleetRegistryOps::validate(
        &expected_authority,
        request.component_topology,
        &request.registry.registry,
    )
    .map_err(TerminalFleetCatalogPublicationError::RegistryValidation)?;
    let manifest = FleetRegistryOps::manifest(
        &expected_authority,
        request.component_topology,
        &request.registry.registry,
    )
    .map_err(TerminalFleetCatalogPublicationError::RegistryValidation)?;
    let version = FleetRegistryOps::version(
        &expected_authority,
        request.component_topology,
        &request.registry.registry,
    )
    .map_err(TerminalFleetCatalogPublicationError::RegistryValidation)?;
    if request.registry.manifest != manifest || request.registry.version != version {
        return Err(TerminalFleetCatalogPublicationError::RegistryEvidenceMismatch);
    }

    validate_terminal_roots(request, &version)
}

fn validate_terminal_roots(
    request: &TerminalFleetCatalogPublicationRequest<'_>,
    registry_version: &FleetRegistryVersion,
) -> Result<(), TerminalFleetCatalogPublicationError> {
    let planned_roots = &request.fleet_install_plan.fleet_subnet_roots;
    let registry_roots = &request.registry.registry.fleet_subnet_roots;
    if planned_roots.is_empty()
        || planned_roots.len() != registry_roots.len()
        || registry_roots.len() != request.root_summaries.len()
    {
        return Err(TerminalFleetCatalogPublicationError::RootSetMismatch);
    }

    for ((planned, registered), summary) in planned_roots
        .iter()
        .zip(registry_roots)
        .zip(request.root_summaries)
    {
        if !root_matches_plan(registered, planned) {
            return Err(TerminalFleetCatalogPublicationError::RootSetMismatch);
        }
        validate_root_summary(registered, registry_version, summary)?;
    }
    Ok(())
}

fn root_matches_plan(
    registered: &FleetSubnetRootEntry,
    planned: &crate::fleet_install_plan::PlannedFleetSubnetRoot,
) -> bool {
    let expected_release_set = planned.initial_release_set;
    registered.placement_subnet == planned.placement_subnet
        && registered.component_admissions == planned.component_admissions
        && registered.component_topology_digest == planned.component_topology_digest
        && registered.active_release_set == expected_release_set
        && registered.limits == planned.limits
        && registered.status == FleetSubnetRootStatus::Active
}

fn validate_root_summary(
    registered: &FleetSubnetRootEntry,
    registry_version: &FleetRegistryVersion,
    summary: &FleetSubnetRootCanisterSummary,
) -> Result<(), TerminalFleetCatalogPublicationError> {
    let total = summary
        .infrastructure_canisters
        .checked_add(summary.component_canisters);
    let managed = 1_u32.checked_add(summary.component_canisters);
    if summary.fleet_registry != *registry_version
        || summary.placement_subnet != registered.placement_subnet
        || summary.fleet_subnet_root != registered.fleet_subnet_root
        || summary.status != FleetSubnetRootStatus::Active
        || summary.infrastructure_canisters != 2
        || total != Some(summary.total_canisters)
        || managed.is_none_or(|count| count > registered.limits.maximum_managed_canisters)
    {
        return Err(TerminalFleetCatalogPublicationError::RootSummaryMismatch {
            root: registered.fleet_subnet_root,
        });
    }
    Ok(())
}
