//! Module: fleet_catalog
//!
//! Responsibility: read and project the friendly network-scoped Fleet catalog.
//! Does not own: activation recovery, Fleet ID generation, or catalog commitment.
//! Boundary: the resolved canonical network selects one exact fail-closed catalog.

#[cfg(test)]
mod tests;

use crate::{
    durable_io::{RegularFileReadError, read_optional_regular_bytes},
    network::{
        NetworkIdentityError, resolve_canonical_network_id_from_root, validate_environment_name,
    },
};
use canic_core::{
    cdk::types::Principal,
    ids::{AppId, CanonicalNetworkId, FleetId, FleetName, FleetNameParseError},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    io,
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

const FLEET_CATALOG_SCHEMA_VERSION: u32 = 1;
const FLEET_CATALOG_RELATIVE_PATH: &str = "fleets/catalog.json";
const CANONICAL_NAME_MAX_BYTES: usize = 40;

///
/// FleetCatalogRequest
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetCatalogRequest {
    pub project_root: PathBuf,
    pub environment: String,
    pub generated_at: String,
}

///
/// FleetCatalogReportV1
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetCatalogReportV1 {
    pub schema_version: u32,
    pub generated_at: String,
    pub project_root: Option<String>,
    pub canonical_network_id: CanonicalNetworkId,
    pub environment: String,
    pub entries: Vec<FleetCatalogEntryV1>,
}

///
/// FleetCatalogEntryV1
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FleetCatalogEntryV1 {
    pub canonical_network_id: CanonicalNetworkId,
    pub fleet_id: FleetId,
    pub fleet_name: FleetName,
    pub app: AppId,
    /// Non-authoritative environment-profile provenance from installation.
    pub environment: String,
    pub deployed_at_unix_secs: u64,
    pub root_principal: String,
    pub root_verification: FleetCatalogRootVerificationV1,
}

///
/// FleetCatalogRootVerificationV1
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FleetCatalogRootVerificationV1 {
    #[serde(rename = "not_verified")]
    NotVerified,
    #[serde(rename = "verified")]
    Verified,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct FleetCatalogRecord {
    schema_version: u32,
    canonical_network_id: CanonicalNetworkId,
    entries: Vec<FleetCatalogEntryV1>,
}

///
/// FleetCatalogError
///

#[derive(Debug, ThisError)]
pub enum FleetCatalogError {
    #[error(transparent)]
    Network(#[from] NetworkIdentityError),

    #[error("Fleet name is invalid: {0}")]
    FleetName(#[from] FleetNameParseError),

    #[error("Fleet {fleet_name} is not known on canonical network {canonical_network_id}")]
    UnknownFleet {
        canonical_network_id: CanonicalNetworkId,
        fleet_name: FleetName,
    },

    #[error("Fleet catalog is not a regular non-symlink file: {}", path.display())]
    NotRegular { path: PathBuf },

    #[error("Fleet catalog is unsupported on platform {0}")]
    UnsupportedPlatform(&'static str),

    #[error("failed to read Fleet catalog {}: {source}", path.display())]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to decode Fleet catalog {}: {source}", path.display())]
    Decode {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("invalid Fleet catalog {}: {reason}", path.display())]
    Invalid { path: PathBuf, reason: String },
}

/// Build a read-only report from the one catalog selected by canonical network identity.
pub fn build_fleet_catalog_report(
    request: &FleetCatalogRequest,
) -> Result<FleetCatalogReportV1, FleetCatalogError> {
    validate_environment_name(&request.environment)?;
    let canonical_network_id =
        resolve_canonical_network_id_from_root(&request.project_root, &request.environment)?;
    let path = fleet_catalog_path(&request.project_root, canonical_network_id);
    let entries = match read_catalog(&path, canonical_network_id)? {
        Some(catalog) => catalog.entries,
        None => Vec::new(),
    };

    Ok(FleetCatalogReportV1 {
        schema_version: FLEET_CATALOG_SCHEMA_VERSION,
        generated_at: request.generated_at.clone(),
        project_root: Some(".".to_string()),
        canonical_network_id,
        environment: request.environment.clone(),
        entries,
    })
}

/// Build a report containing one exact Fleet-name lookup.
pub fn inspect_fleet_catalog_report(
    request: &FleetCatalogRequest,
    fleet_name: &str,
) -> Result<FleetCatalogReportV1, FleetCatalogError> {
    let fleet_name = fleet_name.parse::<FleetName>()?;
    let mut report = build_fleet_catalog_report(request)?;
    let entry = report
        .entries
        .iter()
        .find(|entry| entry.fleet_name == fleet_name)
        .cloned()
        .ok_or(FleetCatalogError::UnknownFleet {
            canonical_network_id: report.canonical_network_id,
            fleet_name,
        })?;
    report.entries = vec![entry];
    Ok(report)
}

#[must_use]
pub fn fleet_catalog_report_text(report: &FleetCatalogReportV1) -> String {
    let mut lines = vec![
        "Fleet catalog:".to_string(),
        format!("generated_at: {}", report.generated_at),
        format!("network: {}", report.canonical_network_id),
        format!("environment: {}", report.environment),
        format!("entries: {}", report.entries.len()),
    ];
    if let Some(project_root) = &report.project_root {
        lines.push(format!("project_root: {project_root}"));
    }
    if report.entries.is_empty() {
        lines.push("fleets: none".to_string());
        return lines.join("\n");
    }

    lines.push("fleets:".to_string());
    for entry in &report.entries {
        lines.push(format!("  {}", entry.fleet_name));
        lines.push(format!("    fleet_id: {}", entry.fleet_id));
        lines.push(format!("    app: {}", entry.app));
        lines.push(format!("    environment: {}", entry.environment));
        lines.push(format!("    root_principal: {}", entry.root_principal));
        lines.push(format!(
            "    root_verification: {}",
            root_verification_label(entry.root_verification)
        ));
    }
    lines.join("\n")
}

fn read_catalog(
    path: &Path,
    canonical_network_id: CanonicalNetworkId,
) -> Result<Option<FleetCatalogRecord>, FleetCatalogError> {
    let Some(bytes) = read_optional_regular_bytes(path).map_err(|error| match error {
        RegularFileReadError::NotRegular => FleetCatalogError::NotRegular {
            path: path.to_path_buf(),
        },
        RegularFileReadError::Io(source) => FleetCatalogError::Read {
            path: path.to_path_buf(),
            source,
        },
        #[cfg(not(unix))]
        RegularFileReadError::UnsupportedPlatform => {
            FleetCatalogError::UnsupportedPlatform(std::env::consts::OS)
        }
    })?
    else {
        return Ok(None);
    };
    let catalog = serde_json::from_slice::<FleetCatalogRecord>(&bytes).map_err(|source| {
        FleetCatalogError::Decode {
            path: path.to_path_buf(),
            source,
        }
    })?;
    validate_catalog(path, &catalog, canonical_network_id)?;
    Ok(Some(catalog))
}

fn validate_catalog(
    path: &Path,
    catalog: &FleetCatalogRecord,
    canonical_network_id: CanonicalNetworkId,
) -> Result<(), FleetCatalogError> {
    if catalog.schema_version != FLEET_CATALOG_SCHEMA_VERSION {
        return invalid(
            path,
            format!(
                "schema version {} is not supported; expected {}",
                catalog.schema_version, FLEET_CATALOG_SCHEMA_VERSION
            ),
        );
    }
    if catalog.canonical_network_id != canonical_network_id {
        return invalid(
            path,
            format!(
                "catalog network {} does not match resolved network {canonical_network_id}",
                catalog.canonical_network_id
            ),
        );
    }

    let mut previous_name: Option<&FleetName> = None;
    let mut fleet_ids = BTreeSet::new();
    for entry in &catalog.entries {
        if entry.canonical_network_id != canonical_network_id {
            return invalid(
                path,
                format!(
                    "Fleet {} records network {}, not {canonical_network_id}",
                    entry.fleet_name, entry.canonical_network_id
                ),
            );
        }
        if previous_name.is_some_and(|previous| previous >= &entry.fleet_name) {
            return invalid(
                path,
                "Fleet entries must be strictly ordered by fleet_name".to_string(),
            );
        }
        if !fleet_ids.insert(entry.fleet_id) {
            return invalid(
                path,
                format!("Fleet ID {} appears more than once", entry.fleet_id),
            );
        }
        validate_canonical_name(entry.app.as_str()).map_err(|reason| {
            FleetCatalogError::Invalid {
                path: path.to_path_buf(),
                reason: format!("App {} {reason}", entry.app),
            }
        })?;
        validate_environment_name(&entry.environment)?;
        Principal::from_text(&entry.root_principal).map_err(|error| {
            FleetCatalogError::Invalid {
                path: path.to_path_buf(),
                reason: format!(
                    "Fleet {} has invalid root principal: {error}",
                    entry.fleet_name
                ),
            }
        })?;
        previous_name = Some(&entry.fleet_name);
    }
    Ok(())
}

fn validate_canonical_name(value: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err("must not be empty".to_string());
    }
    if value.len() > CANONICAL_NAME_MAX_BYTES {
        return Err(format!("must not exceed {CANONICAL_NAME_MAX_BYTES} bytes"));
    }
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err("must use only ASCII letters, numbers, '-' or '_'".to_string());
    }
    Ok(())
}

fn invalid<T>(path: &Path, reason: String) -> Result<T, FleetCatalogError> {
    Err(FleetCatalogError::Invalid {
        path: path.to_path_buf(),
        reason,
    })
}

fn fleet_catalog_path(project_root: &Path, canonical_network_id: CanonicalNetworkId) -> PathBuf {
    project_root
        .join(".canic")
        .join("networks")
        .join(canonical_network_id.to_string())
        .join(FLEET_CATALOG_RELATIVE_PATH)
}

const fn root_verification_label(status: FleetCatalogRootVerificationV1) -> &'static str {
    match status {
        FleetCatalogRootVerificationV1::NotVerified => "not_verified",
        FleetCatalogRootVerificationV1::Verified => "verified",
    }
}
