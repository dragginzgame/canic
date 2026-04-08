use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

use super::{
    build_release_set_entry, config_path, configured_release_roles, load_root_package_version,
    resolve_artifact_root, root_manifest_path, root_release_set_manifest_path,
    workspace_manifest_path,
};

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

// Build and persist the current root release-set manifest from built `.wasm.gz` artifacts.
pub fn emit_root_release_set_manifest(
    workspace_root: &Path,
    dfx_root: &Path,
    network: &str,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let artifact_root = resolve_artifact_root(dfx_root, network)?;
    let config_path = config_path(workspace_root);
    let manifest_path = root_release_set_manifest_path(&artifact_root)?;
    let release_version = load_root_package_version(
        &root_manifest_path(workspace_root),
        &workspace_manifest_path(workspace_root),
    )?;
    let entries = configured_release_roles(&config_path)?
        .into_iter()
        .map(|role_name| build_release_set_entry(dfx_root, &artifact_root, &role_name))
        .collect::<Result<Vec<_>, _>>()?;
    let manifest = RootReleaseSetManifest {
        release_version,
        entries,
    };

    fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest)?)?;
    Ok(manifest_path)
}

// Emit the root release-set manifest only once every required ordinary artifact exists.
pub fn emit_root_release_set_manifest_if_ready(
    workspace_root: &Path,
    dfx_root: &Path,
    network: &str,
) -> Result<Option<std::path::PathBuf>, Box<dyn std::error::Error>> {
    let artifact_root = resolve_artifact_root(dfx_root, network)?;
    let roles = configured_release_roles(&config_path(workspace_root))?;

    for role_name in roles {
        let artifact_path = artifact_root
            .join(&role_name)
            .join(format!("{role_name}.wasm.gz"));
        if !artifact_path.is_file() {
            return Ok(None);
        }
    }

    emit_root_release_set_manifest(workspace_root, dfx_root, network).map(Some)
}

// Load one previously emitted root release-set manifest from disk.
pub fn load_root_release_set_manifest(
    manifest_path: &Path,
) -> Result<RootReleaseSetManifest, Box<dyn std::error::Error>> {
    let source = fs::read(manifest_path)?;
    Ok(serde_json::from_slice(&source)?)
}
