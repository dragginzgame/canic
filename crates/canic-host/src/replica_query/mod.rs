pub use self::{
    status::{local_replica_root_key_from_root, local_replica_status_reachable_from_root},
    transport::local_replica_endpoint_from_root,
};
use self::{
    transport::{local_query, local_query_from_root},
    wire::{
        SubnetRegistryResponseWire, decode_bootstrap_status_response,
        decode_cycle_balance_response, decode_subnet_registry_response,
    },
};
use candid::Decode;
use canic_core::dto::state::BootstrapStatusResponse;
use std::{error::Error, fmt, path::Path};

mod status;
mod transport;
mod wire;

///
/// ReplicaQueryError
///

#[derive(Debug)]
pub enum ReplicaQueryError {
    Io(std::io::Error),
    Cbor(serde_cbor::Error),
    Json(serde_json::Error),
    Query(String),
    Rejected { code: u64, message: String },
}

impl fmt::Display for ReplicaQueryError {
    // Render local replica query failures as compact operator diagnostics.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(formatter, "{err}"),
            Self::Cbor(err) => write!(formatter, "{err}"),
            Self::Json(err) => write!(formatter, "{err}"),
            Self::Query(message) => write!(formatter, "{message}"),
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
            Self::Io(err) => Some(err),
            Self::Cbor(err) => Some(err),
            Self::Json(err) => Some(err),
            Self::Query(_) | Self::Rejected { .. } => None,
        }
    }
}

impl From<std::io::Error> for ReplicaQueryError {
    // Convert local socket and process I/O failures.
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<serde_cbor::Error> for ReplicaQueryError {
    // Convert CBOR encode/decode failures.
    fn from(err: serde_cbor::Error) -> Self {
        Self::Cbor(err)
    }
}

impl From<serde_json::Error> for ReplicaQueryError {
    // Convert JSON rendering failures.
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err)
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
    Decode!(&bytes, bool).map_err(|err| ReplicaQueryError::Query(err.to_string()))
}

/// Query `canic_ready` using the configured port from one ICP root.
pub fn query_ready_from_root(
    network: Option<&str>,
    canister: &str,
    icp_root: &Path,
) -> Result<bool, ReplicaQueryError> {
    let bytes = local_query_from_root(network, canister, "canic_ready", icp_root)?;
    Decode!(&bytes, bool).map_err(|err| ReplicaQueryError::Query(err.to_string()))
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

/// Query `canic_cycle_balance` using the configured port from one ICP root.
pub fn query_cycle_balance_from_root(
    network: Option<&str>,
    canister: &str,
    icp_root: &Path,
) -> Result<u128, ReplicaQueryError> {
    let bytes = local_query_from_root(network, canister, "canic_cycle_balance", icp_root)?;
    decode_cycle_balance_response(&bytes)
}

/// Parse common JSON shapes returned by command-line calls for `canic_ready`.
#[must_use]
pub fn parse_ready_json_value(data: &serde_json::Value) -> bool {
    match data {
        serde_json::Value::Bool(value) => *value,
        serde_json::Value::String(value) => value.trim() == "(true)",
        serde_json::Value::Array(values) => values.iter().any(parse_ready_json_value),
        serde_json::Value::Object(map) => map.values().any(parse_ready_json_value),
        _ => false,
    }
}

/// Query `canic_subnet_registry` and render JSON in the CLI response shape.
pub fn query_subnet_registry_json(
    network: Option<&str>,
    root: &str,
) -> Result<String, ReplicaQueryError> {
    let response = query_subnet_registry_response(network, root)?;
    serde_json::to_string(&response.to_cli_json()).map_err(ReplicaQueryError::from)
}

/// Query `canic_subnet_registry` using the configured port from one ICP root.
pub fn query_subnet_registry_json_from_root(
    network: Option<&str>,
    root: &str,
    icp_root: &Path,
) -> Result<String, ReplicaQueryError> {
    let response = query_subnet_registry_response_from_root(network, root, icp_root)?;
    serde_json::to_string(&response.to_cli_json()).map_err(ReplicaQueryError::from)
}

/// Query `canic_subnet_registry` using the configured port from one ICP root and return roles.
pub fn query_subnet_registry_roles_from_root(
    network: Option<&str>,
    root: &str,
    icp_root: &Path,
) -> Result<Vec<String>, ReplicaQueryError> {
    Ok(query_subnet_registry_response_from_root(network, root, icp_root)?.roles())
}

fn query_subnet_registry_response(
    network: Option<&str>,
    root: &str,
) -> Result<SubnetRegistryResponseWire, ReplicaQueryError> {
    let bytes = local_query(network, root, "canic_subnet_registry")?;
    decode_subnet_registry_response(&bytes)
}

fn query_subnet_registry_response_from_root(
    network: Option<&str>,
    root: &str,
    icp_root: &Path,
) -> Result<SubnetRegistryResponseWire, ReplicaQueryError> {
    let bytes = local_query_from_root(network, root, "canic_subnet_registry", icp_root)?;
    decode_subnet_registry_response(&bytes)
}

#[cfg(test)]
mod tests;
