//! Module: canister_ready
//!
//! Responsibility: query the maintained Canic readiness endpoint.
//! Does not own: readiness state, local replica transport, or install orchestration.
//! Boundary: selects one transport and decodes the canonical boolean response.

use crate::{
    icp::{IcpCli, IcpCommandError, IcpJsonResponseError, decode_json_response},
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
    Icp(IcpCommandError),
    Replica(ReplicaQueryError),
    Response(IcpJsonResponseError),
}

impl fmt::Display for CanisterReadyQueryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Icp(err) => write!(formatter, "{err}"),
            Self::Replica(err) => write!(formatter, "{err}"),
            Self::Response(err) => write!(formatter, "{err}"),
        }
    }
}

impl Error for CanisterReadyQueryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Icp(err) => Some(err),
            Self::Replica(err) => Some(err),
            Self::Response(err) => Some(err),
        }
    }
}

impl From<IcpCommandError> for CanisterReadyQueryError {
    fn from(err: IcpCommandError) -> Self {
        Self::Icp(err)
    }
}

impl From<ReplicaQueryError> for CanisterReadyQueryError {
    fn from(err: ReplicaQueryError) -> Self {
        Self::Replica(err)
    }
}

impl From<IcpJsonResponseError> for CanisterReadyQueryError {
    fn from(err: IcpJsonResponseError) -> Self {
        Self::Response(err)
    }
}

/// Query `canic_ready`, using the local replica API for local environment targets.
pub fn query_canister_ready(
    icp: &IcpCli,
    canister_id: &str,
    environment: &str,
    icp_root: Option<&Path>,
    candid_path: Option<&Path>,
) -> Result<bool, CanisterReadyQueryError> {
    if replica_query::should_use_local_replica_query(Some(environment)) {
        return query_local_canister_ready(environment, canister_id, icp_root).map_err(Into::into);
    }

    query_canister_ready_with_icp(icp, canister_id, candid_path)
}

/// Query `canic_ready` directly through the local replica API.
pub fn query_local_canister_ready(
    environment: &str,
    canister_id: &str,
    icp_root: Option<&Path>,
) -> Result<bool, ReplicaQueryError> {
    replica_query::query_ready(Some(environment), canister_id, icp_root)
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
    decode_json_response(&output).map_err(Into::into)
}
