//! Local ICP configuration inspection and Canic App membership validation.
//!
//! Owns the inline config projection; ICP owns build/recipe execution and network state.

mod manifest;

#[cfg(test)]
mod tests;

use crate::{
    config_discovery::{
        ConfigDiscoveryError, current_canic_workspace_root,
        discover_workspace_canic_config_choices, workspace_app_roots,
    },
    icp_config::manifest::IcpManifest,
    release_set::{AppConfigError, AppConfigSnapshot, WorkspaceDiscoveryError, icp_root},
    workspace_discovery::discover_icp_root_from,
};
use canic_core::ids::BuildNetwork;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read as _,
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

pub use manifest::IcpManifestError;

const ICP_CONFIG_FILE: &str = "icp.yaml";
pub const DEFAULT_LOCAL_GATEWAY_PORT: u16 = 8000;

///
/// IcpConfigError
///

#[derive(Debug, ThisError)]
pub enum IcpConfigError {
    #[error("{}: {source}", path.display())]
    Manifest {
        path: PathBuf,
        source: IcpManifestError,
    },
    #[error("could not find icp.yaml from {}", start.display())]
    NoIcpRoot { start: PathBuf },

    #[error("{0}")]
    Config(String),

    #[error(transparent)]
    ConfigDiscovery(#[from] ConfigDiscoveryError),

    #[error(transparent)]
    AppConfig(#[from] AppConfigError),

    #[error(transparent)]
    WorkspaceDiscovery(#[from] WorkspaceDiscoveryError),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

///
/// IcpProjectConfigReport
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IcpProjectConfigReport {
    pub path: PathBuf,
    pub icp_root: PathBuf,
    pub icp_yaml_present: bool,
    pub canisters: Vec<String>,
    pub environments: Vec<String>,
    pub missing_canisters: Vec<String>,
    pub missing_environments: Vec<String>,
    /// Required App roles excluded by each declared environment's selection.
    pub missing_environment_canisters: BTreeMap<String, Vec<String>>,
    pub local_network_present: bool,
}

impl IcpProjectConfigReport {
    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.icp_yaml_present
            && self.local_network_present
            && self.missing_canisters.is_empty()
            && self.missing_environments.is_empty()
            && self.missing_environment_canisters.is_empty()
    }

    #[must_use]
    pub fn issues(&self) -> Vec<String> {
        let mut issues = Vec::new();
        if !self.icp_yaml_present {
            issues.push(format!("missing {}", self.path.display()));
        }
        if !self.local_network_present {
            issues.push("missing local network entry".to_string());
        }
        if !self.missing_canisters.is_empty() {
            issues.push(format!(
                "missing canisters: {}",
                self.missing_canisters.join(", ")
            ));
        }
        if !self.missing_environments.is_empty() {
            issues.push(format!(
                "missing environments: {}",
                self.missing_environments.join(", ")
            ));
        }
        for (environment, canisters) in &self.missing_environment_canisters {
            issues.push(format!(
                "environment {environment} excludes required canisters: {}",
                canisters.join(", ")
            ));
        }
        issues
    }
}

/// Return the configured local ICP gateway port, falling back to ICP's default.
pub(crate) fn configured_local_gateway_port() -> Result<u16, IcpConfigError> {
    let root = current_icp_root()?;
    configured_local_gateway_port_from_root(&root)
}

/// Return the configured local ICP gateway port for one ICP project root.
pub fn configured_local_gateway_port_from_root(root: &Path) -> Result<u16, IcpConfigError> {
    let path = root.join(ICP_CONFIG_FILE);
    let (source, _) = read_optional_icp_yaml(&path)?;
    local_gateway_port_from_yaml(&source)
        .map_err(|source| IcpConfigError::Manifest { path, source })
}

/// Resolve a selected ICP environment to the build-time network class used by Canic.
///
/// Explicit environment declarations take precedence over implicit local/ic defaults.
/// Canic reads inline network identity without resolving recipes or dependencies.
pub fn resolve_icp_build_network_from_root(
    root: &Path,
    environment: &str,
) -> Result<BuildNetwork, IcpConfigError> {
    let path = root.join(ICP_CONFIG_FILE);
    let (source, _) = read_optional_icp_yaml(&path)?;
    resolve_icp_build_network_from_yaml(&source, environment)
        .map_err(|source| IcpConfigError::Manifest { path, source })
}

/// Inspect whether `icp.yaml` contains the entries implied by Canic App configs.
pub fn inspect_canic_icp_yaml(
    app_filter: Option<&str>,
) -> Result<IcpProjectConfigReport, IcpConfigError> {
    let root = resolve_current_canic_icp_root()?;
    inspect_canic_icp_yaml_from_root(&root, app_filter)
}

/// Inspect one ICP project root without mutating its `icp.yaml`.
pub fn inspect_canic_icp_yaml_from_root(
    root: &Path,
    app_filter: Option<&str>,
) -> Result<IcpProjectConfigReport, IcpConfigError> {
    let path = root.join(ICP_CONFIG_FILE);
    let (source, icp_yaml_present) = read_optional_icp_yaml(&path)?;
    let spec = discover_project_spec(root, app_filter)?;
    let manifest = IcpManifest::parse(&source).map_err(|source| IcpConfigError::Manifest {
        path: path.clone(),
        source,
    })?;
    let local_network_present = icp_yaml_present;

    let missing_canisters = spec
        .canisters
        .iter()
        .filter(|name| !manifest.has_canister(name))
        .cloned()
        .collect::<Vec<_>>();
    let missing_environments = spec
        .environments
        .keys()
        .filter(|name| !manifest.has_environment(name))
        .cloned()
        .collect::<Vec<_>>();

    let missing_environment_canisters = spec
        .environments
        .iter()
        .filter(|(environment, _)| manifest.has_environment(environment))
        .filter_map(|(environment, roles)| {
            let missing = roles
                .iter()
                .filter(|role| !manifest.contains(environment, role))
                .cloned()
                .collect::<Vec<_>>();
            (!missing.is_empty()).then(|| (environment.clone(), missing))
        })
        .collect();

    Ok(IcpProjectConfigReport {
        path,
        icp_root: root.to_path_buf(),
        icp_yaml_present,
        canisters: spec.canisters,
        environments: spec.environments.into_keys().collect(),
        missing_canisters,
        missing_environments,
        missing_environment_canisters,
        local_network_present,
    })
}

fn read_optional_icp_yaml(path: &Path) -> Result<(String, bool), IcpConfigError> {
    match fs::File::open(path) {
        Ok(file) => {
            let mut source = String::new();
            file.take(1024 * 1024 + 1).read_to_string(&mut source)?;
            Ok((source, true))
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok((String::new(), false)),
        Err(err) => Err(err.into()),
    }
}

fn current_icp_root() -> Result<PathBuf, IcpConfigError> {
    let start = std::env::current_dir().map_err(WorkspaceDiscoveryError::CurrentDirectory)?;
    discover_icp_root_from(&start)?.ok_or(IcpConfigError::NoIcpRoot { start })
}

/// Resolve the ICP project root implied by the current Canic app layout.
pub fn resolve_current_canic_icp_root() -> Result<PathBuf, IcpConfigError> {
    let root = current_canic_workspace_root()?.canonicalize()?;
    if !discover_workspace_canic_config_choices(&root)?.is_empty() {
        return Ok(root);
    }

    Ok(icp_root()?)
}

///
/// CanicIcpSpec
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct CanicIcpSpec {
    canisters: Vec<String>,
    environments: BTreeMap<String, Vec<String>>,
}

fn discover_project_spec(
    root: &Path,
    app_filter: Option<&str>,
) -> Result<CanicIcpSpec, IcpConfigError> {
    let choices = discover_workspace_canic_config_choices(root)?;
    if choices.is_empty() {
        return Err(IcpConfigError::Config(format!(
            "no Canic App configs found under {}\nCreate apps/<app>/canic.toml, add matching entries to icp.yaml, then retry the requested command.",
            display_workspace_app_roots(root)
        )));
    }

    let mut canisters = Vec::<String>::new();
    let mut seen_canisters = BTreeSet::<String>::new();
    let mut environments = BTreeMap::<String, Vec<String>>::new();
    let mut matched_filter = app_filter.is_none();

    for config_path in choices {
        let config = AppConfigSnapshot::load(&config_path)?;
        let app = config.app_id().to_string();
        if let Some(filter) = app_filter {
            if filter != app {
                continue;
            }
            matched_filter = true;
        }

        let roles = config.deployable_roles();
        for role in &roles {
            if seen_canisters.insert(role.clone()) {
                canisters.push(role.clone());
            }
        }
        environments.insert(app, roles);
    }

    if let Some(app) = app_filter
        && !matched_filter
    {
        return Err(IcpConfigError::Config(format!(
            "no Canic App config found for {app}\nExpected a config under {} with `[app].name = \"{app}\"`.",
            display_workspace_app_roots(root)
        )));
    }

    Ok(CanicIcpSpec {
        canisters,
        environments,
    })
}

fn display_workspace_app_roots(root: &Path) -> String {
    workspace_app_roots(root)
        .into_iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(" or ")
}

fn resolve_icp_build_network_from_yaml(
    source: &str,
    environment: &str,
) -> Result<BuildNetwork, IcpManifestError> {
    IcpManifest::parse(source)?.build_network(environment)
}

fn local_gateway_port_from_yaml(source: &str) -> Result<u16, IcpManifestError> {
    IcpManifest::parse(source)?.local_gateway_port()
}
