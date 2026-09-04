//! Module: replica_query
//!
//! Responsibility: query maintained Canic endpoints through a direct local replica transport.
//! Does not own: endpoint DTOs, topology projection, or ICP CLI command execution.
//! Boundary: decodes canonical Candid responses and preserves typed transport and endpoint errors.

mod cbor;
mod status;
#[cfg(test)]
mod tests;
mod transport;

use self::transport::local_query;
use std::path::Path;

use candid::{CandidType, Decode, Deserialize, Encode};
use canic_core::{
    dto::{error::Error as CanicError, role::RoleOverviewResponse},
    ids::BuildNetwork,
};
use thiserror::Error as ThisError;

use crate::icp_config::{
    IcpConfigError, resolve_current_canic_icp_root, resolve_icp_build_network_from_root,
};

pub use self::status::local_replica_status_reachable_from_root;

#[derive(CandidType)]
enum RoleStatusRequest {
    Overview,
}

#[derive(CandidType, Deserialize)]
enum RoleStatusResponse {
    Overview(RoleOverviewResponse),
}

///
/// ReplicaQueryError
///

#[derive(Debug, ThisError)]
pub enum ReplicaQueryError {
    #[error(transparent)]
    Candid(candid::Error),

    #[error("{0}")]
    Canister(CanicError),

    #[error("{0}")]
    Cbor(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Query(String),

    #[error("local replica rejected query: code={code} message={message}")]
    Rejected { code: u64, message: String },
}

impl From<cbor::CborError> for ReplicaQueryError {
    // Convert CBOR encode/decode failures.
    fn from(err: cbor::CborError) -> Self {
        Self::Cbor(err.to_string())
    }
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
    let arg = Encode!(&RoleStatusRequest::Overview).map_err(ReplicaQueryError::Candid)?;
    let bytes = local_query(environment, canister, endpoint, &arg, icp_root)?;
    let result = Decode!(&bytes, Result<RoleStatusResponse, CanicError>)
        .map_err(ReplicaQueryError::Candid)?;
    let RoleStatusResponse::Overview(response) = result.map_err(ReplicaQueryError::Canister)?;
    Ok(response.bootstrap.ready)
}
