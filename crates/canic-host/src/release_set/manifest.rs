//! Module: release_set::manifest
//!
//! Responsibility: define, validate, load, and persist root release-set manifests.
//! Does not own: artifact bytes, ICP calls, or bootstrap sequencing.
//! Boundary: admits one exact role/template identity contract for every consumer.

use crate::{
    durable_io::write_bytes,
    release_set::{
        build_release_set_entry, config_path, configured_release_roles, load_root_package_version,
        resolve_artifact_root, root_release_set_manifest_path, workspace_manifest_path,
    },
    role_contract::{declared_role_manifest_path, finding_detail},
};
use std::{collections::BTreeSet, fs, path::Path};

use serde::{Deserialize, Serialize};

///
/// RootReleaseSetManifest
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RootReleaseSetManifest {
    pub release_version: String,
    pub entries: Vec<ReleaseSetEntry>,
}

///
/// ReleaseSetEntry
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReleaseSetEntry {
    pub role: String,
    pub template_id: String,
    pub artifact_relative_path: String,
    pub payload_size_bytes: u64,
    pub payload_sha256_hex: String,
    pub chunk_size_bytes: u64,
    pub chunk_sha256_hex: Vec<String>,
}

/// Validate the canonical identity contract shared by manifest writers,
/// loaders, and staging.
pub fn validate_root_release_set_manifest(
    manifest: &RootReleaseSetManifest,
) -> Result<(), Box<dyn std::error::Error>> {
    if manifest.release_version.trim().is_empty() {
        return Err("release-set manifest version must not be empty".into());
    }

    let mut roles = BTreeSet::new();
    for entry in &manifest.entries {
        if entry.role.trim().is_empty() {
            return Err("release-set manifest role must not be empty".into());
        }
        if !roles.insert(entry.role.as_str()) {
            return Err(format!("duplicate release-set role: {}", entry.role).into());
        }

        let expected_template_id = format!("embedded:{}", entry.role);
        if entry.template_id != expected_template_id {
            return Err(format!(
                "release-set template identity mismatch for role {}: expected {}",
                entry.role, expected_template_id
            )
            .into());
        }
    }

    Ok(())
}

// Build and persist the current root release-set manifest from built `.wasm.gz` artifacts.
pub fn emit_root_release_set_manifest(
    workspace_root: &Path,
    icp_root: &Path,
    network: &str,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let config_path = config_path(workspace_root);
    emit_root_release_set_manifest_with_config(workspace_root, icp_root, network, &config_path)
}

// Build and persist the current root release-set manifest with an explicit config path.
pub fn emit_root_release_set_manifest_with_config(
    workspace_root: &Path,
    icp_root: &Path,
    network: &str,
    config_path: &Path,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let artifact_root = resolve_artifact_root(icp_root, network)?;
    let manifest_path = root_release_set_manifest_path(&artifact_root)?;
    let root_manifest_path =
        declared_role_manifest_path(config_path, &canic_core::ids::CanisterRole::ROOT)
            .map_err(|finding| finding_detail(&finding))?;
    let release_version = load_root_package_version(
        &root_manifest_path,
        &workspace_manifest_path(workspace_root),
    )?;
    let entries = configured_release_roles(config_path)?
        .into_iter()
        .map(|role_name| build_release_set_entry(icp_root, &artifact_root, &role_name))
        .collect::<Result<Vec<_>, _>>()?;
    let manifest = RootReleaseSetManifest {
        release_version,
        entries,
    };

    validate_root_release_set_manifest(&manifest)?;
    write_bytes(&manifest_path, &serde_json::to_vec_pretty(&manifest)?)?;
    Ok(manifest_path)
}

// Emit the root release-set manifest only once every required ordinary artifact exists.
pub fn emit_root_release_set_manifest_if_ready(
    workspace_root: &Path,
    icp_root: &Path,
    network: &str,
) -> Result<Option<std::path::PathBuf>, Box<dyn std::error::Error>> {
    let config_path = config_path(workspace_root);
    emit_root_release_set_manifest_if_ready_with_config(
        workspace_root,
        icp_root,
        network,
        &config_path,
    )
}

// Emit the root release-set manifest using an explicit config path once every
// required ordinary artifact exists.
pub fn emit_root_release_set_manifest_if_ready_with_config(
    workspace_root: &Path,
    icp_root: &Path,
    network: &str,
    config_path: &Path,
) -> Result<Option<std::path::PathBuf>, Box<dyn std::error::Error>> {
    let artifact_root = resolve_artifact_root(icp_root, network)?;
    let roles = configured_release_roles(config_path)?;

    for role_name in roles {
        let artifact_path = artifact_root
            .join(&role_name)
            .join(format!("{role_name}.wasm.gz"));
        if !artifact_path.is_file() {
            return Ok(None);
        }
    }

    emit_root_release_set_manifest_with_config(workspace_root, icp_root, network, config_path)
        .map(Some)
}

// Load one previously emitted root release-set manifest from disk.
pub fn load_root_release_set_manifest(
    manifest_path: &Path,
) -> Result<RootReleaseSetManifest, Box<dyn std::error::Error>> {
    let source = fs::read(manifest_path)?;
    let manifest = serde_json::from_slice(&source)?;
    validate_root_release_set_manifest(&manifest)?;
    Ok(manifest)
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> RootReleaseSetManifest {
        RootReleaseSetManifest {
            release_version: "test-version".to_string(),
            entries: vec![ReleaseSetEntry {
                role: "app".to_string(),
                template_id: "embedded:app".to_string(),
                artifact_relative_path: ".icp/local/canisters/app/app.wasm.gz".to_string(),
                payload_size_bytes: 128,
                payload_sha256_hex: "00".repeat(32),
                chunk_size_bytes: 1_048_576,
                chunk_sha256_hex: vec!["00".repeat(32)],
            }],
        }
    }

    #[test]
    fn release_set_manifest_identity_accepts_canonical_role() {
        assert!(validate_root_release_set_manifest(&manifest()).is_ok());
    }

    #[test]
    fn release_set_manifest_identity_rejects_empty_version_and_role() {
        let mut missing_version = manifest();
        missing_version.release_version.clear();
        let mut missing_role = manifest();
        missing_role.entries[0].role.clear();

        assert!(validate_root_release_set_manifest(&missing_version).is_err());
        assert!(validate_root_release_set_manifest(&missing_role).is_err());
    }

    #[test]
    fn release_set_manifest_identity_rejects_duplicate_role() {
        let mut manifest = manifest();
        manifest.entries.push(manifest.entries[0].clone());

        assert!(validate_root_release_set_manifest(&manifest).is_err());
    }

    #[test]
    fn release_set_manifest_identity_rejects_template_role_mismatch() {
        let mut manifest = manifest();
        manifest.entries[0].template_id = "embedded:other".to_string();

        assert!(validate_root_release_set_manifest(&manifest).is_err());
    }
}
