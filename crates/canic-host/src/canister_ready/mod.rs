use crate::{
    icp::{IcpCli, IcpCommandError},
    replica_query::{self, ReplicaQueryError},
};
use std::{error::Error, fmt, path::Path};

const CANIC_READY_METHOD: &str = "canic_ready";
const ICP_JSON_OUTPUT: &str = "json";

///
/// CanisterReadyQueryError
///

#[derive(Debug)]
pub enum CanisterReadyQueryError {
    Replica(ReplicaQueryError),
    Icp(IcpCommandError),
    Json(serde_json::Error),
}

impl fmt::Display for CanisterReadyQueryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Replica(err) => write!(formatter, "{err}"),
            Self::Icp(err) => write!(formatter, "{err}"),
            Self::Json(err) => write!(formatter, "{err}"),
        }
    }
}

impl Error for CanisterReadyQueryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Replica(err) => Some(err),
            Self::Icp(err) => Some(err),
            Self::Json(err) => Some(err),
        }
    }
}

impl From<ReplicaQueryError> for CanisterReadyQueryError {
    fn from(err: ReplicaQueryError) -> Self {
        Self::Replica(err)
    }
}

impl From<IcpCommandError> for CanisterReadyQueryError {
    fn from(err: IcpCommandError) -> Self {
        Self::Icp(err)
    }
}

impl From<serde_json::Error> for CanisterReadyQueryError {
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err)
    }
}

/// Query `canic_ready`, using the local replica API for local network targets.
pub fn query_canister_ready(
    icp: &IcpCli,
    canister_id: &str,
    network: &str,
    icp_root: Option<&Path>,
    candid_path: Option<&Path>,
) -> Result<bool, CanisterReadyQueryError> {
    if replica_query::should_use_local_replica_query(Some(network)) {
        return query_local_canister_ready(network, canister_id, icp_root).map_err(Into::into);
    }

    query_canister_ready_with_icp(icp, canister_id, candid_path)
}

/// Query `canic_ready` directly through the local replica API.
pub fn query_local_canister_ready(
    network: &str,
    canister_id: &str,
    icp_root: Option<&Path>,
) -> Result<bool, ReplicaQueryError> {
    icp_root.map_or_else(
        || replica_query::query_ready(Some(network), canister_id),
        |root| replica_query::query_ready_from_root(Some(network), canister_id, root),
    )
}

fn query_canister_ready_with_icp(
    icp: &IcpCli,
    canister_id: &str,
    candid_path: Option<&Path>,
) -> Result<bool, CanisterReadyQueryError> {
    let output = icp.canister_query_output_with_candid(
        canister_id,
        CANIC_READY_METHOD,
        Some(ICP_JSON_OUTPUT),
        candid_path,
    )?;
    let data = serde_json::from_str::<serde_json::Value>(&output)?;
    Ok(replica_query::parse_ready_json_value(&data))
}
