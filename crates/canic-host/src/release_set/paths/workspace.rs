use crate::workspace_discovery::{
    WorkspaceDiscoveryError, discover_icp_root_from, discover_workspace_root_from,
};
use std::path::{Path, PathBuf};

use super::super::{APP_SOURCES_ROOT_RELATIVE, ROOT_CONFIG_FILE, WORKSPACE_MANIFEST_RELATIVE};

// Resolve the downstream Cargo workspace root from the current directory.
pub fn workspace_root() -> Result<PathBuf, WorkspaceDiscoveryError> {
    let current_dir = std::env::current_dir().map_err(WorkspaceDiscoveryError::CurrentDirectory)?;
    if let Some(root) = discover_workspace_root_from(&current_dir)? {
        return Ok(root);
    }

    current_dir
        .canonicalize()
        .map_err(|source| WorkspaceDiscoveryError::Canonicalize {
            path: current_dir,
            source,
        })
}

// Resolve the downstream ICP CLI/project root from the current directory.
pub fn icp_root() -> Result<PathBuf, WorkspaceDiscoveryError> {
    let current_dir = std::env::current_dir().map_err(WorkspaceDiscoveryError::CurrentDirectory)?;
    if let Some(root) = discover_icp_root_from(&current_dir)? {
        return Ok(root);
    }

    current_dir
        .canonicalize()
        .map_err(|source| WorkspaceDiscoveryError::Canonicalize {
            path: current_dir,
            source,
        })
}

// Resolve the downstream Canic config path.
#[must_use]
pub fn config_path(workspace_root: &Path) -> PathBuf {
    app_sources_root(workspace_root).join(ROOT_CONFIG_FILE)
}

// Resolve the downstream App source root.
#[must_use]
pub fn app_sources_root(workspace_root: &Path) -> PathBuf {
    workspace_root.join(APP_SOURCES_ROOT_RELATIVE)
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
