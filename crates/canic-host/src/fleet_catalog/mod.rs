//! Module: fleet_catalog
//!
//! Responsibility: commit, read, and project the network-scoped Fleet discovery catalog.
//! Does not own: terminal-install validation, Registry mutation, or Fleet identity allocation.
//! Boundary: one network lock serializes atomic Coordinator-anchored publication; readers fail
//! closed.

#[cfg(test)]
mod tests;

use crate::{
    durable_io::{
        RegularFileLockError, RegularFileReadError, lock_regular_file_with_parents,
        read_optional_regular_bytes, write_bytes,
    },
    network::{
        NetworkIdentityError, resolve_canonical_network_id_from_root, validate_environment_name,
    },
};
use canic_core::{
    cdk::types::Principal,
    ids::{AppId, CanonicalNetworkId, FleetId, FleetName, FleetNameParseError, ReleaseBuildId},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
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
    pub workspace_root: PathBuf,
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
    pub workspace_root: Option<String>,
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
    pub release_build_id: ReleaseBuildId,
    pub coordinator_principal: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct FleetCatalogRecord {
    schema_version: u32,
    canonical_network_id: CanonicalNetworkId,
    entries: Vec<FleetCatalogEntryV1>,
}

///
/// CommittedFleetCatalog
///
/// Exact durable catalog result returned to the terminal-install publication workflow.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CommittedFleetCatalog {
    pub entry: FleetCatalogEntryV1,
    pub catalog_hash: [u8; 32],
    pub advanced: bool,
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

    #[error("failed to commit Fleet catalog {}: {source}", path.display())]
    Write {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("Fleet catalog commitment conflicts with existing {field} authority: {value}")]
    Conflict { field: &'static str, value: String },

    #[error("failed to decode Fleet catalog {}: {source}", path.display())]
    Decode {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("failed to encode Fleet catalog: {source}")]
    Encode {
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
        resolve_canonical_network_id_from_root(&request.workspace_root, &request.environment)?;
    let path = fleet_catalog_path(&request.workspace_root, canonical_network_id);
    let entries = match read_catalog(&path, canonical_network_id)? {
        Some(catalog) => catalog.entries,
        None => Vec::new(),
    };

    Ok(FleetCatalogReportV1 {
        schema_version: FLEET_CATALOG_SCHEMA_VERSION,
        generated_at: request.generated_at.clone(),
        workspace_root: Some(".".to_string()),
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
    let mut report = build_fleet_catalog_report(request)?;
    let entry = require_fleet_catalog_entry(&report, fleet_name)?;
    report.entries = vec![entry];
    Ok(report)
}

/// Resolve one exact installed Fleet from the catalog selected by canonical
/// network identity.
pub fn read_fleet_catalog_entry_from_root(
    workspace_root: &Path,
    environment: &str,
    fleet_name: &str,
) -> Result<Option<FleetCatalogEntryV1>, FleetCatalogError> {
    let fleet_name = fleet_name.parse::<FleetName>()?;
    let report = build_fleet_catalog_report(&FleetCatalogRequest {
        workspace_root: workspace_root.to_path_buf(),
        environment: environment.to_string(),
        generated_at: String::new(),
    })?;
    Ok(report
        .entries
        .into_iter()
        .find(|entry| entry.fleet_name == fleet_name))
}

/// Commit one exact Coordinator-anchored row after the caller validates terminal evidence.
pub(crate) fn commit_fleet_catalog_entry(
    workspace_root: &Path,
    entry: FleetCatalogEntryV1,
) -> Result<CommittedFleetCatalog, FleetCatalogError> {
    let path = fleet_catalog_path(workspace_root, entry.canonical_network_id);
    let lock_path = fleet_catalog_lock_path(workspace_root, entry.canonical_network_id);
    let _lock = lock_regular_file_with_parents(&lock_path).map_err(|error| match error {
        RegularFileLockError::NotRegular => FleetCatalogError::NotRegular {
            path: lock_path.clone(),
        },
        RegularFileLockError::Io(source) => FleetCatalogError::Write {
            path: lock_path.clone(),
            source,
        },
        #[cfg(windows)]
        RegularFileLockError::UnsupportedPlatform => {
            FleetCatalogError::UnsupportedPlatform(std::env::consts::OS)
        }
    })?;

    let existing = read_catalog_document(&path, entry.canonical_network_id)?;
    let mut catalog = existing.as_ref().map_or_else(
        || FleetCatalogRecord {
            schema_version: FLEET_CATALOG_SCHEMA_VERSION,
            canonical_network_id: entry.canonical_network_id,
            entries: Vec::new(),
        },
        |document| document.record.clone(),
    );
    if let Some(existing_entry) = existing_authority(&catalog.entries, &entry)? {
        let bytes = existing
            .expect("existing authority came from an existing catalog")
            .bytes;
        return Ok(CommittedFleetCatalog {
            entry: existing_entry.clone(),
            catalog_hash: Sha256::digest(bytes).into(),
            advanced: false,
        });
    }

    catalog.entries.push(entry.clone());
    catalog
        .entries
        .sort_by(|left, right| left.fleet_name.cmp(&right.fleet_name));
    validate_catalog(&path, &catalog, entry.canonical_network_id)?;
    let bytes = canonical_catalog_bytes(&catalog)?;
    write_bytes(&path, &bytes).map_err(|source| FleetCatalogError::Write {
        path: path.clone(),
        source,
    })?;
    let durable = read_catalog_document(&path, entry.canonical_network_id)?.ok_or_else(|| {
        FleetCatalogError::Invalid {
            path: path.clone(),
            reason: "committed Fleet catalog is missing after publication".to_string(),
        }
    })?;
    if durable.record != catalog || durable.bytes != bytes {
        return invalid(
            &path,
            "committed Fleet catalog differs from the exact publication bytes".to_string(),
        );
    }
    Ok(CommittedFleetCatalog {
        entry,
        catalog_hash: Sha256::digest(bytes).into(),
        advanced: true,
    })
}

fn require_fleet_catalog_entry(
    report: &FleetCatalogReportV1,
    fleet_name: &str,
) -> Result<FleetCatalogEntryV1, FleetCatalogError> {
    let fleet_name = fleet_name.parse::<FleetName>()?;
    report
        .entries
        .iter()
        .find(|entry| entry.fleet_name == fleet_name)
        .cloned()
        .ok_or(FleetCatalogError::UnknownFleet {
            canonical_network_id: report.canonical_network_id,
            fleet_name,
        })
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
    if let Some(workspace_root) = &report.workspace_root {
        lines.push(format!("workspace_root: {workspace_root}"));
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
        lines.push(format!(
            "    coordinator_principal: {}",
            entry.coordinator_principal
        ));
    }
    lines.join("\n")
}

fn read_catalog(
    path: &Path,
    canonical_network_id: CanonicalNetworkId,
) -> Result<Option<FleetCatalogRecord>, FleetCatalogError> {
    Ok(read_catalog_document(path, canonical_network_id)?.map(|document| document.record))
}

struct FleetCatalogDocument {
    record: FleetCatalogRecord,
    bytes: Vec<u8>,
}

fn read_catalog_document(
    path: &Path,
    canonical_network_id: CanonicalNetworkId,
) -> Result<Option<FleetCatalogDocument>, FleetCatalogError> {
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
    Ok(Some(FleetCatalogDocument {
        record: catalog,
        bytes,
    }))
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
    let mut coordinator_principals = BTreeMap::new();
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
        if entry.deployed_at_unix_secs == 0 {
            return invalid(
                path,
                format!(
                    "Fleet {} has a non-positive deployment time",
                    entry.fleet_name
                ),
            );
        }
        let coordinator_principal =
            Principal::from_text(&entry.coordinator_principal).map_err(|error| {
                FleetCatalogError::Invalid {
                    path: path.to_path_buf(),
                    reason: format!(
                        "Fleet {} has invalid Coordinator principal: {error}",
                        entry.fleet_name
                    ),
                }
            })?;
        if coordinator_principal == Principal::anonymous()
            || coordinator_principal == Principal::management_canister()
        {
            return invalid(
                path,
                format!(
                    "Fleet {} does not identify a Canister Coordinator",
                    entry.fleet_name
                ),
            );
        }
        if let Some(first) = coordinator_principals.insert(coordinator_principal, &entry.fleet_name)
        {
            return invalid(
                path,
                format!(
                    "Coordinator principal {} belongs to both Fleet {first} and {}",
                    entry.coordinator_principal, entry.fleet_name
                ),
            );
        }
        previous_name = Some(&entry.fleet_name);
    }
    Ok(())
}

fn existing_authority<'a>(
    entries: &'a [FleetCatalogEntryV1],
    requested: &FleetCatalogEntryV1,
) -> Result<Option<&'a FleetCatalogEntryV1>, FleetCatalogError> {
    let by_name = entries
        .iter()
        .find(|entry| entry.fleet_name == requested.fleet_name);
    let by_id = entries
        .iter()
        .find(|entry| entry.fleet_id == requested.fleet_id);
    let by_coordinator = entries
        .iter()
        .find(|entry| entry.coordinator_principal == requested.coordinator_principal);
    for (field, value, existing) in [
        ("fleet_name", requested.fleet_name.to_string(), by_name),
        ("fleet_id", requested.fleet_id.to_string(), by_id),
        (
            "coordinator_principal",
            requested.coordinator_principal.clone(),
            by_coordinator,
        ),
    ] {
        if let Some(existing) = existing {
            if same_fleet_authority(existing, requested) {
                return Ok(Some(existing));
            }
            return Err(FleetCatalogError::Conflict { field, value });
        }
    }
    Ok(None)
}

fn same_fleet_authority(existing: &FleetCatalogEntryV1, requested: &FleetCatalogEntryV1) -> bool {
    existing.canonical_network_id == requested.canonical_network_id
        && existing.fleet_id == requested.fleet_id
        && existing.fleet_name == requested.fleet_name
        && existing.app == requested.app
        && existing.coordinator_principal == requested.coordinator_principal
}

fn canonical_catalog_bytes(catalog: &FleetCatalogRecord) -> Result<Vec<u8>, FleetCatalogError> {
    let mut bytes = serde_json::to_vec_pretty(catalog)
        .map_err(|source| FleetCatalogError::Encode { source })?;
    bytes.push(b'\n');
    Ok(bytes)
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

fn fleet_catalog_path(workspace_root: &Path, canonical_network_id: CanonicalNetworkId) -> PathBuf {
    workspace_root
        .join(".canic")
        .join("networks")
        .join(canonical_network_id.to_string())
        .join(FLEET_CATALOG_RELATIVE_PATH)
}

fn fleet_catalog_lock_path(
    workspace_root: &Path,
    canonical_network_id: CanonicalNetworkId,
) -> PathBuf {
    workspace_root
        .join(".canic")
        .join("networks")
        .join(canonical_network_id.to_string())
        .join("fleets/catalog.lock")
}
