use crate::{
    icp::{IcpCli, IcpCommandError, existing_local_canister_candid_path},
    install_root::{
        InstallState, read_named_deployment_install_state,
        read_named_deployment_install_state_from_root,
    },
    registry::{RegistryEntry, RegistryParseError, parse_registry_entries},
    replica_query,
};
use std::{collections::BTreeMap, path::Path};
use thiserror::Error as ThisError;

///
/// InstalledDeploymentRequest
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstalledDeploymentRequest {
    pub deployment: String,
    pub network: String,
    pub icp: String,
    pub detect_lost_local_root: bool,
}

///
/// InstalledDeploymentResolution
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstalledDeploymentResolution {
    pub source: InstalledDeploymentSource,
    pub state: InstallState,
    pub registry: InstalledDeploymentRegistry,
    pub topology: ResolvedDeploymentTopology,
}

///
/// InstalledDeploymentSource
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InstalledDeploymentSource {
    LocalReplica,
    IcpCli,
}

///
/// InstalledDeploymentRegistry
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstalledDeploymentRegistry {
    pub root_canister_id: String,
    pub entries: Vec<RegistryEntry>,
}

///
/// ResolvedDeploymentTopology
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedDeploymentTopology {
    pub root_canister_id: String,
    pub children_by_parent: BTreeMap<Option<String>, Vec<String>>,
    pub roles_by_canister: BTreeMap<String, String>,
}

///
/// InstalledDeploymentError
///

#[derive(Debug, ThisError)]
pub enum InstalledDeploymentError {
    #[error("deployment target {deployment} is not installed on network {network}")]
    NoInstalledDeployment { network: String, deployment: String },

    #[error("failed to read canic deployment state: {0}")]
    InstallState(String),

    #[error("local replica query failed: {0}")]
    ReplicaQuery(String),

    #[error("icp command failed: {command}\n{stderr}")]
    IcpFailed { command: String, stderr: String },

    #[error(
        "deployment target {deployment} points to root {root}, but that canister is not present on network {network}"
    )]
    LostLocalDeployment {
        deployment: String,
        network: String,
        root: String,
    },

    #[error(transparent)]
    Registry(#[from] RegistryParseError),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub fn resolve_installed_deployment(
    request: &InstalledDeploymentRequest,
) -> Result<InstalledDeploymentResolution, InstalledDeploymentError> {
    let state = read_installed_deployment_state(&request.network, &request.deployment)?;
    let (source, registry_json) = query_registry(request, &state.root_canister_id)?;
    installed_deployment_resolution(state, source, registry_json)
}

pub fn resolve_installed_deployment_from_root(
    request: &InstalledDeploymentRequest,
    icp_root: &Path,
) -> Result<InstalledDeploymentResolution, InstalledDeploymentError> {
    let state =
        read_installed_deployment_state_from_root(&request.network, &request.deployment, icp_root)?;
    let (source, registry_json) =
        query_registry_from_root(request, &state.root_canister_id, icp_root)?;
    installed_deployment_resolution(state, source, registry_json)
}

fn installed_deployment_resolution(
    state: InstallState,
    source: InstalledDeploymentSource,
    registry_json: String,
) -> Result<InstalledDeploymentResolution, InstalledDeploymentError> {
    let entries = parse_registry_entries(&registry_json)?;
    let registry = InstalledDeploymentRegistry {
        root_canister_id: state.root_canister_id.clone(),
        entries,
    };
    let topology = ResolvedDeploymentTopology::from_registry(&registry);
    Ok(InstalledDeploymentResolution {
        source,
        state,
        registry,
        topology,
    })
}

pub fn read_installed_deployment_state(
    network: &str,
    deployment: &str,
) -> Result<InstallState, InstalledDeploymentError> {
    read_named_deployment_install_state(network, deployment)
        .map_err(|err| InstalledDeploymentError::InstallState(err.to_string()))?
        .ok_or_else(|| InstalledDeploymentError::NoInstalledDeployment {
            network: network.to_string(),
            deployment: deployment.to_string(),
        })
}

