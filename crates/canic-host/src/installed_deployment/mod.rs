use crate::{
    icp::{IcpCli, IcpCommandError, existing_local_canister_candid_path},
    install_root::{
        InstallState, InstallStateError, read_named_deployment_install_state,
        read_named_deployment_install_state_from_root,
    },
    registry::{RegistryEntry, RegistryParseError},
    replica_query::ReplicaQueryError,
    subnet_registry::{SubnetRegistryQueryError, SubnetRegistryQuerySource, query_subnet_registry},
};
use std::{collections::BTreeMap, path::Path};
use thiserror::Error as ThisError;

const IC_REJECT_CODE_DESTINATION_INVALID: u64 = 3;

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
    InstallState(#[from] InstallStateError),

    #[error("local replica query failed: {0}")]
    ReplicaQuery(#[source] ReplicaQueryError),

    #[error(transparent)]
    Icp(#[from] IcpCommandError),

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
    let (source, entries) = query_registry(request, &state.root_canister_id)?;
    Ok(installed_deployment_resolution(state, source, entries))
}

pub fn resolve_installed_deployment_from_root(
    request: &InstalledDeploymentRequest,
    icp_root: &Path,
) -> Result<InstalledDeploymentResolution, InstalledDeploymentError> {
    let state =
        read_installed_deployment_state_from_root(&request.network, &request.deployment, icp_root)?;
    let (source, entries) = query_registry_from_root(request, &state.root_canister_id, icp_root)?;
    Ok(installed_deployment_resolution(state, source, entries))
}

fn installed_deployment_resolution(
    state: InstallState,
    source: InstalledDeploymentSource,
    entries: Vec<RegistryEntry>,
) -> InstalledDeploymentResolution {
    let registry = InstalledDeploymentRegistry {
        root_canister_id: state.root_canister_id.clone(),
        entries,
    };
    let topology = ResolvedDeploymentTopology::from_registry(&registry);
    InstalledDeploymentResolution {
        source,
        state,
        registry,
        topology,
    }
}

pub fn read_installed_deployment_state(
    network: &str,
    deployment: &str,
) -> Result<InstallState, InstalledDeploymentError> {
    read_named_deployment_install_state(network, deployment)
        .map_err(InstalledDeploymentError::InstallState)?
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
        .map_err(InstalledDeploymentError::InstallState)?
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
) -> Result<(InstalledDeploymentSource, Vec<RegistryEntry>), InstalledDeploymentError> {
    let icp = IcpCli::new(&request.icp, Some(request.network.clone()));
    let query = query_subnet_registry(&icp, root, &request.network, None, None)
        .map_err(|err| installed_deployment_registry_error(request, root, err))?;
    Ok((installed_deployment_source(query.source), query.entries))
}

fn query_registry_from_root(
    request: &InstalledDeploymentRequest,
    root: &str,
    icp_root: &Path,
) -> Result<(InstalledDeploymentSource, Vec<RegistryEntry>), InstalledDeploymentError> {
    let icp = IcpCli::new(&request.icp, Some(request.network.clone())).with_cwd(icp_root);
    let candid_path = existing_local_canister_candid_path(icp_root, &request.network, "root");
    let query = query_subnet_registry(
        &icp,
        root,
        &request.network,
        Some(icp_root),
        candid_path.as_deref(),
    )
    .map_err(|err| installed_deployment_registry_error(request, root, err))?;
    Ok((installed_deployment_source(query.source), query.entries))
}

const fn installed_deployment_source(
    source: SubnetRegistryQuerySource,
) -> InstalledDeploymentSource {
    match source {
        SubnetRegistryQuerySource::LocalReplica => InstalledDeploymentSource::LocalReplica,
        SubnetRegistryQuerySource::IcpCli => InstalledDeploymentSource::IcpCli,
    }
}

fn installed_deployment_registry_error(
    request: &InstalledDeploymentRequest,
    root: &str,
    error: SubnetRegistryQueryError,
) -> InstalledDeploymentError {
    match error {
        SubnetRegistryQueryError::Replica(err) => local_registry_error(request, root, err),
        SubnetRegistryQueryError::Icp(err) => InstalledDeploymentError::Icp(err),
        SubnetRegistryQueryError::Registry(err) => InstalledDeploymentError::Registry(err),
    }
}

fn local_registry_error(
    request: &InstalledDeploymentRequest,
    root: &str,
    error: ReplicaQueryError,
) -> InstalledDeploymentError {
    if request.detect_lost_local_root && is_missing_destination_error(&error) {
        return InstalledDeploymentError::LostLocalDeployment {
            deployment: request.deployment.clone(),
            network: request.network.clone(),
            root: root.to_string(),
        };
    }
    InstalledDeploymentError::ReplicaQuery(error)
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
