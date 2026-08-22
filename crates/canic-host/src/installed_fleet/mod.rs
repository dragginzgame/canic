use crate::{
    fleet_catalog::{FleetCatalogEntryV1, FleetCatalogError, read_fleet_catalog_entry_from_root},
    fleet_install_plan::load_persisted_fleet_install_plan,
    install_root::{
        discover_workspace_canic_config_choices, load_verified_installed_fleet_registry,
        select_discovered_app_config_path,
    },
    registry::RegistryEntry,
    release_set::AppConfigSnapshot,
};
use candid::Principal;
use canic_core::{
    dto::fleet_registry::{FleetRegistry, FleetSubnetRootStatus},
    ids::{
        FleetBinding, FleetCoordinatorRootFundingPolicy, FleetKey, FleetSubnetRootFundingAuthority,
        SubnetId,
    },
};
use std::{collections::BTreeMap, path::Path};
use thiserror::Error as ThisError;

///
/// InstalledFleetRequest
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstalledFleetRequest {
    pub fleet: String,
    pub environment: String,
}

///
/// InstalledFleetResolution
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstalledFleetResolution {
    pub source: InstalledFleetSource,
    pub fleet: FleetCatalogEntryV1,
    pub registry: InstalledFleetRegistry,
    pub topology: ResolvedFleetTopology,
}

/// Exact selected Root from authenticated installed Fleet authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstalledFleetRootResolution {
    pub fleet: FleetCatalogEntryV1,
    pub root_canister_id: Principal,
}

/// Exact Coordinator from authenticated installed Fleet authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstalledFleetCoordinatorResolution {
    pub fleet: FleetCatalogEntryV1,
    pub coordinator_canister_id: Principal,
}

/// Authenticated installed placement and funding authority for one Root.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstalledFleetRootFundingResolution {
    pub fleet_subnet_root: Principal,
    pub placement_subnet: SubnetId,
    pub status: FleetSubnetRootStatus,
    pub funding: FleetSubnetRootFundingAuthority,
    pub placement_cost: crate::fleet_install_plan::PlannedSubnetPlacementCostEvidence,
}

/// Authenticated installed infrastructure funding authority retained for diagnostics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstalledFleetFundingResolution {
    pub fleet: FleetCatalogEntryV1,
    pub coordinator_canister_id: Principal,
    pub coordinator_root_funding: Option<FleetCoordinatorRootFundingPolicy>,
    pub coordinator_placement_cost: crate::fleet_install_plan::PlannedSubnetPlacementCostEvidence,
    pub roots: Vec<InstalledFleetRootFundingResolution>,
}

struct InstalledFleetAuthority {
    fleet: FleetCatalogEntryV1,
    registry: FleetRegistry,
    plan: crate::fleet_install_plan::FleetInstallPlan,
}

///
/// InstalledFleetSource
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InstalledFleetSource {
    LocalReplica,
    IcpCli,
}

///
/// InstalledFleetRegistry
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstalledFleetRegistry {
    pub root_canister_id: String,
    pub entries: Vec<RegistryEntry>,
}

///
/// ResolvedFleetTopology
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedFleetTopology {
    pub root_canister_id: String,
    pub children_by_parent: BTreeMap<Option<String>, Vec<String>>,
    pub roles_by_canister: BTreeMap<String, String>,
}

///
/// InstalledFleetError
///

#[derive(Debug, ThisError)]
pub enum InstalledFleetError {
    #[error("Fleet {fleet} is not installed on environment {environment}")]
    NoInstalledFleet { environment: String, fleet: String },

    #[error("failed to read the canonical-network Fleet catalog: {0}")]
    FleetCatalog(#[from] FleetCatalogError),

    #[error(
        "Fleet {fleet} is Coordinator-anchored at {coordinator}; the removed single-root topology resolver cannot serve this operation"
    )]
    CoordinatorAnchoredTopologyUnavailable { fleet: String, coordinator: String },

    #[error("installed Fleet authority is invalid: {0}")]
    InstalledAuthority(String),

    #[error("installed Fleet {fleet} has no current root {root}")]
    RootNotInFleet { fleet: String, root: Principal },
}

pub fn resolve_installed_fleet_from_root(
    request: &InstalledFleetRequest,
    icp_root: &Path,
) -> Result<InstalledFleetResolution, InstalledFleetError> {
    let fleet = read_installed_fleet_from_root(&request.environment, &request.fleet, icp_root)?;
    Err(
        InstalledFleetError::CoordinatorAnchoredTopologyUnavailable {
            fleet: fleet.fleet_name.to_string(),
            coordinator: fleet.coordinator_principal,
        },
    )
}

pub fn read_installed_fleet_from_root(
    environment: &str,
    fleet: &str,
    icp_root: &Path,
) -> Result<FleetCatalogEntryV1, InstalledFleetError> {
    read_fleet_catalog_entry_from_root(icp_root, environment, fleet)
        .map_err(InstalledFleetError::FleetCatalog)?
        .ok_or_else(|| InstalledFleetError::NoInstalledFleet {
            environment: environment.to_string(),
            fleet: fleet.to_string(),
        })
}

