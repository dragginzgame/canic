//! Module: replica_query
//!
//! Responsibility: query maintained Canic endpoints through a direct local replica transport.
//! Does not own: endpoint DTOs, registry projection, or ICP CLI command execution.
//! Boundary: decodes canonical Candid responses and preserves typed transport and endpoint errors.

mod cbor;
mod status;
mod transport;
mod wire;

use self::{
    transport::local_query,
    wire::{
        decode_bootstrap_status_response, decode_cycle_balance_response,
        decode_subnet_registry_response,
    },
};
use crate::registry::{RegistryEntry, RegistryParseError, registry_entries_from_response};
use std::path::Path;

use candid::Decode;
use canic_core::dto::{error::Error as CanicError, state::BootstrapStatusResponse};
use thiserror::Error as ThisError;

pub use self::status::local_replica_status_reachable_from_root;
pub(crate) use self::{
    status::local_replica_root_key_from_root, transport::local_replica_endpoint_from_root,
};

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

    #[error(transparent)]
    Registry(#[from] RegistryParseError),

    #[error("local replica rejected query: code={code} message={message}")]
    Rejected { code: u64, message: String },
}

impl From<cbor::CborError> for ReplicaQueryError {
    // Convert CBOR encode/decode failures.
    fn from(err: cbor::CborError) -> Self {
        Self::Cbor(err.to_string())
    }
}

/// Return whether the selected environment should use direct local replica queries.
#[must_use]
pub fn should_use_local_replica_query(environment: Option<&str>) -> bool {
    environment
        .is_none_or(|environment| environment == "local" || environment.starts_with("http://"))
}

/// Query `canic_ready` directly through the local replica HTTP API.
pub(crate) fn query_ready(
    environment: Option<&str>,
    canister: &str,
    icp_root: Option<&Path>,
) -> Result<bool, ReplicaQueryError> {
    let bytes = local_query(environment, canister, "canic_ready", icp_root)?;
    Decode!(&bytes, bool).map_err(ReplicaQueryError::Candid)
}

/// Query `canic_bootstrap_status` using the configured port from one ICP root.
pub(crate) fn query_bootstrap_status_from_root(
    environment: Option<&str>,
    canister: &str,
    icp_root: &Path,
) -> Result<BootstrapStatusResponse, ReplicaQueryError> {
    let bytes = local_query(
        environment,
        canister,
        "canic_bootstrap_status",
        Some(icp_root),
    )?;
    decode_bootstrap_status_response(&bytes)
}

/// Query `canic_cycle_balance` directly through the local replica HTTP API.
pub(crate) fn query_cycle_balance(
    environment: Option<&str>,
    canister: &str,
    icp_root: Option<&Path>,
) -> Result<u128, ReplicaQueryError> {
    let bytes = local_query(environment, canister, "canic_cycle_balance", icp_root)?;
    decode_cycle_balance_response(&bytes)
}

/// Query `canic_subnet_registry` and return validated host entries.
pub(crate) fn query_subnet_registry_entries(
    environment: Option<&str>,
    root: &str,
    icp_root: Option<&Path>,
) -> Result<Vec<RegistryEntry>, ReplicaQueryError> {
    let bytes = local_query(environment, root, "canic_subnet_registry", icp_root)?;
    let response = decode_subnet_registry_response(&bytes)?;
    registry_entries_from_response(response).map_err(Into::into)
}

/// Query `canic_subnet_registry` using the configured port from one ICP root and return roles.
pub(crate) fn query_subnet_registry_roles_from_root(
    environment: Option<&str>,
    root: &str,
    icp_root: &Path,
) -> Result<Vec<String>, ReplicaQueryError> {
    Ok(
        query_subnet_registry_entries(environment, root, Some(icp_root))?
            .into_iter()
            .filter_map(|entry| entry.role)
            .collect(),
    )
}
