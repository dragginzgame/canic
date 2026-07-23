use crate::{
    install_root::{
        ConfigDiscoveryError, current_canic_project_root, discover_project_canic_config_choices,
        project_fleet_roots,
    },
    release_set::{AppConfigError, AppConfigSnapshot, WorkspaceDiscoveryError, icp_root},
    workspace_discovery::discover_icp_root_from,
};
use canic_core::ids::BuildNetwork;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

const ICP_CONFIG_FILE: &str = "icp.yaml";
pub const DEFAULT_LOCAL_GATEWAY_PORT: u16 = 8000;

///
/// IcpConfigError
///

#[derive(Debug, ThisError)]
pub enum IcpConfigError {
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
    pub local_network_present: bool,
}

impl IcpProjectConfigReport {
    #[must_use]
    pub const fn is_ready(&self) -> bool {
        self.icp_yaml_present
            && self.local_network_present
            && self.missing_canisters.is_empty()
            && self.missing_environments.is_empty()
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
    let source = fs::read_to_string(root.join(ICP_CONFIG_FILE))?;
    Ok(local_gateway_port_from_yaml(&source))
}

/// Resolve a selected ICP environment to the build-time network class used by Canic.
///
/// The implicit `local` and `ic` environments resolve without project config.
/// Other names must exist under `environments` in `icp.yaml`; their declared
/// ICP network decides whether Cargo builds a local/test or IC mainnet artifact.
pub fn resolve_icp_build_network_from_root(
    root: &Path,
    environment: &str,
) -> Result<BuildNetwork, IcpConfigError> {
    let environment = environment.trim();
    if environment.is_empty() {
        return Err(IcpConfigError::Config(
            "ICP environment name must not be empty".to_string(),
        ));
    }
    match environment {
        "local" => return Ok(BuildNetwork::Local),
        "ic" => return Ok(BuildNetwork::Ic),
        _ => {}
    }

    let path = root.join(ICP_CONFIG_FILE);
    let source = fs::read_to_string(&path).map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            IcpConfigError::Config(format!(
                "ICP environment '{environment}' cannot be resolved because {} is missing",
                path.display()
            ))
        } else {
            IcpConfigError::Io(err)
        }
    })?;
    resolve_icp_build_network_from_yaml(&source, environment)
        .map_err(|message| IcpConfigError::Config(format!("{}: {message}", path.display())))
}

/// Inspect whether `icp.yaml` contains the entries implied by Canic App configs.
pub fn inspect_canic_icp_yaml(
    fleet_filter: Option<&str>,
) -> Result<IcpProjectConfigReport, IcpConfigError> {
    let root = resolve_current_canic_icp_root()?;
    inspect_canic_icp_yaml_from_root(&root, fleet_filter)
}

/// Inspect one ICP project root without mutating its `icp.yaml`.
pub fn inspect_canic_icp_yaml_from_root(
    root: &Path,
    fleet_filter: Option<&str>,
) -> Result<IcpProjectConfigReport, IcpConfigError> {
    let path = root.join(ICP_CONFIG_FILE);
    let (source, icp_yaml_present) = read_optional_icp_yaml(&path)?;
    let spec = discover_project_spec(root, fleet_filter)?;
    let configured_canisters = top_level_named_items(&source, "canisters:");
    let configured_environments = top_level_named_items(&source, "environments:");
    let lines = source.lines().collect::<Vec<_>>();
    let local_network_present = local_network_block(&lines).is_some();

    let missing_canisters = spec
        .canisters
        .iter()
        .filter(|name| !configured_canisters.contains(*name))
        .cloned()
        .collect::<Vec<_>>();
    let missing_environments = spec
        .environments
        .keys()
        .filter(|name| !configured_environments.contains(*name))
        .cloned()
        .collect::<Vec<_>>();

    Ok(IcpProjectConfigReport {
        path,
        icp_root: root.to_path_buf(),
        icp_yaml_present,
        canisters: spec.canisters,
        environments: spec.environments.into_keys().collect(),
        missing_canisters,
        missing_environments,
        local_network_present,
    })
}

