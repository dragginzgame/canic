use crate::{
    install_root::{
        CANIC_FLEETS_ROOT_ENV, discover_project_canic_config_choices_with_root,
        project_fleet_roots_with_override,
    },
    release_set::{configured_fleet_name, configured_fleet_roles, icp_root},
    workspace_discovery::discover_icp_root_from,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt, fs,
    io::ErrorKind,
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
/// IcpProjectSyncReport
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IcpProjectSyncReport {
    pub path: PathBuf,
    pub changed: bool,
    pub canisters: Vec<String>,
    pub environments: Vec<String>,
}

/// Return the configured local ICP gateway port, falling back to ICP's default.
pub fn configured_local_gateway_port() -> Result<u16, IcpConfigError> {
    let root = current_icp_root()?;
    configured_local_gateway_port_from_root(&root)
}

/// Set the local ICP gateway port in this project's `icp.yaml`.
pub fn set_configured_local_gateway_port(port: u16) -> Result<PathBuf, IcpConfigError> {
    let root = current_icp_root()?;
    let path = root.join(ICP_CONFIG_FILE);
    let source = fs::read_to_string(&path)?;
    let updated = upsert_local_gateway_port(&source, port);
    fs::write(&path, updated)?;
    Ok(path)
}

/// Reconcile the Canic-managed canister and environment sections in `icp.yaml`.
pub fn sync_canic_icp_yaml(
    fleet_filter: Option<&str>,
) -> Result<IcpProjectSyncReport, IcpConfigError> {
    sync_canic_icp_yaml_with_fleet_root(fleet_filter, None)
}

/// Reconcile Canic-managed `icp.yaml` sections using an explicit fleet config root.
pub fn sync_canic_icp_yaml_with_fleet_root(
    fleet_filter: Option<&str>,
    fleet_root_override: Option<&Path>,
) -> Result<IcpProjectSyncReport, IcpConfigError> {
    let root = current_project_root()?;
    let path = root.join(ICP_CONFIG_FILE);
    let source = match fs::read_to_string(&path) {
        Ok(source) => source,
        Err(err) if err.kind() == ErrorKind::NotFound => String::new(),
        Err(err) => return Err(err.into()),
    };
    let spec = discover_project_spec(&root, fleet_filter, fleet_root_override)?;
    let updated = sync_canic_sections(&source, &spec.canisters, &spec.environments);
    let changed = updated != source;
    if changed {
        fs::write(&path, updated)?;
    }

    Ok(IcpProjectSyncReport {
        path,
        changed,
        canisters: spec.canisters,
        environments: spec.environments.into_keys().collect(),
    })
}

fn current_icp_root() -> Result<PathBuf, IcpConfigError> {
    let start = std::env::current_dir()?;
    discover_icp_root_from(&start).ok_or(IcpConfigError::NoIcpRoot { start })
}

fn current_project_root() -> Result<PathBuf, IcpConfigError> {
    let start = std::env::current_dir()?;
    if let Some(root) = discover_icp_root_from(&start) {
        return Ok(root);
    }

    icp_root().map_err(|err| IcpConfigError::Config(err.to_string()))
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
    fleet_root_override: Option<&Path>,
) -> Result<CanicIcpSpec, IcpConfigError> {
    let choices = discover_project_canic_config_choices_with_root(root, fleet_root_override)
        .map_err(|err| IcpConfigError::Config(err.to_string()))?;
    if choices.is_empty() {
        return Err(IcpConfigError::Config(format!(
            "no Canic fleet configs found under {}\nCreate fleets/<fleet>/canic.toml or backend/fleets/<fleet>/canic.toml, then rerun `canic replica start` or `canic fleet sync --fleet <fleet>`.",
            display_project_fleet_roots(root, fleet_root_override)
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
            display_project_fleet_roots(root, fleet_root_override)
        )));
    }

    Ok(CanicIcpSpec {
        canisters,
        environments,
    })
}

fn display_project_fleet_roots(root: &Path, fleet_root_override: Option<&Path>) -> String {
    let roots = project_fleet_roots_with_override(root, fleet_root_override)
        .into_iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(" or ");
    format!(
        "{roots}\nUse `--fleets-dir <dir>` or {CANIC_FLEETS_ROOT_ENV}=<dir> for a different layout."
    )
}

fn configured_local_gateway_port_from_root(root: &Path) -> Result<u16, IcpConfigError> {
    let source = fs::read_to_string(root.join(ICP_CONFIG_FILE))?;
    Ok(configured_local_gateway_port_from_source(&source))
}

fn sync_canic_sections(
    source: &str,
    canisters: &[String],
    environments: &BTreeMap<String, Vec<String>>,
) -> String {
    let without_canisters = remove_top_level_section(source, "canisters:");
    let rest = remove_top_level_section(&without_canisters, "environments:");
    let mut sections = vec![
        render_canisters_section(canisters),
        render_environments_section(environments),
    ];
    let rest = rest.trim();
    if !rest.is_empty() {
        sections.push(rest.to_string());
    }

    let mut updated = sections.join("\n\n");
    updated.push('\n');
    updated
}

fn render_canisters_section(canisters: &[String]) -> String {
    if canisters.is_empty() {
        return "canisters: []".to_string();
    }

    let mut lines = vec!["canisters:".to_string()];
    for (index, canister) in canisters.iter().enumerate() {
        if index > 0 {
            lines.push(String::new());
        }
        lines.extend([
            format!("  - name: {canister}"),
            "    build:".to_string(),
            "      steps:".to_string(),
            "        - type: script".to_string(),
            "          commands:".to_string(),
            format!(
                "            - cargo run -q -p canic-host --example build_artifact -- {canister}"
            ),
        ]);
    }
    lines.join("\n")
}

fn render_environments_section(environments: &BTreeMap<String, Vec<String>>) -> String {
    if environments.is_empty() {
        return "environments: []".to_string();
    }

    environments
        .iter()
        .enumerate()
        .flat_map(|(index, (environment, canisters))| {
            let mut lines = Vec::new();
            if index > 0 {
                lines.push(String::new());
            }
            if index == 0 {
                lines.push("environments:".to_string());
            }
            lines.extend([
                format!("  - name: {environment}"),
                "    network: local".to_string(),
                format!("    canisters: [{}]", canisters.join(", ")),
            ]);
            lines
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn remove_top_level_section(source: &str, header: &str) -> String {
    let mut lines = source.lines().map(str::to_string).collect::<Vec<_>>();
    let line_refs = lines.iter().map(String::as_str).collect::<Vec<_>>();
    let Some((start, end)) = top_level_section(&line_refs, header) else {
        return source.to_string();
    };
    lines.drain(start..end);

    let mut compacted = Vec::<String>::new();
    let mut previous_blank = false;
    for line in lines {
        let blank = line.trim().is_empty();
        if blank && previous_blank {
            continue;
        }
        compacted.push(line);
        previous_blank = blank;
    }

    compacted.join("\n")
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

fn configured_local_gateway_port_from_source(source: &str) -> u16 {
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

fn upsert_local_gateway_port(source: &str, port: u16) -> String {
    let had_trailing_newline = source.ends_with('\n');
    let mut lines = source.lines().map(str::to_string).collect::<Vec<_>>();

    let local_block = {
        let line_refs = lines.iter().map(String::as_str).collect::<Vec<_>>();
        local_network_block(&line_refs)
    };
    if let Some((start, end)) = local_block {
        if let Some(index) = (start..end).find(|index| lines[*index].trim().starts_with("port:")) {
            let indent = line_indent(&lines[index]);
            lines[index] = format!("{}port: {port}", " ".repeat(indent));
            return join_lines(lines, had_trailing_newline);
        }

        if let Some(gateway_index) = (start..end).find(|index| lines[*index].trim() == "gateway:") {
            let indent = line_indent(&lines[gateway_index]) + 2;
            lines.insert(
                gateway_index + 1,
                format!("{}port: {port}", " ".repeat(indent)),
            );
            return join_lines(lines, had_trailing_newline);
        }

        lines.splice(end..end, local_network_gateway_lines(port));
        return join_lines(lines, had_trailing_newline);
    }

    let networks = {
        let line_refs = lines.iter().map(String::as_str).collect::<Vec<_>>();
        networks_section(&line_refs)
    };
    if let Some((networks_start, networks_end)) = networks {
        let local_network = local_network_lines(port);
        let inserted_len = local_network.len();
        lines.splice(networks_end..networks_end, local_network);
        if networks_end == networks_start + 1 {
            lines.insert(networks_end + inserted_len, String::new());
        }
        return join_lines(lines, had_trailing_newline);
    }

    let insert_at = lines
        .iter()
        .position(|line| line.trim() == "environments:")
        .unwrap_or(lines.len());
    let mut insert = vec!["networks:".to_string()];
    insert.extend(local_network_lines(port));
    insert.push(String::new());
    lines.splice(insert_at..insert_at, insert);
    join_lines(lines, had_trailing_newline)
}

fn networks_section(lines: &[&str]) -> Option<(usize, usize)> {
    let start = lines.iter().position(|line| line.trim() == "networks:")?;
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

fn local_network_block(lines: &[&str]) -> Option<(usize, usize)> {
    let (section_start, section_end) = networks_section(lines)?;
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

fn local_network_lines(port: u16) -> Vec<String> {
    vec![
        "  - name: local".to_string(),
        "    mode: managed".to_string(),
        "    gateway:".to_string(),
        "      bind: 127.0.0.1".to_string(),
        format!("      port: {port}"),
    ]
}

fn local_network_gateway_lines(port: u16) -> Vec<String> {
    vec![
        "    gateway:".to_string(),
        "      bind: 127.0.0.1".to_string(),
        format!("      port: {port}"),
    ]
}

fn line_indent(line: &str) -> usize {
    line.chars().take_while(|c| *c == ' ').count()
}

fn join_lines(lines: Vec<String>, had_trailing_newline: bool) -> String {
    let mut joined = lines.join("\n");
    if had_trailing_newline {
        joined.push('\n');
    }
    joined
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
            configured_local_gateway_port_from_source(source),
            DEFAULT_LOCAL_GATEWAY_PORT
        );
    }

    #[test]
    fn reads_local_gateway_port_from_network_config() {
        let source = "networks:\n  - name: local\n    mode: managed\n    gateway:\n      bind: 127.0.0.1\n      port: 8001\n";

        assert_eq!(configured_local_gateway_port_from_source(source), 8001);
    }

    #[test]
    fn inserts_local_network_before_environments() {
        let source = "canisters: []\n\nenvironments:\n  - name: local\n    network: local\n";

        let updated = upsert_local_gateway_port(source, 8002);

        assert!(updated.contains("networks:\n  - name: local\n    mode: managed"));
        assert!(updated.contains("      port: 8002"));
        assert!(updated.find("networks:") < updated.find("environments:"));
    }

    #[test]
    fn replaces_existing_local_gateway_port() {
        let source = "networks:\n  - name: local\n    mode: managed\n    gateway:\n      bind: 127.0.0.1\n      port: 8001\n";

        let updated = upsert_local_gateway_port(source, 8003);

        assert!(updated.contains("      port: 8003"));
        assert!(!updated.contains("      port: 8001"));
    }

    #[test]
    fn syncs_canic_sections_and_preserves_other_top_level_sections() {
        let source = "canisters:\n  - name: old\n\nnetworks:\n  - name: local\n    mode: managed\n    gateway:\n      bind: 127.0.0.1\n      port: 8009\n\nenvironments:\n  - name: old\n    network: local\n    canisters: [old]\n";
        let canisters = vec!["root".to_string(), "app".to_string()];
        let environments = BTreeMap::from([(
            "test".to_string(),
            vec!["root".to_string(), "app".to_string()],
        )]);

        let updated = sync_canic_sections(source, &canisters, &environments);

        assert!(updated.starts_with("canisters:\n  - name: root\n"));
        assert!(
            updated.contains(
                "            - cargo run -q -p canic-host --example build_artifact -- app"
            )
        );
        assert!(updated.contains(
            "environments:\n  - name: test\n    network: local\n    canisters: [root, app]"
        ));
        assert!(updated.contains("networks:\n  - name: local\n    mode: managed"));
        assert!(!updated.contains("- name: old"));
    }

    #[test]
    fn renders_empty_canic_sections_for_empty_project_specs() {
        let updated = sync_canic_sections("", &[], &BTreeMap::new());

        assert_eq!(updated, "canisters: []\n\nenvironments: []\n");
    }

    #[test]
    fn discovers_split_source_fleet_configs_for_icp_sync() {
        let root = temp_dir("canic-icp-sync-split-source");
        let config = root.join("backend/fleets/toko/canic.toml");
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

        let spec = discover_project_spec(&root, Some("toko"), None).expect("discover spec");

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
    fn icp_sync_rejects_missing_fleet_configs() {
        let root = temp_dir("canic-icp-sync-missing");
        fs::create_dir_all(&root).expect("create root");

        let err =
            discover_project_spec(&root, None, None).expect_err("missing configs should fail");
        let message = err.to_string();

        assert!(message.contains("no Canic fleet configs found under"));
        assert!(message.contains("fleets/<fleet>/canic.toml"));
        assert!(message.contains("backend/fleets/<fleet>/canic.toml"));
        fs::remove_dir_all(root).expect("clean temp dir");
    }
}
