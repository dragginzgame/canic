//! Module: cycle_balance
//!
//! Responsibility: query the maintained Canic cycle-balance endpoint.
//! Does not own: cycle accounting, local replica transport, or report aggregation.
//! Boundary: decodes the canonical typed endpoint result into a host balance.

#[cfg(test)]
mod tests;

use crate::{
    icp::{IcpCli, IcpCommandError, IcpJsonResponseError, decode_json_result_response},
    icp_config::IcpConfigError,
    replica_query::{self, ReplicaQueryError},
};
use candid::{CandidType, Deserialize};
use canic_core::dto::role::CycleBalanceStatusResponse;
use std::path::Path;
use thiserror::Error as ThisError;

use canic_core::protocol;

const ICP_JSON_OUTPUT: &str = "json";

#[derive(CandidType, Deserialize)]
enum RoleStatusResponse {
    CycleBalance(CycleBalanceStatusResponse),
}

///
/// CycleBalanceQueryError
///

#[derive(Debug, ThisError)]
pub enum CycleBalanceQueryError {
    #[error(transparent)]
    IcpConfig(#[from] IcpConfigError),

    #[error(transparent)]
    Icp(#[from] IcpCommandError),

    #[error(transparent)]
    Replica(#[from] ReplicaQueryError),

    #[error(transparent)]
    Response(#[from] IcpJsonResponseError),
}

/// Query the role-owned cycle-balance status through the selected transport.
pub fn query_cycle_balance(
    icp: &IcpCli,
    canister_id: &str,
    environment: &str,
    icp_root: Option<&Path>,
    candid_path: Option<&Path>,
) -> Result<u128, CycleBalanceQueryError> {
    if replica_query::uses_local_replica_transport(Some(environment), icp_root)? {
        return replica_query::query_cycle_balance(Some(environment), canister_id, icp_root)
            .map_err(Into::into);
    }

    let output = icp.canister_query_arg_output_with_candid(
        canister_id,
        protocol::CANIC_STATUS,
        "(variant { CycleBalance })",
        Some(ICP_JSON_OUTPUT),
        candid_path,
    )?;
    let response = decode_json_result_response::<RoleStatusResponse>(&output)?;
    let RoleStatusResponse::CycleBalance(response) = response;
    Ok(response.cycles)
}
