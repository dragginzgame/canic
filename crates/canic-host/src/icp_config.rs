use crate::{
    install_root::{
        current_canic_project_root, discover_project_canic_config_choices, project_fleet_roots,
    },
    release_set::{configured_fleet_name, configured_fleet_roles, icp_root},
    workspace_discovery::discover_icp_root_from,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt, fs,
    path::{Path, PathBuf},
};

const ICP_CONFIG_FILE: &str = "icp.yaml";
pub const DEFAULT_LOCAL_GATEWAY_PORT: u16 = 8000;

///
/// IcpConfigError
///

#[derive(Debug)]
pub enum IcpConfigError {
    NoIcpRoot { start: PathBuf },
    Config(String),
    Io(std::io::Error),
}

impl fmt::Display for IcpConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoIcpRoot { start } => {
                write!(
                    formatter,
                    "could not find icp.yaml from {}",
                    start.display()
                )
            }
            Self::Config(message) => write!(formatter, "{message}"),
            Self::Io(err) => write!(formatter, "{err}"),
        }
    }
}

impl Error for IcpConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Config(_) | Self::NoIcpRoot { .. } => None,
        }
    }
}

impl From<std::io::Error> for IcpConfigError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
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
    pub fn is_ready(&self) -> bool {
        self.icp_yaml_present
            && self.local_network_present
            && self.missing_canisters.is_empty()
            && self.missing_environments.is_empty()
    }

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
pub fn configured_local_gateway_port() -> Result<u16, IcpConfigError> {
    let root = current_icp_root()?;
    configured_local_gateway_port_from_root(&root)
}

/// Return the configured local ICP gateway port for one ICP project root.
pub fn configured_local_gateway_port_from_root(root: &Path) -> Result<u16, IcpConfigError> {
    let source = fs::read_to_string(root.join(ICP_CONFIG_FILE))?;
    Ok(local_gateway_port_from_yaml(&source))
}

/// Inspect whether `icp.yaml` contains the entries implied by Canic fleet configs.
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
    let spec = discover_project_spec(&root, fleet_filter)?;
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
    let start = std::env::current_dir()?;
    discover_icp_root_from(&start).ok_or(IcpConfigError::NoIcpRoot { start })
}

/// Resolve the ICP project root implied by the current Canic fleet layout.
pub fn resolve_current_canic_icp_root() -> Result<PathBuf, IcpConfigError> {
    if let Ok(path) = std::env::var("CANIC_ICP_ROOT") {
        return PathBuf::from(path)
            .canonicalize()
            .map_err(IcpConfigError::from);
    }

    let search_root = current_project_search_root()?;
    let choices = discover_project_canic_config_choices(&search_root)
        .map_err(|err| IcpConfigError::Config(err.to_string()))?;
    if !choices.is_empty() {
        return Ok(search_root);
    }

    current_icp_root().or_else(|_| {
        icp_root()
            .map_err(|err| IcpConfigError::Config(err.to_string()))
            .and_then(|path| path.canonicalize().map_err(IcpConfigError::from))
    })
}

