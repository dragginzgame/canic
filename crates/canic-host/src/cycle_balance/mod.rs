use crate::{
    icp::{IcpCli, IcpCommandError},
    replica_query,
    response_parse::parse_cycle_balance_response,
};
use canic_core::protocol;
use std::{error::Error, fmt, path::Path};

const ICP_JSON_OUTPUT: &str = "json";

///
/// CycleBalanceQueryError
///

#[derive(Debug)]
pub enum CycleBalanceQueryError {
    Icp(IcpCommandError),
    Parse { canister: String, output: String },
}

impl fmt::Display for CycleBalanceQueryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Icp(err) => write!(formatter, "{err}"),
            Self::Parse { canister, output } => write!(
                formatter,
                "could not parse {canister} {} response: {output}",
                protocol::CANIC_CYCLE_BALANCE
            ),
        }
    }
}

impl Error for CycleBalanceQueryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Icp(err) => Some(err),
            Self::Parse { .. } => None,
        }
    }
}

impl From<IcpCommandError> for CycleBalanceQueryError {
    fn from(err: IcpCommandError) -> Self {
        Self::Icp(err)
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
    parse_cycle_balance_response(&output).ok_or_else(|| CycleBalanceQueryError::Parse {
        canister: canister_id.to_string(),
        output,
    })
}

/// Query `canic_cycle_balance` for reporting paths that treat missing live data as absent.
#[must_use]
pub fn query_cycle_balance_optional(
    icp: &IcpCli,
    canister_id: &str,
    network: &str,
    icp_root: Option<&Path>,
    candid_path: Option<&Path>,
) -> Option<u128> {
    query_cycle_balance(icp, canister_id, network, icp_root, candid_path).ok()
}