fn read_optional_icp_yaml(path: &Path) -> Result<(String, bool), IcpConfigError> {
    match fs::read_to_string(path) {
        Ok(source) => Ok((source, true)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok((String::new(), false)),
        Err(err) => Err(err.into()),
    }
}

fn current_icp_root() -> Result<PathBuf, IcpConfigError> {
    let start = std::env::current_dir().map_err(WorkspaceDiscoveryError::CurrentDirectory)?;
    discover_icp_root_from(&start)?.ok_or(IcpConfigError::NoIcpRoot { start })
}

/// Resolve the ICP project root implied by the current Canic fleet layout.
pub fn resolve_current_canic_icp_root() -> Result<PathBuf, IcpConfigError> {
    let root = current_canic_project_root()?.canonicalize()?;
    if !discover_project_canic_config_choices(&root)?.is_empty() {
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
    fleet_filter: Option<&str>,
) -> Result<CanicIcpSpec, IcpConfigError> {
    let choices = discover_project_canic_config_choices(root)?;
    if choices.is_empty() {
        return Err(IcpConfigError::Config(format!(
            "no Canic App configs found under {}\nCreate fleets/<app>/canic.toml, then add matching entries to icp.yaml and rerun `canic status`.",
            display_project_fleet_roots(root)
        )));
    }

    let mut canisters = Vec::<String>::new();
    let mut seen_canisters = BTreeSet::<String>::new();
    let mut environments = BTreeMap::<String, Vec<String>>::new();
    let mut matched_filter = fleet_filter.is_none();

    for config_path in choices {
        let config = AppConfigSnapshot::load(&config_path)?;
        let app = config.app_id().to_string();
        if let Some(filter) = fleet_filter {
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

    if let Some(fleet) = fleet_filter
        && !matched_filter
    {
        return Err(IcpConfigError::Config(format!(
            "no Canic App config found for {fleet}\nExpected a config under {} with `[app].name = \"{fleet}\"`.",
            display_project_fleet_roots(root)
        )));
    }

    Ok(CanicIcpSpec {
        canisters,
        environments,
    })
}

fn display_project_fleet_roots(root: &Path) -> String {
    project_fleet_roots(root)
        .into_iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(" or ")
}

fn top_level_section(lines: &[&str], header: &str) -> Option<(usize, usize)> {
    let start = lines
        .iter()
        .position(|line| line_indent(line) == 0 && line.trim() == header)?;
    let end = lines
        .iter()
        .enumerate()
        .skip(start + 1)
        .find(|(_, line)| {
            !line.trim().is_empty() && line_indent(line) == 0 && !line.trim_start().starts_with('#')
        })
        .map_or(lines.len(), |(index, _)| index);
    Some((start, end))
}

fn resolve_icp_build_network_from_yaml(
    source: &str,
    environment: &str,
) -> Result<BuildNetwork, String> {
    let lines = source.lines().collect::<Vec<_>>();
    let (_, environment_start, environment_end) =
        named_item_block(&lines, "environments:", environment)?.ok_or_else(|| {
            format!(
                "ICP environment '{environment}' is not declared under environments; add it or use the implicit local/ic environment"
            )
        })?;
    let configured_network =
        item_scalar_field(&lines, environment_start, environment_end, "network")?.ok_or_else(
            || format!("ICP environment '{environment}' has no configured backing network"),
        )?;

    match configured_network.as_str() {
        "ic" => Ok(BuildNetwork::Ic),
        "local" => Ok(BuildNetwork::Local),
        _ => {
            let (_, network_start, network_end) =
                named_item_block(&lines, "networks:", &configured_network)?.ok_or_else(|| {
                    format!(
                        "ICP environment '{environment}' references undeclared backing network '{configured_network}'"
                    )
                })?;
            let mode = item_scalar_field(&lines, network_start, network_end, "mode")?
                .ok_or_else(|| format!("ICP backing network '{configured_network}' has no mode"))?;
            match mode.as_str() {
                // ICP CLI reserves the implicit `ic` network for mainnet.
                // Declared managed and connected networks are non-mainnet build classes.
                "connected" | "managed" => Ok(BuildNetwork::Local),
                _ => Err(format!(
                    "ICP backing network '{configured_network}' has unsupported mode '{mode}'"
                )),
            }
        }
    }
}

fn named_item_block(
    lines: &[&str],
    section: &str,
    name: &str,
) -> Result<Option<(String, usize, usize)>, String> {
    let Some((section_start, section_end)) = top_level_section(lines, section) else {
        return Ok(None);
    };
    let starts = lines[section_start + 1..section_end]
        .iter()
        .enumerate()
        .filter_map(|(offset, line)| {
            if line_indent(line) != 2 {
                return None;
            }
            line.trim()
                .strip_prefix("- name:")
                .map(trim_yaml_scalar)
                .filter(|item_name| !item_name.is_empty())
                .map(|item_name| (item_name.to_string(), section_start + 1 + offset))
        })
        .collect::<Vec<_>>();
    let matches = starts
        .iter()
        .enumerate()
        .filter(|(_, (item_name, _))| item_name == name)
        .collect::<Vec<_>>();
    let [(match_index, (item_name, start))] = matches.as_slice() else {
        return if matches.is_empty() {
            Ok(None)
        } else {
            Err(format!("duplicate '{name}' entries under {section}"))
        };
    };
    let end = starts
        .get(match_index + 1)
        .map_or(section_end, |(_, next_start)| *next_start);
    Ok(Some((item_name.clone(), *start, end)))
}

fn item_scalar_field(
    lines: &[&str],
    start: usize,
    end: usize,
    field: &str,
) -> Result<Option<String>, String> {
    let prefix = format!("{field}:");
    let values = lines[start + 1..end]
        .iter()
        .filter_map(|line| {
            if line_indent(line) != 4 {
                return None;
            }
            line.trim()
                .strip_prefix(&prefix)
                .map(trim_yaml_scalar)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        })
        .collect::<Vec<_>>();
    match values.as_slice() {
        [] => Ok(None),
        [value] => Ok(Some(value.clone())),
        _ => Err(format!(
            "duplicate '{field}' fields in '{}'",
            section_name(lines, start)
        )),
    }
}

fn section_name<'a>(lines: &'a [&'a str], start: usize) -> &'a str {
    lines[start]
        .trim()
        .strip_prefix("- name:")
        .map_or("item", trim_yaml_scalar)
}

fn local_gateway_port_from_yaml(source: &str) -> u16 {
    let lines = source.lines().collect::<Vec<_>>();
    let Some((start, end)) = local_network_block(&lines) else {
        return DEFAULT_LOCAL_GATEWAY_PORT;
    };

    lines[start..end]
        .iter()
        .find_map(|line| {
            line.trim()
                .strip_prefix("port:")
                .and_then(|value| value.trim().parse::<u16>().ok())
        })
        .unwrap_or(DEFAULT_LOCAL_GATEWAY_PORT)
}

fn local_network_block(lines: &[&str]) -> Option<(usize, usize)> {
    let (section_start, section_end) = top_level_section(lines, "networks:")?;
    let start = lines[section_start + 1..section_end]
        .iter()
        .position(|line| line_indent(line) == 2 && line.trim() == "- name: local")?
        + section_start
        + 1;
    let end = lines[start + 1..section_end]
        .iter()
        .position(|line| line_indent(line) == 2 && line.trim_start().starts_with("- name:"))
        .map_or(section_end, |offset| start + 1 + offset);
    Some((start, end))
}

fn top_level_named_items(source: &str, header: &str) -> BTreeSet<String> {
    let lines = source.lines().collect::<Vec<_>>();
    let Some((start, end)) = top_level_section(&lines, header) else {
        return BTreeSet::new();
    };

    lines[start + 1..end]
        .iter()
        .filter_map(|line| {
            if line_indent(line) != 2 {
                return None;
            }
            line.trim()
                .strip_prefix("- name:")
                .map(trim_yaml_scalar)
                .filter(|name| !name.is_empty())
                .map(str::to_string)
        })
        .collect()
}

fn trim_yaml_scalar(value: &str) -> &str {
    value.trim().trim_matches('"').trim_matches('\'')
}

fn line_indent(line: &str) -> usize {
    line.chars().take_while(|c| *c == ' ').count()
}

#[cfg(test)]
mod tests;