/// Resolve one explicit non-Removed Root through the Coordinator-anchored install authority.
pub fn resolve_installed_fleet_root_from_root(
    request: &InstalledFleetRequest,
    selected_root: Principal,
    icp_root: &Path,
) -> Result<InstalledFleetRootResolution, InstalledFleetError> {
    let installed = load_installed_fleet_authority(request, icp_root)?;
    let selected = select_current_root(
        installed
            .registry
            .fleet_subnet_roots
            .iter()
            .map(|entry| (entry.fleet_subnet_root, entry.status)),
        selected_root,
    )
    .ok_or_else(|| InstalledFleetError::RootNotInFleet {
        fleet: request.fleet.clone(),
        root: selected_root,
    })?;

    Ok(InstalledFleetRootResolution {
        fleet: installed.fleet,
        root_canister_id: selected,
    })
}

/// Resolve the unique Coordinator through the same verified installed authority.
pub fn resolve_installed_fleet_coordinator_from_root(
    request: &InstalledFleetRequest,
    icp_root: &Path,
) -> Result<InstalledFleetCoordinatorResolution, InstalledFleetError> {
    let installed = load_installed_fleet_authority(request, icp_root)?;
    Ok(InstalledFleetCoordinatorResolution {
        coordinator_canister_id: installed.registry.authority.binding.coordinator,
        fleet: installed.fleet,
    })
}

/// Resolve funding profile, placement cost and current Roots from verified install authority.
pub fn resolve_installed_fleet_funding_from_root(
    request: &InstalledFleetRequest,
    icp_root: &Path,
) -> Result<InstalledFleetFundingResolution, InstalledFleetError> {
    let installed = load_installed_fleet_authority(request, icp_root)?;
    let mut roots = Vec::with_capacity(installed.registry.fleet_subnet_roots.len());
    for root in &installed.registry.fleet_subnet_roots {
        let mut placements = installed
            .plan
            .fleet_subnet_roots
            .iter()
            .filter(|planned| planned.placement_subnet == root.placement_subnet);
        let planned = placements.next().ok_or_else(|| {
            InstalledFleetError::InstalledAuthority(format!(
                "Registry Root {} has no exact planned placement {}",
                root.fleet_subnet_root, root.placement_subnet
            ))
        })?;
        if placements.next().is_some() || planned.funding != root.funding {
            return Err(InstalledFleetError::InstalledAuthority(format!(
                "Registry Root {} conflicts with planned placement funding authority",
                root.fleet_subnet_root
            )));
        }
        roots.push(InstalledFleetRootFundingResolution {
            fleet_subnet_root: root.fleet_subnet_root,
            placement_subnet: root.placement_subnet,
            status: root.status,
            funding: root.funding.clone(),
            placement_cost: planned.placement_cost.clone(),
        });
    }
    Ok(InstalledFleetFundingResolution {
        coordinator_canister_id: installed.registry.authority.binding.coordinator,
        coordinator_root_funding: installed.plan.coordinator.root_funding.clone(),
        coordinator_placement_cost: installed.plan.coordinator.placement_cost.clone(),
        roots,
        fleet: installed.fleet,
    })
}

fn load_installed_fleet_authority(
    request: &InstalledFleetRequest,
    icp_root: &Path,
) -> Result<InstalledFleetAuthority, InstalledFleetError> {
    let fleet = read_installed_fleet_from_root(&request.environment, &request.fleet, icp_root)?;
    let choices = discover_workspace_canic_config_choices(icp_root)
        .map_err(|error| InstalledFleetError::InstalledAuthority(error.to_string()))?;
    let config_path = select_discovered_app_config_path(&choices, fleet.app.as_str())
        .map_err(|error| InstalledFleetError::InstalledAuthority(error.to_string()))?
        .ok_or_else(|| {
            InstalledFleetError::InstalledAuthority(format!(
                "no discovered canic.toml declares catalog App {}",
                fleet.app
            ))
        })?;
    let config = AppConfigSnapshot::load(&config_path)
        .map_err(|error| InstalledFleetError::InstalledAuthority(error.to_string()))?;
    let binding = FleetBinding {
        fleet: FleetKey {
            canonical_network_id: fleet.canonical_network_id,
            fleet_id: fleet.fleet_id,
        },
        app: fleet.app.clone(),
    };
    let plan = load_persisted_fleet_install_plan(
        icp_root,
        config.model(),
        &binding,
        fleet.release_build_id,
    )
    .map_err(|error| InstalledFleetError::InstalledAuthority(error.to_string()))?;
    let registry = load_verified_installed_fleet_registry(&plan)
        .map_err(InstalledFleetError::InstalledAuthority)?;
    let catalog_coordinator = fleet
        .coordinator_principal
        .parse::<Principal>()
        .map_err(|error| InstalledFleetError::InstalledAuthority(error.to_string()))?;
    if registry.authority.binding.coordinator != catalog_coordinator {
        return Err(InstalledFleetError::InstalledAuthority(
            "catalog Coordinator differs from verified installed Registry".to_string(),
        ));
    }
    Ok(InstalledFleetAuthority {
        fleet,
        registry,
        plan: plan.plan,
    })
}

fn select_current_root(
    roots: impl IntoIterator<Item = (Principal, FleetSubnetRootStatus)>,
    selected_root: Principal,
) -> Option<Principal> {
    roots
        .into_iter()
        .find(|(root, status)| *root == selected_root && *status != FleetSubnetRootStatus::Removed)
        .map(|(root, _)| root)
}

#[cfg(test)]
mod tests;
