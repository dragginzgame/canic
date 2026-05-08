//! Workspace and ICP CLI root discovery helpers for downstream install tooling.

use serde_json::Value as JsonValue;
use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::cargo_metadata::{CargoMetadataPackage, cargo_metadata_no_deps_cached};

const WORKSPACE_MANIFEST_RELATIVE: &str = "Cargo.toml";
const ICP_CONFIG_FILE: &str = "icp.yaml";

// Resolve the nearest Cargo workspace root from a starting file or directory path.
pub fn discover_workspace_root_from(path: &Path) -> Option<PathBuf> {
    let start = if path.is_file() { path.parent()? } else { path };

    for candidate in start.ancestors() {
        let manifest_path = candidate.join(WORKSPACE_MANIFEST_RELATIVE);
        if !manifest_path.is_file() {
            continue;
        }

        let manifest = fs::read_to_string(&manifest_path).ok()?;
        if manifest.contains("[workspace]") {
            return candidate.canonicalize().ok();
        }
    }

    None
}

// Resolve the nearest ICP CLI root from a starting file or directory path.
pub fn discover_icp_root_from(path: &Path) -> Option<PathBuf> {
    let start = if path.is_file() { path.parent()? } else { path };

    for candidate in start.ancestors() {
        let icp_config = candidate.join(ICP_CONFIG_FILE);
        if icp_config.is_file() {
            return candidate.canonicalize().ok();
        }
    }

    None
}

// Normalize a workspace-relative path against the chosen workspace root.
pub fn normalize_workspace_path(workspace_root: &Path, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        workspace_root.join(path)
    }
}

// Discover the manifest for a role using workspace metadata first, then package naming.
pub fn discover_canister_manifest_from_metadata(
    workspace_root: &Path,
    role_name: &str,
) -> Option<PathBuf> {
    let metadata = cargo_metadata_no_deps_cached(workspace_root).ok()?;
    let expected_package_name = format!("canister_{role_name}");

    metadata
        .packages
        .into_iter()
        .find(|package| {
            package_declares_role(package, role_name) || package.name == expected_package_name
        })
        .map(|package| package.manifest_path)
}

// Check whether a package declares the requested Canic role in Cargo metadata.
fn package_declares_role(package: &CargoMetadataPackage, role_name: &str) -> bool {
    package
        .metadata
        .as_ref()
        .and_then(|metadata| metadata.get("canic"))
        .and_then(|canic| canic.get("role"))
        .and_then(JsonValue::as_str)
        == Some(role_name)
}
