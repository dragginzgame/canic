//! Module: config_discovery
//!
//! Responsibility: discover current Canic App configuration files in one workspace.
//! Does not own: Fleet convergence, prompting, reviewed plans, or runtime state.
//! Boundary: returns canonical local paths and rejects duplicate App identities.

#[cfg(test)]
mod tests;

use crate::{
    release_set::{WorkspaceDiscoveryError, read_app_config_identity},
    workspace_discovery::{discovery_start, has_workspace_manifest},
};
use std::{
    collections::BTreeMap,
    env, fs, io,
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

const APPS_ROOT: &str = "apps";
const MAX_CONFIG_DEPTH: usize = 32;
const MAX_CONFIG_ENTRIES: usize = 100_000;
const EXCLUDED_DIRECTORIES: [&str; 4] = [".canic", ".git", "node_modules", "target"];

/// Typed failure while discovering current App configuration.

#[derive(Debug, ThisError)]
pub enum ConfigDiscoveryError {
    #[error("failed to resolve current directory: {0}")]
    CurrentDirectory(#[source] io::Error),

    #[error("failed to canonicalize Canic workspace path {path}: {source}")]
    Canonicalize {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to read Canic config directory {path}: {source}")]
    Directory {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to inspect Canic config path {path}: {source}")]
    Path {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("multiple configs declare app {app}: {configs}")]
    DuplicateApp { app: String, configs: String },

    #[error("Canic config discovery exceeds {maximum} directory levels at {path}")]
    DepthLimit { path: PathBuf, maximum: usize },

    #[error("Canic config discovery exceeds {maximum} entries at {path}")]
    EntryLimit { path: PathBuf, maximum: usize },

    #[error(transparent)]
    WorkspaceDiscovery(#[from] WorkspaceDiscoveryError),
}

/// Resolve the operator-facing Canic workspace root from the current directory.
pub fn current_canic_workspace_root() -> Result<PathBuf, ConfigDiscoveryError> {
    let current_dir = env::current_dir().map_err(ConfigDiscoveryError::CurrentDirectory)?;
    let current_dir =
        current_dir
            .canonicalize()
            .map_err(|source| ConfigDiscoveryError::Canonicalize {
                path: current_dir,
                source,
            })?;
    Ok(discover_canic_workspace_root_from(&current_dir)?.unwrap_or(current_dir))
}

/// Find the nearest App configuration root without crossing a Cargo workspace boundary.
pub fn discover_canic_workspace_root_from(
    start: &Path,
) -> Result<Option<PathBuf>, ConfigDiscoveryError> {
    let start = discovery_start(start)?;
    for candidate in start.ancestors() {
        if !discover_workspace_canic_config_choices(candidate)?.is_empty() {
            return Ok(Some(candidate.to_path_buf()));
        }
        if has_workspace_manifest(candidate)? {
            break;
        }
    }
    Ok(None)
}

/// Discover App configuration choices in the current workspace.
pub fn discover_current_canic_config_choices() -> Result<Vec<PathBuf>, ConfigDiscoveryError> {
    let workspace_root = current_canic_workspace_root()?;
    discover_workspace_canic_config_choices(&workspace_root)
}

/// Discover candidate `canic.toml` files under conventional App roots.
/// Generated trees and symlinks are excluded; exhausted traversal bounds are errors.
pub fn discover_workspace_canic_config_choices(
    workspace_root: &Path,
) -> Result<Vec<PathBuf>, ConfigDiscoveryError> {
    let mut choices = Vec::new();
    let mut visited_entries = 0;
    for root in workspace_app_roots(workspace_root) {
        collect_canic_config_choices(&root, &mut choices, 0, &mut visited_entries)?;
    }
    choices.sort();
    choices.dedup();
    reject_duplicate_app_names(&choices)?;
    Ok(choices)
}

/// Return conventional App roots for one workspace.
#[must_use]
pub fn workspace_app_roots(workspace_root: &Path) -> Vec<PathBuf> {
    vec![workspace_root.join(APPS_ROOT)]
}

/// Select one exact discovered config by its declared App identity.
pub fn select_discovered_app_config_path(
    choices: &[PathBuf],
    app: &str,
) -> Result<Option<PathBuf>, ConfigDiscoveryError> {
    Ok(unique_configs_by_app(choices)?.remove(app))
}

fn reject_duplicate_app_names(choices: &[PathBuf]) -> Result<(), ConfigDiscoveryError> {
    let _ = unique_configs_by_app(choices)?;
    Ok(())
}

fn unique_configs_by_app(
    choices: &[PathBuf],
) -> Result<BTreeMap<String, PathBuf>, ConfigDiscoveryError> {
    let mut by_app = BTreeMap::<String, Vec<&PathBuf>>::new();
    for path in choices {
        if let Ok(app) = read_app_config_identity(path) {
            by_app.entry(app).or_default().push(path);
        }
    }
    for (app, paths) in &by_app {
        if paths.len() > 1 {
            return Err(ConfigDiscoveryError::DuplicateApp {
                app: app.clone(),
                configs: paths
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", "),
            });
        }
    }
    Ok(by_app
        .into_iter()
        .filter_map(|(app, mut paths)| paths.pop().cloned().map(|path| (app, path)))
        .collect())
}

fn collect_canic_config_choices(
    root: &Path,
    choices: &mut Vec<PathBuf>,
    depth: usize,
    visited_entries: &mut usize,
) -> Result<(), ConfigDiscoveryError> {
    let metadata = match fs::symlink_metadata(root) {
        Ok(metadata) => metadata,
        Err(source) if source.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(source) => {
            return Err(ConfigDiscoveryError::Path {
                path: root.to_path_buf(),
                source,
            });
        }
    };
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Ok(());
    }
    if depth > MAX_CONFIG_DEPTH {
        return Err(ConfigDiscoveryError::DepthLimit {
            path: root.to_path_buf(),
            maximum: MAX_CONFIG_DEPTH,
        });
    }
    for entry in fs::read_dir(root).map_err(|source| ConfigDiscoveryError::Directory {
        path: root.to_path_buf(),
        source,
    })? {
        let entry = entry.map_err(|source| ConfigDiscoveryError::Directory {
            path: root.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if *visited_entries >= MAX_CONFIG_ENTRIES {
            return Err(ConfigDiscoveryError::EntryLimit {
                path,
                maximum: MAX_CONFIG_ENTRIES,
            });
        }
        *visited_entries += 1;
        let file_type = entry
            .file_type()
            .map_err(|source| ConfigDiscoveryError::Path {
                path: path.clone(),
                source,
            })?;
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            if !EXCLUDED_DIRECTORIES
                .iter()
                .any(|name| entry.file_name() == *name)
            {
                collect_canic_config_choices(&path, choices, depth + 1, visited_entries)?;
            }
        } else if file_type.is_file()
            && path.file_name().and_then(|name| name.to_str()) == Some("canic.toml")
        {
            choices.push(path);
        }
    }
    Ok(())
}
