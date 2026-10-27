//! Module: replica_query
//!
//! Responsibility: query maintained Canic endpoints through a direct local replica transport.
//! Does not own: endpoint DTOs, topology projection, or ICP CLI command execution.
//! Boundary: decodes canonical Candid responses and preserves typed transport and endpoint errors.

mod status;
#[cfg(test)]
mod tests;
mod transport;

use self::transport::local_query;
use crate::icp_config::{
    IcpConfigError, resolve_current_canic_icp_root, resolve_icp_build_network_from_root,
};
use canic_contracts::protocol::public_overview;
use canic_contracts::{
    dto::{error::Error as CanicError, role::RoleOverviewResponse},
    ids::BuildNetwork,
};
use std::path::Path;
use thiserror::Error as ThisError;

pub use self::status::{local_replica_gateway_listening, local_replica_status_reachable_from_root};

///
/// ReplicaQueryError
///

#[derive(Debug, ThisError)]
pub enum ReplicaQueryError {
    #[error(transparent)]
    Candid(ic_platform::candid::contract::Error),

    #[error(transparent)]
    Wire(#[from] ic_wire::Error),

    #[error("{0}")]
    Canister(CanicError),

    #[error(transparent)]
    Agent(Box<ic_platform::agent::AgentError>),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Query(String),

    #[error("local replica rejected query: code={code} message={message}")]
    Rejected { code: u64, message: String },
}

/// Resolve whether the selected environment uses the direct replica transport.
pub fn uses_local_replica_transport(
    environment: Option<&str>,
    icp_root: Option<&Path>,
) -> Result<bool, IcpConfigError> {
    let Some(environment) = environment else {
        return Ok(true);
    };
    if environment.starts_with("http://") {
        return Ok(true);
    }

    let discovered_root;
    let root = if let Some(root) = icp_root {
        root
    } else {
        discovered_root = resolve_current_canic_icp_root()?;
        &discovered_root
    };
    Ok(resolve_icp_build_network_from_root(root, environment)? == BuildNetwork::Local)
}

/// Query role-owned readiness directly through the local replica HTTP API.
pub(crate) fn query_ready(
    environment: Option<&str>,
    canister: &str,
    endpoint: &str,
    icp_root: Option<&Path>,
) -> Result<bool, ReplicaQueryError> {
    let arg = ic_wire::native::encode_request(
        public_overview::METHOD_ID,
        &(),
        public_overview::REQUEST_LIMITS,
    )?;
    let bytes = local_query(environment, canister, endpoint, &arg, icp_root)?;
    let result: Result<RoleOverviewResponse, CanicError> = ic_wire::native::decode_response(
        public_overview::METHOD_ID,
        &bytes,
        public_overview::RESPONSE_LIMITS,
    )?;
    Ok(result.map_err(ReplicaQueryError::Canister)?.bootstrap.ready)
}
