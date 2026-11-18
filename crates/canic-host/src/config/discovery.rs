//! One bounded catalog for app listings and selection; no saved context or registry.
use super::{Config, load_config, name};
use crate::{Error, Result, supplier};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

/// One validated local config and its source location.
#[derive(Debug)]
pub struct DiscoveredConfig {
    /// Location used for relative artifacts and journal paths.
    pub path: PathBuf,
    /// Validated declarations; directory names never establish app identity.
    pub config: Config,
}

/// Read explicit configuration, or current and direct-child `canic.toml` files.
/// Automatic discovery skips symlinked directories and generated/hidden trees.
/// # Errors
/// Refuses malformed configs, duplicate app identities and more than 1024 entries or 64 configs.
pub fn discover_configs(
    directory: &Path,
    explicit: Option<&Path>,
) -> Result<Vec<DiscoveredConfig>> {
    let mut paths = Vec::new();
    if let Some(path) = explicit {
        paths.push(directory.join(path));
    } else {
        let local = directory.join("canic.toml");
        if local.try_exists().map_err(supplier)? {
            paths.push(local);
        }
        let scan = if directory.as_os_str().is_empty() {
            Path::new(".")
        } else {
            directory
        };
        for (count, entry) in std::fs::read_dir(scan).map_err(supplier)?.enumerate() {
            if count >= 1024 {
                return Err(Error::Invalid(
                    "app discovery exceeds 1024 entries; select --config".into(),
                ));
            }
            let entry = entry.map_err(supplier)?;
            let file_name = entry.file_name();
            let Some(child) = file_name.to_str() else {
                continue;
            };
            if name(child).is_err()
                || matches!(child, "target" | "node_modules" | "vendor")
                || !entry.file_type().map_err(supplier)?.is_dir()
            {
                continue;
            }
            let path = directory.join(child).join("canic.toml");
            match std::fs::symlink_metadata(&path) {
                Ok(metadata) if metadata.is_file() => paths.push(path),
                Ok(_) => continue,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => return Err(supplier(error)),
            }
            if paths.len() > 64 {
                return Err(Error::Invalid(
                    "app discovery exceeds 64 configs; select --config".into(),
                ));
            }
        }
        paths.sort();
    }
    let mut owners = BTreeMap::new();
    let mut configs = Vec::new();
    for path in paths {
        let config = load_config(&path)?;
        for app in config.apps.keys() {
            if owners.insert(app.clone(), ()).is_some() {
                return Err(Error::Invalid(format!(
                    "multiple configs declare app {app}; select --config explicitly"
                )));
            }
        }
        configs.push(DiscoveredConfig { path, config });
    }
    Ok(configs)
}

/// Locate an app using the same catalog as listings. Explicit paths always win.
/// # Errors
/// Refuses missing/ambiguous apps, duplicate identities and malformed discovered configs.
pub fn resolve_config_path(
    directory: &Path,
    explicit: Option<&Path>,
    app: Option<&str>,
) -> Result<PathBuf> {
    if let Some(path) = explicit {
        return Ok(directory.join(path));
    }
    if let Some(app) = app {
        name(app)?;
    }
    let configs = discover_configs(directory, None)?;
    if let Some(app) = app {
        return configs
            .into_iter()
            .find(|item| item.config.apps.contains_key(app))
            .map(|item| item.path)
            .ok_or_else(|| {
                Error::Invalid(format!(
                    "unknown app {app}; use app list or select --config"
                ))
            });
    }
    match configs.as_slice() {
        [] => Ok(directory.join("canic.toml")),
        [item] => Ok(item.path.clone()),
        _ => Err(Error::Invalid(
            "multiple app configs found; select --app or --config".into(),
        )),
    }
}
