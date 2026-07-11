use crate::workspace_discovery::{
    discover_icp_root_from, discover_workspace_root_from, normalize_workspace_path,
};
use std::path::{Path, PathBuf};

use super::super::{CANISTERS_ROOT_RELATIVE, ROOT_CONFIG_FILE, WORKSPACE_MANIFEST_RELATIVE};

// Resolve the downstream Cargo workspace root from the current directory,
// config hints, or an explicit override.
pub fn workspace_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Ok(path) = std::env::var("CANIC_WORKSPACE_ROOT") {
        return Ok(PathBuf::from(path).canonicalize()?);
    }

    if let Some(root) = std::env::var_os("CANIC_WORKSPACE_MANIFEST_PATH")
        .map(PathBuf::from)
        .and_then(|path| discover_workspace_root_from(&path))
    {
        return Ok(root);
    }

    if let Some(root) = std::env::var_os("CANIC_CONFIG_PATH")
        .map(PathBuf::from)
        .and_then(|path| discover_workspace_root_from(&path))
    {
        return Ok(root);
    }

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

// Resolve the downstream ICP CLI/project root from the current directory or an
// explicit override.
pub fn icp_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Ok(path) = std::env::var("CANIC_ICP_ROOT") {
        return Ok(PathBuf::from(path).canonicalize()?);
    }

    let current_dir = std::env::current_dir()?.canonicalize()?;
    if let Some(root) = discover_icp_root_from(&current_dir) {
        return Ok(root);
    }

    if let Ok(path) = std::env::var("CANIC_WORKSPACE_ROOT") {
        let workspace_root = PathBuf::from(path).canonicalize()?;
        if let Some(root) = discover_icp_root_from(&workspace_root) {
            return Ok(root);
        }
        return Ok(workspace_root);
    }

    Ok(current_dir)
}

// Resolve the downstream Canic config path.
#[must_use]
pub fn config_path(workspace_root: &Path) -> PathBuf {
    std::env::var_os("CANIC_CONFIG_PATH").map_or_else(
        || canisters_root(workspace_root).join(ROOT_CONFIG_FILE),
        |path| normalize_workspace_path(workspace_root, PathBuf::from(path)),
    )
}

// Resolve the downstream canister-manifest root.
#[must_use]
pub fn canisters_root(workspace_root: &Path) -> PathBuf {
    if let Some(path) = std::env::var_os("CANIC_CANISTERS_ROOT") {
        return normalize_workspace_path(workspace_root, PathBuf::from(path));
    }

    if let Some(path) = std::env::var_os("CANIC_CONFIG_PATH") {
        let config_path = normalize_workspace_path(workspace_root, PathBuf::from(path));
        if let Some(parent) = config_path.parent() {
            return parent.to_path_buf();
        }
    }

    workspace_root.join(CANISTERS_ROOT_RELATIVE)
}

// Resolve the downstream workspace manifest path.
#[must_use]
pub fn workspace_manifest_path(workspace_root: &Path) -> PathBuf {
    std::env::var_os("CANIC_WORKSPACE_MANIFEST_PATH").map_or_else(
        || workspace_root.join(WORKSPACE_MANIFEST_RELATIVE),
        |path| normalize_workspace_path(workspace_root, PathBuf::from(path)),
    )
}

// Render a path relative to the workspace root when possible.
#[must_use]
pub fn display_workspace_path(workspace_root: &Path, path: &Path) -> String {
    path.strip_prefix(workspace_root)
        .unwrap_or(path)
        .display()
        .to_string()
}