fn current_project_search_root() -> Result<PathBuf, IcpConfigError> {
    let root = current_canic_project_root()
        .map_err(|err| IcpConfigError::Config(err.to_string()))?
        .canonicalize()?;
    if !discover_project_canic_config_choices(&root)
        .map_err(|err| IcpConfigError::Config(err.to_string()))?
        .is_empty()
    {
        return Ok(root);
    }

    if let Ok(root) = icp_root() {
        return Ok(root);
    }
    Ok(std::env::current_dir()?.canonicalize()?)
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
    let choices = discover_project_canic_config_choices(root)
        .map_err(|err| IcpConfigError::Config(err.to_string()))?;
    if choices.is_empty() {
        return Err(IcpConfigError::Config(format!(
            "no Canic fleet configs found under {}\nCreate fleets/<fleet>/canic.toml, then add matching entries to icp.yaml and rerun `canic status`.",
            display_project_fleet_roots(root)
        )));
    }

    let mut canisters = Vec::<String>::new();
    let mut seen_canisters = BTreeSet::<String>::new();
    let mut environments = BTreeMap::<String, Vec<String>>::new();
    let mut matched_filter = fleet_filter.is_none();

    for config_path in choices {
        let fleet = configured_fleet_name(&config_path)
            .map_err(|err| IcpConfigError::Config(err.to_string()))?;
        if fleet_filter.is_some_and(|filter| filter == fleet) {
            matched_filter = true;
        }

        let roles = configured_fleet_roles(&config_path)
            .map_err(|err| IcpConfigError::Config(err.to_string()))?;
        for role in &roles {
            if seen_canisters.insert(role.clone()) {
                canisters.push(role.clone());
            }
        }
        environments.insert(fleet, roles);
    }

    if let Some(fleet) = fleet_filter
        && !matched_filter
    {
        return Err(IcpConfigError::Config(format!(
            "no Canic fleet config found for {fleet}\nExpected a config under {} with `[fleet].name = \"{fleet}\"`.",
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
mod tests {
    use super::*;
    use crate::test_support::temp_dir;
    use std::fs;

    #[test]
    fn defaults_local_gateway_port_without_network_config() {
        let source = "canisters: []\n";

        assert_eq!(
            local_gateway_port_from_yaml(source),
            DEFAULT_LOCAL_GATEWAY_PORT
        );
    }

    #[test]
    fn reads_local_gateway_port_from_network_config() {
        let source = "networks:\n  - name: local\n    mode: managed\n    gateway:\n      bind: 127.0.0.1\n      port: 8001\n";

        assert_eq!(local_gateway_port_from_yaml(source), 8001);
    }

    #[test]
    fn ignores_nested_networks_keys_when_reading_local_gateway_port() {
        let source = "canisters:\n  - name: root\n    metadata:\n      networks:\n        - local\n\nnetworks:\n  - name: local\n    mode: managed\n    gateway:\n      bind: 127.0.0.1\n      port: 8010\n";

        assert_eq!(local_gateway_port_from_yaml(source), 8010);
    }

    #[test]
    fn inspects_icp_yaml_without_mutating_it() {
        let root = temp_dir("canic-icp-read-only");
        let config = root.join("fleets/toko/canic.toml");
        fs::create_dir_all(config.parent().expect("config parent")).expect("create config parent");
        fs::write(
            &config,
            r#"
[fleet]
name = "toko"

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.app]
kind = "singleton"
"#,
        )
        .expect("write config");
        let source = r#"
canisters:
  - name: root

networks:
  - name: local
    mode: managed
    gateway:
      port: 8010

environments:
  - name: toko
    network: local
    canisters: [root]
"#;
        fs::write(root.join("icp.yaml"), source).expect("write icp yaml");

        let report = inspect_canic_icp_yaml_from_root(&root, Some("toko")).expect("inspect");

        assert_eq!(report.canisters, vec!["root", "app"]);
        assert_eq!(report.environments, vec!["toko"]);
        assert_eq!(report.missing_canisters, vec!["app"]);
        assert!(report.missing_environments.is_empty());
        assert!(report.local_network_present);
        assert!(!report.is_ready());
        assert_eq!(
            fs::read_to_string(root.join("icp.yaml")).expect("read icp yaml"),
            source
        );
        fs::remove_dir_all(root).expect("clean temp dir");
    }

    #[test]
    fn reports_missing_icp_yaml_as_incomplete() {
        let root = temp_dir("canic-icp-missing-yaml");
        let config = root.join("fleets/toko/canic.toml");
        fs::create_dir_all(config.parent().expect("config parent")).expect("create config parent");
        fs::write(
            &config,
            r#"
[fleet]
name = "toko"

[subnets.prime.canisters.root]
kind = "root"
"#,
        )
        .expect("write config");

        let report = inspect_canic_icp_yaml_from_root(&root, Some("toko")).expect("inspect");

        assert!(!report.icp_yaml_present);
        assert_eq!(report.missing_canisters, vec!["root"]);
        assert_eq!(report.missing_environments, vec!["toko"]);
        assert!(!report.local_network_present);
        assert!(!report.is_ready());
        fs::remove_dir_all(root).expect("clean temp dir");
    }

    #[test]
    fn discovers_root_fleet_configs_for_icp_inspection() {
        let root = temp_dir("canic-icp-inspect-root-fleets");
        let config = root.join("fleets/toko/canic.toml");
        fs::create_dir_all(config.parent().expect("config parent")).expect("create config parent");
        fs::write(
            &config,
            r#"
[fleet]
name = "toko"

[subnets.prime.canisters.root]
kind = "root"

[subnets.prime.canisters.app]
kind = "singleton"
"#,
        )
        .expect("write config");

        let spec = discover_project_spec(&root, Some("toko")).expect("discover spec");

        assert_eq!(spec.canisters, vec!["root", "app"]);
        assert_eq!(
            spec.environments,
            BTreeMap::from([(
                "toko".to_string(),
                vec!["root".to_string(), "app".to_string()]
            )])
        );
        fs::remove_dir_all(root).expect("clean temp dir");
    }

    #[test]
    fn nested_commands_discover_outer_project_root_with_fleets() {
        let root = temp_dir("canic-icp-root-nested");
        let config = root.join("fleets/toko/canic.toml");
        let nested = root.join("backend/src");
        fs::create_dir_all(&nested).expect("create nested dir");
        fs::create_dir_all(config.parent().expect("config parent")).expect("create config parent");
        fs::write(root.join("icp.yaml"), "").expect("write icp config");
        fs::write(&config, "[fleet]\nname = \"toko\"\n").expect("write config");

        let icp_root = crate::install_root::discover_canic_project_root_from(&nested)
            .expect("discover project root")
            .expect("project root is present");

        assert_eq!(icp_root, root.canonicalize().expect("canonical root"));
        fs::remove_dir_all(root).expect("clean temp dir");
    }

    #[test]
    fn outer_project_root_wins_over_nested_fleets() {
        let root = temp_dir("canic-icp-root-outer-wins");
        let outer_config = root.join("fleets/toko/canic.toml");
        let nested_config = root.join("services/fleets/toko/canic.toml");
        let nested = root.join("services/src");
        fs::create_dir_all(outer_config.parent().expect("outer config parent"))
            .expect("create outer config parent");
        fs::create_dir_all(nested_config.parent().expect("nested config parent"))
            .expect("create nested config parent");
        fs::create_dir_all(&nested).expect("create nested dir");
        fs::write(root.join("icp.yaml"), "").expect("write icp config");
        fs::write(&outer_config, "[fleet]\nname = \"toko\"\n").expect("write outer config");
        fs::write(&nested_config, "[fleet]\nname = \"toko\"\n").expect("write nested config");

        let icp_root = crate::install_root::discover_canic_project_root_from(&nested)
            .expect("discover project root")
            .expect("project root is present");

        assert_eq!(icp_root, root.canonicalize().expect("canonical root"));
        fs::remove_dir_all(root).expect("clean temp dir");
    }

    #[test]
    fn icp_inspection_rejects_missing_fleet_configs() {
        let root = temp_dir("canic-icp-inspect-missing");
        fs::create_dir_all(&root).expect("create root");

        let err = discover_project_spec(&root, None).expect_err("missing configs should fail");
        let message = err.to_string();

        assert!(message.contains("no Canic fleet configs found under"));
        assert!(message.contains("fleets/<fleet>/canic.toml"));
        fs::remove_dir_all(root).expect("clean temp dir");
    }
}
