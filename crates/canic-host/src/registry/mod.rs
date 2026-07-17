//! Module: registry
//!
//! Responsibility: project typed subnet-registry responses into host registry entries.
//! Does not own: registry persistence, ICP command execution, or deployment topology policy.
//! Boundary: decodes the canonical ICP envelope and validates redundant registry identity fields.

#[cfg(test)]
mod tests;

use crate::icp::{IcpJsonResponseError, decode_json_result_response};
use canic_core::{
    cdk::utils::hash::hex_bytes,
    dto::topology::{SubnetRegistryEntry, SubnetRegistryResponse},
};
use thiserror::Error as ThisError;

///
/// RegistryEntry
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegistryEntry {
    pub pid: String,
    pub role: Option<String>,
    pub parent_pid: Option<String>,
    pub module_hash: Option<String>,
}

///
/// RegistryParseError
///

#[derive(Debug, ThisError)]
pub enum RegistryParseError {
    #[error("registry entry principal mismatch: entry={entry_pid} record={record_pid}")]
    PrincipalMismatch {
        entry_pid: String,
        record_pid: String,
    },

    #[error(transparent)]
    Response(#[from] IcpJsonResponseError),

    #[error("registry entry role mismatch for {pid}: entry={entry_role} record={record_role}")]
    RoleMismatch {
        pid: String,
        entry_role: String,
        record_role: String,
    },
}

/// Decode and validate one canonical ICP subnet-registry response.
pub fn parse_registry_entries(
    registry_json: &str,
) -> Result<Vec<RegistryEntry>, RegistryParseError> {
    let response = decode_json_result_response::<SubnetRegistryResponse>(registry_json)?;
    registry_entries_from_response(response)
}

/// Validate and project one typed subnet-registry response.
pub(crate) fn registry_entries_from_response(
    response: SubnetRegistryResponse,
) -> Result<Vec<RegistryEntry>, RegistryParseError> {
    response.0.into_iter().map(registry_entry).collect()
}

fn registry_entry(entry: SubnetRegistryEntry) -> Result<RegistryEntry, RegistryParseError> {
    let pid = entry.pid.to_text();
    let record_pid = entry.record.pid.to_text();
    if pid != record_pid {
        return Err(RegistryParseError::PrincipalMismatch {
            entry_pid: pid,
            record_pid,
        });
    }

    let role = entry.role.into_string();
    let record_role = entry.record.role.into_string();
    if role != record_role {
        return Err(RegistryParseError::RoleMismatch {
            pid,
            entry_role: role,
            record_role,
        });
    }

    Ok(RegistryEntry {
        pid,
        role: Some(role),
        parent_pid: entry.record.parent_pid.map(|pid| pid.to_text()),
        module_hash: entry.record.module_hash.map(hex_bytes),
    })
}
