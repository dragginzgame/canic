use crate::{
    fleet_catalog::{FleetCatalogEntryV1, FleetCatalogError, read_fleet_catalog_entry_from_root},
    icp::{IcpCli, IcpCommandError, existing_local_canister_candid_path},
    registry::{RegistryEntry, RegistryParseError},
    replica_query::ReplicaQueryError,
    subnet_registry::{SubnetRegistryQueryError, SubnetRegistryQuerySource, query_subnet_registry},
};
use std::{collections::BTreeMap, path::Path};
use thiserror::Error as ThisError;

const IC_REJECT_CODE_DESTINATION_INVALID: u64 = 3;

///
/// InstalledFleetRequest
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstalledFleetRequest {
    pub fleet: String,
    pub environment: String,
    pub icp: String,
    pub detect_lost_local_root: bool,
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

    #[error("local replica query failed: {0}")]
    ReplicaQuery(#[source] ReplicaQueryError),

    #[error(transparent)]
    Icp(#[from] IcpCommandError),

    #[error(
        "Fleet {fleet} points to root {root}, but that canister is not present on environment {environment}"
    )]
    LostLocalFleet {
        fleet: String,
        environment: String,
        root: String,
    },

    #[error(transparent)]
    Registry(#[from] RegistryParseError),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub fn resolve_installed_fleet_from_root(
    request: &InstalledFleetRequest,
    icp_root: &Path,
) -> Result<InstalledFleetResolution, InstalledFleetError> {
    let fleet = read_installed_fleet_from_root(&request.environment, &request.fleet, icp_root)?;
    let (source, entries) = query_registry_from_root(request, &fleet.root_principal, icp_root)?;
    Ok(installed_fleet_resolution(fleet, source, entries))
}

fn installed_fleet_resolution(
    fleet: FleetCatalogEntryV1,
    source: InstalledFleetSource,
    entries: Vec<RegistryEntry>,
) -> InstalledFleetResolution {
    let registry = InstalledFleetRegistry {
        root_canister_id: fleet.root_principal.clone(),
        entries,
    };
    let topology = ResolvedFleetTopology::from_registry(&registry);
    InstalledFleetResolution {
        source,
        fleet,
        registry,
        topology,
    }
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

impl ResolvedFleetTopology {
    fn from_registry(registry: &InstalledFleetRegistry) -> Self {
        let mut children_by_parent = BTreeMap::<Option<String>, Vec<String>>::new();
        let mut roles_by_canister = BTreeMap::new();
        for entry in &registry.entries {
            children_by_parent
                .entry(entry.parent_pid.clone())
                .or_default()
                .push(entry.pid.clone());
            if let Some(role) = &entry.role {
                roles_by_canister.insert(entry.pid.clone(), role.clone());
            }
        }
        for children in children_by_parent.values_mut() {
            children.sort();
        }
        Self {
            root_canister_id: registry.root_canister_id.clone(),
            children_by_parent,
            roles_by_canister,
        }
    }
}

fn query_registry_from_root(
    request: &InstalledFleetRequest,
    root: &str,
    icp_root: &Path,
) -> Result<(InstalledFleetSource, Vec<RegistryEntry>), InstalledFleetError> {
    let icp = IcpCli::new(&request.icp, Some(request.environment.clone())).with_cwd(icp_root);
    let candid_path = existing_local_canister_candid_path(icp_root, &request.environment, "root");
    let query = query_subnet_registry(
        &icp,
        root,
        &request.environment,
        Some(icp_root),
        candid_path.as_deref(),
    )
    .map_err(|err| installed_fleet_registry_error(request, root, err))?;
    Ok((installed_fleet_source(query.source), query.entries))
}

const fn installed_fleet_source(source: SubnetRegistryQuerySource) -> InstalledFleetSource {
    match source {
        SubnetRegistryQuerySource::LocalReplica => InstalledFleetSource::LocalReplica,
        SubnetRegistryQuerySource::IcpCli => InstalledFleetSource::IcpCli,
    }
}

fn installed_fleet_registry_error(
    request: &InstalledFleetRequest,
    root: &str,
    error: SubnetRegistryQueryError,
) -> InstalledFleetError {
    match error {
        SubnetRegistryQueryError::Replica(err) => local_registry_error(request, root, err),
        SubnetRegistryQueryError::Icp(err) => InstalledFleetError::Icp(err),
        SubnetRegistryQueryError::Registry(err) => InstalledFleetError::Registry(err),
    }
}

fn local_registry_error(
    request: &InstalledFleetRequest,
    root: &str,
    error: ReplicaQueryError,
) -> InstalledFleetError {
    if request.detect_lost_local_root && is_missing_destination_error(&error) {
        return InstalledFleetError::LostLocalFleet {
            fleet: request.fleet.clone(),
            environment: request.environment.clone(),
            root: root.to_string(),
        };
    }
    InstalledFleetError::ReplicaQuery(error)
}

const fn is_missing_destination_error(error: &ReplicaQueryError) -> bool {
    matches!(
        error,
        ReplicaQueryError::Rejected {
            code: IC_REJECT_CODE_DESTINATION_INVALID,
            ..
        }
    )
}

#[cfg(test)]
mod tests;
