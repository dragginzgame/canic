use crate::{
    fleet_catalog::{FleetCatalogEntryV1, FleetCatalogError, read_fleet_catalog_entry_from_root},
    registry::RegistryEntry,
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

#[cfg(test)]
mod tests;
