//! Module: cycle_balance
//!
//! Responsibility: query the maintained Canic cycle-balance endpoint.
//! Does not own: cycle accounting, local replica transport, or report aggregation.
//! Boundary: decodes the canonical typed endpoint result into a host balance.

#[cfg(test)]
mod tests;

use crate::{
    icp::{IcpCli, IcpCommandError, IcpJsonResponseError, decode_json_result_response},
    replica_query::{self, ReplicaQueryError},
};
use std::path::Path;
use thiserror::Error as ThisError;

use canic_core::protocol;

const ICP_JSON_OUTPUT: &str = "json";

///
/// CycleBalanceQueryError
///

#[derive(Debug, ThisError)]
pub enum CycleBalanceQueryError {
    #[error(transparent)]
    Icp(#[from] IcpCommandError),

    #[error(transparent)]
    Replica(#[from] ReplicaQueryError),

    #[error(transparent)]
    Response(#[from] IcpJsonResponseError),
}

/// Query `canic_cycle_balance` through the transport selected by the network.
pub fn query_cycle_balance(
    icp: &IcpCli,
    canister_id: &str,
    network: &str,
    icp_root: Option<&Path>,
    candid_path: Option<&Path>,
) -> Result<u128, CycleBalanceQueryError> {
    if replica_query::should_use_local_replica_query(Some(network)) {
        return query_local_cycle_balance(network, canister_id, icp_root).map_err(Into::into);
    }

    let output = icp.canister_query_output_with_candid(
        canister_id,
        protocol::CANIC_CYCLE_BALANCE,
        Some(ICP_JSON_OUTPUT),
        candid_path,
    )?;
    decode_json_result_response(&output).map_err(Into::into)
}

fn query_local_cycle_balance(
    network: &str,
    canister_id: &str,
    icp_root: Option<&Path>,
) -> Result<u128, ReplicaQueryError> {
    icp_root.map_or_else(
        || replica_query::query_cycle_balance(Some(network), canister_id),
        |root| replica_query::query_cycle_balance_from_root(Some(network), canister_id, root),
    )
}
