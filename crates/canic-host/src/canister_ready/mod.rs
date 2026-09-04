//! Module: canister_ready
//!
//! Responsibility: query the maintained Canic readiness endpoint.
//! Does not own: readiness state, local replica transport, or install orchestration.
//! Boundary: selects one transport and decodes the canonical boolean response.

use crate::{
    icp::{IcpCli, IcpCommandError, IcpJsonResponseError, decode_json_result_response},
    icp_config::IcpConfigError,
    protocol_binding::ResolvedProtocolBinding,
    replica_query::{self, ReplicaQueryError},
};
use candid::{CandidType, Deserialize};
use canic_core::{dto::role::RoleOverviewResponse, protocol::status_endpoint_for_role};
use std::path::Path;
use thiserror::Error as ThisError;

const ICP_JSON_OUTPUT: &str = "json";

#[derive(CandidType, Deserialize)]
enum RoleStatusResponse {
    Overview(RoleOverviewResponse),
}

///
/// CanisterReadyQueryError
///

#[derive(Debug, ThisError)]
pub enum CanisterReadyQueryError {
    #[error(transparent)]
    IcpConfig(#[from] IcpConfigError),

    #[error(transparent)]
    Icp(#[from] IcpCommandError),

    #[error(transparent)]
    Replica(#[from] ReplicaQueryError),

    #[error(transparent)]
    Response(#[from] IcpJsonResponseError),
}

/// Query role-owned readiness, using the local replica API for local targets.
pub fn query_canister_ready(
    icp: &IcpCli,
    canister_id: &str,
    environment: &str,
    icp_root: Option<&Path>,
    binding: &ResolvedProtocolBinding,
) -> Result<bool, CanisterReadyQueryError> {
    if replica_query::uses_local_replica_transport(Some(environment), icp_root)? {
        return query_local_canister_ready(environment, canister_id, icp_root, binding)
            .map_err(Into::into);
    }

    query_canister_ready_with_icp(icp, canister_id, binding)
}

/// Query role-owned readiness directly through the local replica API.
pub fn query_local_canister_ready(
    environment: &str,
    canister_id: &str,
    icp_root: Option<&Path>,
    binding: &ResolvedProtocolBinding,
) -> Result<bool, ReplicaQueryError> {
    replica_query::query_ready(
        Some(environment),
        canister_id,
        status_endpoint_for_role(&binding.binding().role),
        icp_root,
    )
}

fn query_canister_ready_with_icp(
    icp: &IcpCli,
    canister_id: &str,
    binding: &ResolvedProtocolBinding,
) -> Result<bool, CanisterReadyQueryError> {
    let output = icp.canister_query_arg_output_with_candid(
        canister_id,
        status_endpoint_for_role(&binding.binding().role),
        "(variant { Overview })",
        Some(ICP_JSON_OUTPUT),
        Some(&binding.candid_path),
    )?;
    let response = decode_json_result_response::<RoleStatusResponse>(&output)?;
    let RoleStatusResponse::Overview(response) = response;
    Ok(response.bootstrap.ready)
}
