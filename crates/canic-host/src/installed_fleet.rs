use crate::{
    icp::{IcpCli, IcpCommandError},
    install_root::{InstallState, read_named_fleet_install_state},
    registry::{RegistryEntry, RegistryParseError, parse_registry_entries},
    replica_query,
};
use std::collections::BTreeMap;
use thiserror::Error as ThisError;

///
/// InstalledFleetRequest
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstalledFleetRequest {
    pub fleet: String,
    pub network: String,
    pub icp: String,
    pub detect_lost_local_root: bool,
}

///
/// InstalledFleetResolution
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstalledFleetResolution {
    pub source: InstalledFleetSource,
    pub state: InstallState,
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
    #[error("fleet {fleet} is not installed on network {network}")]
    NoInstalledFleet { network: String, fleet: String },

    #[error("failed to read canic fleet state: {0}")]
    InstallState(String),

    #[error("local replica query failed: {0}")]
    ReplicaQuery(String),

    #[error("icp command failed: {command}\n{stderr}")]
    IcpFailed { command: String, stderr: String },

    #[error(
        "fleet {fleet} points to root {root}, but that canister is not present on network {network}"
    )]
    LostLocalFleet {
        fleet: String,
        network: String,
        root: String,
    },

    #[error(transparent)]
    Registry(#[from] RegistryParseError),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub fn resolve_installed_fleet(
    request: &InstalledFleetRequest,
) -> Result<InstalledFleetResolution, InstalledFleetError> {
    let state = read_installed_fleet_state(&request.network, &request.fleet)?;
    let (source, registry_json) = query_registry(request, &state.root_canister_id)?;
    let entries = parse_registry_entries(&registry_json)?;
    let registry = InstalledFleetRegistry {
        root_canister_id: state.root_canister_id.clone(),
        entries,
    };
    let topology = ResolvedFleetTopology::from_registry(&registry);
    Ok(InstalledFleetResolution {
        source,
        state,
        registry,
        topology,
    })
}

pub fn read_installed_fleet_state(
    network: &str,
    fleet: &str,
) -> Result<InstallState, InstalledFleetError> {
    read_named_fleet_install_state(network, fleet)
        .map_err(|err| InstalledFleetError::InstallState(err.to_string()))?
        .ok_or_else(|| InstalledFleetError::NoInstalledFleet {
            network: network.to_string(),
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

fn query_registry(
    request: &InstalledFleetRequest,
    root: &str,
) -> Result<(InstalledFleetSource, String), InstalledFleetError> {
    if replica_query::should_use_local_replica_query(Some(&request.network)) {
        return replica_query::query_subnet_registry_json(Some(&request.network), root)
            .map(|registry| (InstalledFleetSource::LocalReplica, registry))
            .map_err(|err| local_registry_error(request, root, err.to_string()));
    }

    IcpCli::new(&request.icp, None, Some(request.network.clone()))
        .canister_call_output(root, "canic_subnet_registry", Some("json"))
        .map(|registry| (InstalledFleetSource::IcpCli, registry))
        .map_err(installed_fleet_icp_error)
}

fn local_registry_error(
    request: &InstalledFleetRequest,
    root: &str,
    error: String,
) -> InstalledFleetError {
    if request.detect_lost_local_root && is_canister_not_found_error(&error) {
        return InstalledFleetError::LostLocalFleet {
            fleet: request.fleet.clone(),
            network: request.network.clone(),
            root: root.to_string(),
        };
    }
    InstalledFleetError::ReplicaQuery(error)
}

fn is_canister_not_found_error(error: &str) -> bool {
    error.contains("Canister ") && error.contains(" not found")
}

fn installed_fleet_icp_error(error: IcpCommandError) -> InstalledFleetError {
    match error {
        IcpCommandError::Io(err) => InstalledFleetError::Io(err),
        IcpCommandError::Failed { command, stderr } => {
            InstalledFleetError::IcpFailed { command, stderr }
        }
        IcpCommandError::SnapshotIdUnavailable { output } => InstalledFleetError::IcpFailed {
            command: "icp canister snapshot create".to_string(),
            stderr: output,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Ensure the resolved topology gives command code parent/role projections without reparsing.
    #[test]
    fn topology_indexes_registry_entries() {
        let registry = InstalledFleetRegistry {
            root_canister_id: "root-id".to_string(),
            entries: vec![
                RegistryEntry {
                    pid: "child-b".to_string(),
                    role: Some("worker".to_string()),
                    kind: None,
                    parent_pid: Some("root-id".to_string()),
                    module_hash: None,
                },
                RegistryEntry {
                    pid: "root-id".to_string(),
                    role: Some("root".to_string()),
                    kind: None,
                    parent_pid: None,
                    module_hash: None,
                },
                RegistryEntry {
                    pid: "child-a".to_string(),
                    role: Some("app".to_string()),
                    kind: None,
                    parent_pid: Some("root-id".to_string()),
                    module_hash: None,
                },
            ],
        };

        let topology = ResolvedFleetTopology::from_registry(&registry);

        assert_eq!(
            topology
                .children_by_parent
                .get(&Some("root-id".to_string())),
            Some(&vec!["child-a".to_string(), "child-b".to_string()])
        );
        assert_eq!(topology.roles_by_canister["child-a"], "app");
        assert_eq!(topology.root_canister_id, "root-id");
    }

    // Ensure local replica missing-canister errors are recognized for lost fleet guidance.
    #[test]
    fn detects_local_canister_not_found_error() {
        assert!(is_canister_not_found_error(
            "local replica rejected query: code=3 message=Canister uxrrr-q7777-77774-qaaaq-cai not found"
        ));
        assert!(!is_canister_not_found_error(
            "local replica rejected query: code=5 message=some other failure"
        ));
    }
}
