//! Module: cycle_balance
//!
//! Responsibility: query the maintained Canic cycle-balance endpoint.
//! Does not own: cycle accounting, local replica transport, or report aggregation.
//! Boundary: decodes the canonical typed endpoint result into a host balance.

use crate::{
    icp::{IcpCli, IcpCommandError, IcpJsonResponseError, decode_json_result_response},
    replica_query,
};
use std::{error::Error, fmt, path::Path};

use canic_core::protocol;

const ICP_JSON_OUTPUT: &str = "json";

///
/// CycleBalanceQueryError
///

#[derive(Debug)]
pub enum CycleBalanceQueryError {
    Icp(IcpCommandError),
    Response(IcpJsonResponseError),
}

impl fmt::Display for CycleBalanceQueryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Icp(err) => write!(formatter, "{err}"),
            Self::Response(err) => write!(formatter, "{err}"),
        }
    }
}

impl Error for CycleBalanceQueryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Icp(err) => Some(err),
            Self::Response(err) => Some(err),
        }
    }
}

impl From<IcpCommandError> for CycleBalanceQueryError {
    fn from(err: IcpCommandError) -> Self {
        Self::Icp(err)
    }
}

impl From<IcpJsonResponseError> for CycleBalanceQueryError {
    fn from(err: IcpJsonResponseError) -> Self {
        Self::Response(err)
    }
}

/// Query `canic_cycle_balance`, using direct local replica calls when available.
pub fn query_cycle_balance(
    icp: &IcpCli,
    canister_id: &str,
    network: &str,
    icp_root: Option<&Path>,
    candid_path: Option<&Path>,
) -> Result<u128, CycleBalanceQueryError> {
    if replica_query::should_use_local_replica_query(Some(network))
        && let Some(root) = icp_root
        && let Ok(cycles) =
            replica_query::query_cycle_balance_from_root(Some(network), canister_id, root)
    {
        return Ok(cycles);
    }

    let output = icp.canister_query_output_with_candid(
        canister_id,
        protocol::CANIC_CYCLE_BALANCE,
        Some(ICP_JSON_OUTPUT),
        candid_path,
    )?;
    decode_json_result_response(&output).map_err(Into::into)
}
