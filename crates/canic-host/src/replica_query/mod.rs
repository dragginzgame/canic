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
    transport::{local_query, local_query_from_root},
    wire::{
        decode_bootstrap_status_response, decode_cycle_balance_response,
        decode_subnet_registry_response,
    },
};
use crate::registry::{RegistryEntry, RegistryParseError, registry_entries_from_response};
use std::{error::Error, fmt, path::Path};

use candid::Decode;
use canic_core::dto::{
    error::Error as CanicError, state::BootstrapStatusResponse, topology::SubnetRegistryResponse,
};

pub use self::{
    status::{local_replica_root_key_from_root, local_replica_status_reachable_from_root},
    transport::local_replica_endpoint_from_root,
};

///
/// ReplicaQueryError
///

#[derive(Debug)]
pub enum ReplicaQueryError {
    Candid(candid::Error),
    Canister(CanicError),
    Cbor(String),
    Io(std::io::Error),
    Query(String),
    Registry(RegistryParseError),
    Rejected { code: u64, message: String },
}

impl fmt::Display for ReplicaQueryError {
    // Render local replica query failures as compact operator diagnostics.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Candid(err) => write!(formatter, "{err}"),
            Self::Canister(err) => write!(formatter, "{err}"),
            Self::Cbor(err) => formatter.write_str(err),
            Self::Io(err) => write!(formatter, "{err}"),
            Self::Query(message) => write!(formatter, "{message}"),
            Self::Registry(err) => write!(formatter, "{err}"),
            Self::Rejected { code, message } => {
                write!(
                    formatter,
                    "local replica rejected query: code={code} message={message}"
                )
            }
        }
    }
}

impl Error for ReplicaQueryError {
    // Preserve structured source errors for I/O and serialization failures.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Candid(err) => Some(err),
            Self::Io(err) => Some(err),
            Self::Registry(err) => Some(err),
            Self::Canister(_) | Self::Cbor(_) | Self::Query(_) | Self::Rejected { .. } => None,
        }
    }
}

impl From<std::io::Error> for ReplicaQueryError {
    // Convert local socket and process I/O failures.
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<cbor::CborError> for ReplicaQueryError {
    // Convert CBOR encode/decode failures.
    fn from(err: cbor::CborError) -> Self {
        Self::Cbor(err.to_string())
    }
}

impl From<RegistryParseError> for ReplicaQueryError {
    fn from(err: RegistryParseError) -> Self {
        Self::Registry(err)
    }
}

/// Return whether the selected network should use direct local replica queries.
#[must_use]
pub fn should_use_local_replica_query(network: Option<&str>) -> bool {
    network.is_none_or(|network| network == "local" || network.starts_with("http://"))
}

/// Query `canic_ready` directly through the local replica HTTP API.
pub fn query_ready(network: Option<&str>, canister: &str) -> Result<bool, ReplicaQueryError> {
    let bytes = local_query(network, canister, "canic_ready")?;
    Decode!(&bytes, bool).map_err(ReplicaQueryError::Candid)
}

/// Query `canic_ready` using the configured port from one ICP root.
pub fn query_ready_from_root(
    network: Option<&str>,
    canister: &str,
    icp_root: &Path,
) -> Result<bool, ReplicaQueryError> {
    let bytes = local_query_from_root(network, canister, "canic_ready", icp_root)?;
    Decode!(&bytes, bool).map_err(ReplicaQueryError::Candid)
}

/// Query `canic_bootstrap_status` using the configured port from one ICP root.
pub fn query_bootstrap_status_from_root(
    network: Option<&str>,
    canister: &str,
    icp_root: &Path,
) -> Result<BootstrapStatusResponse, ReplicaQueryError> {
    let bytes = local_query_from_root(network, canister, "canic_bootstrap_status", icp_root)?;
    decode_bootstrap_status_response(&bytes)
}

/// Query `canic_cycle_balance` directly through the local replica HTTP API.
pub(crate) fn query_cycle_balance(
    network: Option<&str>,
    canister: &str,
) -> Result<u128, ReplicaQueryError> {
    let bytes = local_query(network, canister, "canic_cycle_balance")?;
    decode_cycle_balance_response(&bytes)
}

/// Query `canic_cycle_balance` using the configured port from one ICP root.
pub fn query_cycle_balance_from_root(
    network: Option<&str>,
    canister: &str,
    icp_root: &Path,
) -> Result<u128, ReplicaQueryError> {
    let bytes = local_query_from_root(network, canister, "canic_cycle_balance", icp_root)?;
    decode_cycle_balance_response(&bytes)
}

/// Query `canic_subnet_registry` and return validated host entries.
pub fn query_subnet_registry_entries(
    network: Option<&str>,
    root: &str,
) -> Result<Vec<RegistryEntry>, ReplicaQueryError> {
    let response = query_subnet_registry_response(network, root)?;
    registry_entries_from_response(response).map_err(Into::into)
}

/// Query `canic_subnet_registry` from one ICP root and return validated host entries.
pub fn query_subnet_registry_entries_from_root(
    network: Option<&str>,
    root: &str,
    icp_root: &Path,
) -> Result<Vec<RegistryEntry>, ReplicaQueryError> {
    let response = query_subnet_registry_response_from_root(network, root, icp_root)?;
    registry_entries_from_response(response).map_err(Into::into)
}

/// Query `canic_subnet_registry` using the configured port from one ICP root and return roles.
pub fn query_subnet_registry_roles_from_root(
    network: Option<&str>,
    root: &str,
    icp_root: &Path,
) -> Result<Vec<String>, ReplicaQueryError> {
    Ok(
        query_subnet_registry_entries_from_root(network, root, icp_root)?
            .into_iter()
            .filter_map(|entry| entry.role)
            .collect(),
    )
}

fn query_subnet_registry_response(
    network: Option<&str>,
    root: &str,
) -> Result<SubnetRegistryResponse, ReplicaQueryError> {
    let bytes = local_query(network, root, "canic_subnet_registry")?;
    decode_subnet_registry_response(&bytes)
}

fn query_subnet_registry_response_from_root(
    network: Option<&str>,
    root: &str,
    icp_root: &Path,
) -> Result<SubnetRegistryResponse, ReplicaQueryError> {
    let bytes = local_query_from_root(network, root, "canic_subnet_registry", icp_root)?;
    decode_subnet_registry_response(&bytes)
}
