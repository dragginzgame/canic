//! Module: config_discovery
//!
//! Responsibility: discover current Canic App configuration files in one workspace.
//! Does not own: Fleet convergence, prompting, reviewed plans, or runtime state.
//! Boundary: returns canonical local paths and rejects duplicate App identities.

use crate::release_set::{WorkspaceDiscoveryError, read_app_config_identity};
use std::{
    collections::BTreeMap,
    env, fs, io,
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

const APPS_ROOT: &str = "apps";
const ICP_CONFIG_FILE: &str = "icp.yaml";

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

/// Find the nearest conventional Canic workspace from one start path.
pub fn discover_canic_workspace_root_from(
    start: &Path,
) -> Result<Option<PathBuf>, ConfigDiscoveryError> {
    let mut nearest_apps_root = None;
    for candidate in start.ancestors() {
        if !discover_workspace_canic_config_choices(candidate)?.is_empty() {
            let root =
                candidate
                    .canonicalize()
                    .map_err(|source| ConfigDiscoveryError::Canonicalize {
                        path: candidate.to_path_buf(),
                        source,
                    })?;
            if candidate.join(ICP_CONFIG_FILE).is_file() {
                return Ok(Some(root));
            }
            if nearest_apps_root.is_none() {
                nearest_apps_root = Some(root);
            }
        }
    }
    Ok(nearest_apps_root)
}

/// Discover App configuration choices in the current workspace.
pub fn discover_current_canic_config_choices() -> Result<Vec<PathBuf>, ConfigDiscoveryError> {
    let workspace_root = current_canic_workspace_root()?;
    discover_workspace_canic_config_choices(&workspace_root)
}

/// Discover candidate `canic.toml` files under conventional App roots.
pub fn discover_workspace_canic_config_choices(
    workspace_root: &Path,
) -> Result<Vec<PathBuf>, ConfigDiscoveryError> {
    let mut choices = Vec::new();
    for root in workspace_app_roots(workspace_root) {
        collect_canic_config_choices(&root, &mut choices)?;
    }
    choices.sort();
    choices.dedup();
    reject_duplicate_app_names(&choices)?;
    Ok(choices)
}

/// Discover candidate `canic.toml` files under one App root.
pub fn discover_canic_config_choices(root: &Path) -> Result<Vec<PathBuf>, ConfigDiscoveryError> {
    let mut choices = Vec::new();
    collect_canic_config_choices(root, &mut choices)?;
    choices.sort();
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
) -> Result<(), ConfigDiscoveryError> {
    if !root.is_dir() {
        return Ok(());
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
            collect_canic_config_choices(&path, choices)?;
        } else if file_type.is_file()
            && path.file_name().and_then(|name| name.to_str()) == Some("canic.toml")
        {
            choices.push(path);
        }
    }
    Ok(())
}
