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
use std::path::Path;
use thiserror::Error as ThisError;

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

#[derive(Debug, ThisError)]
pub enum SubnetRegistryQueryError {
    #[error(transparent)]
    Icp(#[from] IcpCommandError),

    #[error(transparent)]
    Registry(#[from] RegistryParseError),

    #[error(transparent)]
    Replica(#[from] ReplicaQueryError),
}

/// Query `canic_subnet_registry`, using the local replica API for local targets.
pub fn query_subnet_registry(
    icp: &IcpCli,
    root: &str,
    environment: &str,
    icp_root: Option<&Path>,
    candid_path: Option<&Path>,
) -> Result<SubnetRegistryQuery, SubnetRegistryQueryError> {
    if replica_query::should_use_local_replica_query(Some(environment)) {
        return query_local_subnet_registry(root, environment, icp_root);
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
    environment: &str,
    icp_root: Option<&Path>,
) -> Result<SubnetRegistryQuery, SubnetRegistryQueryError> {
    let entries = replica_query::query_subnet_registry_entries(Some(environment), root, icp_root)?;
    Ok(SubnetRegistryQuery {
        source: SubnetRegistryQuerySource::LocalReplica,
        entries,
    })
}
