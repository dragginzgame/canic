//! Module: subnet_registry
//!
//! Responsibility: query one root canister's maintained subnet registry.
//! Does not own: registry state, endpoint DTOs, or deployment discovery policy.
//! Boundary: selects one transport and returns validated canonical host entries.

use crate::{
    icp::{IcpCli, IcpCommandError},
    registry::{RegistryEntry, RegistryParseError, parse_registry_entries},
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
    pub entries: Vec<RegistryEntry>,
}

///
/// SubnetRegistryQuerySource
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SubnetRegistryQuerySource {
    IcpCli,
    LocalReplica,
}

///
/// SubnetRegistryQueryError
///

#[derive(Debug)]
pub enum SubnetRegistryQueryError {
    Icp(IcpCommandError),
    Registry(RegistryParseError),
    Replica(ReplicaQueryError),
}

impl fmt::Display for SubnetRegistryQueryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Icp(err) => write!(formatter, "{err}"),
            Self::Registry(err) => write!(formatter, "{err}"),
            Self::Replica(err) => write!(formatter, "{err}"),
        }
    }
}

impl Error for SubnetRegistryQueryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Icp(err) => Some(err),
            Self::Registry(err) => Some(err),
            Self::Replica(err) => Some(err),
        }
    }
}

impl From<IcpCommandError> for SubnetRegistryQueryError {
    fn from(err: IcpCommandError) -> Self {
        Self::Icp(err)
    }
}

impl From<RegistryParseError> for SubnetRegistryQueryError {
    fn from(err: RegistryParseError) -> Self {
        Self::Registry(err)
    }
}

impl From<ReplicaQueryError> for SubnetRegistryQueryError {
    fn from(err: ReplicaQueryError) -> Self {
        Self::Replica(err)
    }
}

/// Query `canic_subnet_registry`, using the local replica API for local targets.
pub fn query_subnet_registry(
    icp: &IcpCli,
    root: &str,
    network: &str,
    icp_root: Option<&Path>,
    candid_path: Option<&Path>,
) -> Result<SubnetRegistryQuery, SubnetRegistryQueryError> {
    if replica_query::should_use_local_replica_query(Some(network)) {
        return query_local_subnet_registry(root, network, icp_root);
    }

    let output = icp.canister_query_output_with_candid(
        root,
        CANIC_SUBNET_REGISTRY_METHOD,
        Some(ICP_JSON_OUTPUT),
        candid_path,
    )?;
    Ok(SubnetRegistryQuery {
        source: SubnetRegistryQuerySource::IcpCli,
        entries: parse_registry_entries(&output)?,
    })
}

fn query_local_subnet_registry(
    root: &str,
    network: &str,
    icp_root: Option<&Path>,
) -> Result<SubnetRegistryQuery, SubnetRegistryQueryError> {
    let entries = icp_root.map_or_else(
        || replica_query::query_subnet_registry_entries(Some(network), root),
        |root_path| {
            replica_query::query_subnet_registry_entries_from_root(Some(network), root, root_path)
        },
    )?;
    Ok(SubnetRegistryQuery {
        source: SubnetRegistryQuerySource::LocalReplica,
        entries,
    })
}
