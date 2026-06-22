use crate::{
    icp::{IcpCli, IcpCommandError},
    replica_query::{self, ReplicaQueryError},
};
use std::{error::Error, fmt, path::Path};

const CANIC_SUBNET_REGISTRY_METHOD: &str = "canic_subnet_registry";
const ICP_JSON_OUTPUT: &str = "json";

///
/// SubnetRegistryQuery
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubnetRegistryQuery {
    pub(crate) source: SubnetRegistryQuerySource,
    pub registry_json: String,
}

///
/// SubnetRegistryQuerySource
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SubnetRegistryQuerySource {
    LocalReplica,
    IcpCli,
}

///
/// SubnetRegistryQueryError
///

#[derive(Debug)]
pub enum SubnetRegistryQueryError {
    Replica(ReplicaQueryError),
    Icp(IcpCommandError),
}

impl fmt::Display for SubnetRegistryQueryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Replica(err) => write!(formatter, "{err}"),
            Self::Icp(err) => write!(formatter, "{err}"),
        }
    }
}

impl Error for SubnetRegistryQueryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Replica(err) => Some(err),
            Self::Icp(err) => Some(err),
        }
    }
}

impl From<ReplicaQueryError> for SubnetRegistryQueryError {
    fn from(err: ReplicaQueryError) -> Self {
        Self::Replica(err)
    }
}

impl From<IcpCommandError> for SubnetRegistryQueryError {
    fn from(err: IcpCommandError) -> Self {
        Self::Icp(err)
    }
}

/// Query `canic_subnet_registry`, using the local replica API for local targets.
pub fn query_subnet_registry_json(
    icp: &IcpCli,
    root: &str,
    network: &str,
    icp_root: Option<&Path>,
    candid_path: Option<&Path>,
) -> Result<SubnetRegistryQuery, SubnetRegistryQueryError> {
    if replica_query::should_use_local_replica_query(Some(network)) {
        return query_local_subnet_registry_json(root, network, icp_root);
    }

    let registry_json = icp.canister_query_output_with_candid(
        root,
        CANIC_SUBNET_REGISTRY_METHOD,
        Some(ICP_JSON_OUTPUT),
        candid_path,
    )?;
    Ok(SubnetRegistryQuery {
        source: SubnetRegistryQuerySource::IcpCli,
        registry_json,
    })
}

fn query_local_subnet_registry_json(
    root: &str,
    network: &str,
    icp_root: Option<&Path>,
) -> Result<SubnetRegistryQuery, SubnetRegistryQueryError> {
    let registry_json = icp_root.map_or_else(
        || replica_query::query_subnet_registry_json(Some(network), root),
        |root_path| {
            replica_query::query_subnet_registry_json_from_root(Some(network), root, root_path)
        },
    )?;
    Ok(SubnetRegistryQuery {
        source: SubnetRegistryQuerySource::LocalReplica,
        registry_json,
    })
}
