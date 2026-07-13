use crate::workspace_discovery::{discover_icp_root_from, discover_workspace_root_from};
use std::path::{Path, PathBuf};

use super::super::{CANISTERS_ROOT_RELATIVE, ROOT_CONFIG_FILE, WORKSPACE_MANIFEST_RELATIVE};

// Resolve the downstream Cargo workspace root from the current directory.
pub fn workspace_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Some(root) = discover_workspace_root_from(&std::env::current_dir()?) {
        return Ok(root);
    }

    Ok(std::env::current_dir()?.canonicalize()?)
}

// Resolve the Cargo workspace containing an explicit config or workspace hint.
pub fn workspace_root_from(path: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Some(root) = discover_workspace_root_from(path) {
        return Ok(root);
    }

    workspace_root()
}

// Resolve the downstream ICP CLI/project root from the current directory.
pub fn icp_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let current_dir = std::env::current_dir()?.canonicalize()?;
    if let Some(root) = discover_icp_root_from(&current_dir) {
        return Ok(root);
    }

    Ok(current_dir)
}

// Resolve the downstream Canic config path.
#[must_use]
pub fn config_path(workspace_root: &Path) -> PathBuf {
    canisters_root(workspace_root).join(ROOT_CONFIG_FILE)
}

// Resolve the downstream canister-manifest root.
#[must_use]
pub fn canisters_root(workspace_root: &Path) -> PathBuf {
    workspace_root.join(CANISTERS_ROOT_RELATIVE)
}

// Resolve the downstream workspace manifest path.
#[must_use]
pub fn workspace_manifest_path(workspace_root: &Path) -> PathBuf {
    workspace_root.join(WORKSPACE_MANIFEST_RELATIVE)
}

// Render a path relative to the workspace root when possible.
#[must_use]
pub fn display_workspace_path(workspace_root: &Path, path: &Path) -> String {
    path.strip_prefix(workspace_root)
        .unwrap_or(path)
        .display()
        .to_string()
}
