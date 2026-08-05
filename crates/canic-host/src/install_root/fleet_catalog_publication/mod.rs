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
    pub workspace_root: &'a Path,
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

    #[error("terminal Fleet Subnet Root count or immutable root plan differs from the Registry")]
    RootSetMismatch,

    #[error("terminal evidence for Fleet Subnet Root {root} is incomplete or contradictory")]
    RootSummaryMismatch { root: Principal },
}

/// Publish one Coordinator-anchored Fleet row after exact terminal validation.
pub(super) fn publish_terminal_fleet_catalog(
    request: TerminalFleetCatalogPublicationRequest<'_>,
) -> Result<CommittedFleetCatalog, TerminalFleetCatalogPublicationError> {
    if request.deployed_at_unix_secs == 0 {
        return Err(TerminalFleetCatalogPublicationError::NonPositiveDeploymentTime);
    }
    validate_terminal_fleet_registry(
        request.fleet_install_plan,
        request.component_topology,
        request.coordinator,
        request.registry,
    )?;
    validate_terminal_root_summaries(
        request.fleet_install_plan,
        request.registry,
        request.root_summaries,
    )?;
    let entry = FleetCatalogEntryV1 {
        canonical_network_id: request.fleet_install_plan.fleet.fleet.canonical_network_id,
        fleet_id: request.fleet_install_plan.fleet.fleet.fleet_id,
        fleet_name: request.fleet_name,
        app: request.fleet_install_plan.fleet.app.clone(),
        environment: request.environment.to_string(),
        deployed_at_unix_secs: request.deployed_at_unix_secs,
        coordinator_principal: request.coordinator.to_text(),
    };
    commit_fleet_catalog_entry(request.workspace_root, entry).map_err(Into::into)
}

/// Validate the complete Coordinator Registry before any root is trusted as a query target.
pub(super) fn validate_terminal_fleet_registry(
    fleet_install_plan: &FleetInstallPlan,
    component_topology: &ComponentTopology,
    coordinator: Principal,
    registry: &FleetRegistrySnapshotResponse,
) -> Result<(), TerminalFleetCatalogPublicationError> {
    let expected_authority = FleetRegistryAuthority {
        binding: FleetCoordinatorBinding {
            fleet: fleet_install_plan.fleet.clone(),
            coordinator_subnet: fleet_install_plan.coordinator.coordinator_subnet,
            coordinator,
        },
        epoch: 1,
    };
    if coordinator == Principal::anonymous()
        || coordinator == Principal::management_canister()
        || registry.registry.authority != expected_authority
    {
        return Err(TerminalFleetCatalogPublicationError::AuthorityMismatch);
    }

    FleetRegistryOps::validate(&expected_authority, component_topology, &registry.registry)
        .map_err(TerminalFleetCatalogPublicationError::RegistryValidation)?;
    let manifest =
        FleetRegistryOps::manifest(&expected_authority, component_topology, &registry.registry)
            .map_err(TerminalFleetCatalogPublicationError::RegistryValidation)?;
    let version =
        FleetRegistryOps::version(&expected_authority, component_topology, &registry.registry)
            .map_err(TerminalFleetCatalogPublicationError::RegistryValidation)?;
    if registry.manifest != manifest || registry.version != version {
        return Err(TerminalFleetCatalogPublicationError::RegistryEvidenceMismatch);
    }

    validate_terminal_root_set(fleet_install_plan, registry)
}

fn validate_terminal_root_set(
    fleet_install_plan: &FleetInstallPlan,
    registry: &FleetRegistrySnapshotResponse,
) -> Result<(), TerminalFleetCatalogPublicationError> {
    let planned_roots = &fleet_install_plan.fleet_subnet_roots;
    let registry_roots = &registry.registry.fleet_subnet_roots;
    if planned_roots.is_empty() || planned_roots.len() != registry_roots.len() {
        return Err(TerminalFleetCatalogPublicationError::RootSetMismatch);
    }

    for (planned, registered) in planned_roots.iter().zip(registry_roots) {
        if !root_matches_plan(registered, planned) {
            return Err(TerminalFleetCatalogPublicationError::RootSetMismatch);
        }
    }
    Ok(())
}

fn validate_terminal_root_summaries(
    fleet_install_plan: &FleetInstallPlan,
    registry: &FleetRegistrySnapshotResponse,
    root_summaries: &[FleetSubnetRootCanisterSummary],
) -> Result<(), TerminalFleetCatalogPublicationError> {
    let planned_roots = &fleet_install_plan.fleet_subnet_roots;
    let registry_roots = &registry.registry.fleet_subnet_roots;
    if planned_roots.len() != registry_roots.len() || registry_roots.len() != root_summaries.len() {
        return Err(TerminalFleetCatalogPublicationError::RootSetMismatch);
    }

    for (registered, summary) in registry_roots.iter().zip(root_summaries) {
        validate_root_summary(registered, &registry.version, summary)?;
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
    let matches_registry = root_summary_matches_registry(registered, registry_version, summary);
    let matches_counts =
        RootSummaryCounts::derive(summary).is_some_and(|counts| counts.matches(summary));
    if !matches_registry || !matches_counts {
        return Err(TerminalFleetCatalogPublicationError::RootSummaryMismatch {
            root: registered.fleet_subnet_root,
        });
    }
    Ok(())
}

fn root_summary_matches_registry(
    registered: &FleetSubnetRootEntry,
    registry_version: &FleetRegistryVersion,
    summary: &FleetSubnetRootCanisterSummary,
) -> bool {
    summary.fleet_registry == *registry_version
        && summary.placement_subnet == registered.placement_subnet
        && summary.fleet_subnet_root == registered.fleet_subnet_root
        && summary.status == FleetSubnetRootStatus::Active
}

struct RootSummaryCounts {
    total_canisters: u32,
}

impl RootSummaryCounts {
    fn derive(summary: &FleetSubnetRootCanisterSummary) -> Option<Self> {
        let total_canisters = summary
            .infrastructure_canisters
            .checked_add(summary.component_canisters)?
            .checked_add(summary.pooled_canisters)?;
        Some(Self { total_canisters })
    }

    const fn matches(&self, summary: &FleetSubnetRootCanisterSummary) -> bool {
        summary.infrastructure_canisters == 2 && self.total_canisters == summary.total_canisters
    }
}
