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

// Resolve exactly one canister manifest for a role, restricted to packages below
// the selected canister root.
pub fn resolve_canister_manifest_from_metadata_under(
    workspace_root: &Path,
    role_name: &str,
    search_root: &Path,
) -> Result<PathBuf, String> {
    let metadata = cargo_metadata_no_deps_cached(workspace_root).map_err(|err| {
        format!("cargo metadata failed while resolving role '{role_name}': {err}")
    })?;
    let search_root = search_root
        .canonicalize()
        .unwrap_or_else(|_| search_root.to_path_buf());

    let matches = metadata
        .packages
        .into_iter()
        .filter(|package| package.manifest_path.starts_with(&search_root))
        .filter(|package| package_declares_role(package, role_name))
        .map(|package| package.manifest_path)
        .collect::<Vec<_>>();

    match matches.as_slice() {
        [manifest_path] => Ok(manifest_path.clone()),
        [] => Err(format!(
            "no canister package under {} declares [package.metadata.canic] role = \"{role_name}\"",
            search_root.display()
        )),
        paths => Err(format!(
            "multiple canister packages under {} declare [package.metadata.canic] role = \"{role_name}\": {}",
            search_root.display(),
            paths
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
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