pub fn read_installed_deployment_state_from_root(
    network: &str,
    deployment: &str,
    icp_root: &Path,
) -> Result<InstallState, InstalledDeploymentError> {
    read_named_deployment_install_state_from_root(icp_root, network, deployment)
        .map_err(|err| InstalledDeploymentError::InstallState(err.to_string()))?
        .ok_or_else(|| InstalledDeploymentError::NoInstalledDeployment {
            network: network.to_string(),
            deployment: deployment.to_string(),
        })
}

impl ResolvedDeploymentTopology {
    fn from_registry(registry: &InstalledDeploymentRegistry) -> Self {
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
    request: &InstalledDeploymentRequest,
    root: &str,
) -> Result<(InstalledDeploymentSource, String), InstalledDeploymentError> {
    if replica_query::should_use_local_replica_query(Some(&request.network)) {
        return replica_query::query_subnet_registry_json(Some(&request.network), root)
            .map(|registry| (InstalledDeploymentSource::LocalReplica, registry))
            .map_err(|err| local_registry_error(request, root, err.to_string()));
    }

    IcpCli::new(&request.icp, None, Some(request.network.clone()))
        .canister_query_output(root, "canic_subnet_registry", Some("json"))
        .map(|registry| (InstalledDeploymentSource::IcpCli, registry))
        .map_err(installed_deployment_icp_error)
}

fn query_registry_from_root(
    request: &InstalledDeploymentRequest,
    root: &str,
    icp_root: &Path,
) -> Result<(InstalledDeploymentSource, String), InstalledDeploymentError> {
    if replica_query::should_use_local_replica_query(Some(&request.network)) {
        return replica_query::query_subnet_registry_json_from_root(
            Some(&request.network),
            root,
            icp_root,
        )
        .map(|registry| (InstalledDeploymentSource::LocalReplica, registry))
        .map_err(|err| local_registry_error(request, root, err.to_string()));
    }

    IcpCli::new(&request.icp, None, Some(request.network.clone()))
        .with_cwd(icp_root)
        .canister_query_output_with_candid(
            root,
            "canic_subnet_registry",
            Some("json"),
            existing_local_canister_candid_path(icp_root, &request.network, "root").as_deref(),
        )
        .map(|registry| (InstalledDeploymentSource::IcpCli, registry))
        .map_err(installed_deployment_icp_error)
}

fn local_registry_error(
    request: &InstalledDeploymentRequest,
    root: &str,
    error: String,
) -> InstalledDeploymentError {
    if request.detect_lost_local_root && is_canister_not_found_error(&error) {
        return InstalledDeploymentError::LostLocalDeployment {
            deployment: request.deployment.clone(),
            network: request.network.clone(),
            root: root.to_string(),
        };
    }
    InstalledDeploymentError::ReplicaQuery(error)
}

fn is_canister_not_found_error(error: &str) -> bool {
    error.contains("Canister ") && error.contains(" not found")
}

fn installed_deployment_icp_error(error: IcpCommandError) -> InstalledDeploymentError {
    match error {
        IcpCommandError::Io(err) => InstalledDeploymentError::Io(err),
        IcpCommandError::Failed { command, stderr } => {
            InstalledDeploymentError::IcpFailed { command, stderr }
        }
        IcpCommandError::Json {
            command, output, ..
        } => InstalledDeploymentError::IcpFailed {
            command,
            stderr: output,
        },
        error @ (IcpCommandError::MissingCli { .. }
        | IcpCommandError::IncompatibleCliVersion { .. }) => InstalledDeploymentError::IcpFailed {
            command: "icp --version".to_string(),
            stderr: error.to_string(),
        },
        IcpCommandError::SnapshotIdUnavailable { output } => InstalledDeploymentError::IcpFailed {
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
        let registry = InstalledDeploymentRegistry {
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

        let topology = ResolvedDeploymentTopology::from_registry(&registry);

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
