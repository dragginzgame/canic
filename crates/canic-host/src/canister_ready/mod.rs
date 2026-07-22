//! Module: canister_ready
//!
//! Responsibility: query the maintained Canic readiness endpoint.
//! Does not own: readiness state, local replica transport, or install orchestration.
//! Boundary: selects one transport and decodes the canonical boolean response.

use crate::{
    icp::{IcpCli, IcpCommandError, IcpJsonResponseError, decode_json_response},
    replica_query::{self, ReplicaQueryError},
};
use std::path::Path;
use thiserror::Error as ThisError;

const CANIC_READY_METHOD: &str = "canic_ready";
const ICP_JSON_OUTPUT: &str = "json";

///
/// CanisterReadyQueryError
///

#[derive(Debug, ThisError)]
pub enum CanisterReadyQueryError {
    #[error(transparent)]
    Icp(#[from] IcpCommandError),

    #[error(transparent)]
    Replica(#[from] ReplicaQueryError),

    #[error(transparent)]
    Response(#[from] IcpJsonResponseError),
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
