use crate::{
    install_root::{
        current_canic_project_root, discover_project_canic_config_choices, project_fleet_roots,
    },
    release_set::{
        ConfiguredLocalNetwork, configured_fleet_name, configured_fleet_roles,
        configured_local_network, icp_root,
    },
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
    pub icp_root: PathBuf,
    pub changed: bool,
    pub canisters: Vec<String>,
    pub environments: Vec<String>,
}

/// Return the configured local ICP gateway port, falling back to ICP's default.
pub fn configured_local_gateway_port() -> Result<u16, IcpConfigError> {
    let root = current_icp_root()?;
    configured_local_gateway_port_from_root(&root)
}

/// Return the configured local ICP gateway port for one ICP project root.
pub fn configured_local_gateway_port_from_root(root: &Path) -> Result<u16, IcpConfigError> {
    let source = fs::read_to_string(root.join(ICP_CONFIG_FILE))?;
    Ok(configured_local_gateway_port_from_source(&source))
}

/// Set the local ICP gateway port in one ICP project root.
pub fn set_configured_local_gateway_port_in_root(
    root: &Path,
    port: u16,
) -> Result<PathBuf, IcpConfigError> {
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
    let root = resolve_current_canic_icp_root()?;
    let path = root.join(ICP_CONFIG_FILE);
    let source = match fs::read_to_string(&path) {
        Ok(source) => source,
        Err(err) if err.kind() == ErrorKind::NotFound => String::new(),
        Err(err) => return Err(err.into()),
    };
    let spec = discover_project_spec(&root, fleet_filter)?;
    let updated = sync_canic_sections(
        &source,
        &spec.canisters,
        &spec.environments,
        spec.local_network,
    );
    let changed = updated != source;
    if changed {
        fs::write(&path, updated)?;
    }

    Ok(IcpProjectSyncReport {
        path,
        icp_root: root,
        changed,
        canisters: spec.canisters,
        environments: spec.environments.into_keys().collect(),
    })
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
    local_network: ConfiguredLocalNetwork,
}

fn discover_project_spec(
    root: &Path,
    fleet_filter: Option<&str>,
) -> Result<CanicIcpSpec, IcpConfigError> {
    let choices = discover_project_canic_config_choices(root)
        .map_err(|err| IcpConfigError::Config(err.to_string()))?;
    if choices.is_empty() {
        return Err(IcpConfigError::Config(format!(
            "no Canic fleet configs found under {}\nCreate fleets/<fleet>/canic.toml, then rerun `canic replica start` or `canic fleet sync --fleet <fleet>`.",
            display_project_fleet_roots(root)
        )));
    }

    let mut canisters = Vec::<String>::new();
    let mut seen_canisters = BTreeSet::<String>::new();
    let mut environments = BTreeMap::<String, Vec<String>>::new();
    let mut local_network = ConfiguredLocalNetwork::default();
    let mut ii_source = None::<PathBuf>;
    let mut nns_source = None::<PathBuf>;
    let mut matched_filter = fleet_filter.is_none();

    for config_path in choices {
        let fleet = configured_fleet_name(&config_path)
            .map_err(|err| IcpConfigError::Config(err.to_string()))?;
        if fleet_filter.is_some_and(|filter| filter == fleet) {
            matched_filter = true;
        }

        let roles = configured_fleet_roles(&config_path)
            .map_err(|err| IcpConfigError::Config(err.to_string()))?;
        let network = configured_local_network(&config_path)
            .map_err(|err| IcpConfigError::Config(err.to_string()))?;
        merge_local_network_flag(
            "ii",
            &mut local_network.ii,
            &mut ii_source,
            network.ii,
            &config_path,
        )?;
        merge_local_network_flag(
            "nns",
            &mut local_network.nns,
            &mut nns_source,
            network.nns,
            &config_path,
        )?;
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
        local_network,
    })
}

fn merge_local_network_flag(
    name: &str,
    current: &mut Option<bool>,
    current_source: &mut Option<PathBuf>,
    next: Option<bool>,
    source_path: &Path,
) -> Result<(), IcpConfigError> {
    let Some(next) = next else {
        return Ok(());
    };

    match current {
        Some(existing) if *existing != next => {
            let previous = current_source.as_ref().map_or_else(
                || "<unknown>".to_string(),
                |path| path.display().to_string(),
            );
            Err(IcpConfigError::Config(format!(
                "conflicting [fleet.local].{name} values across fleet configs:\n- {previous} -> {existing}\n- {} -> {next}\nUse one shared value for the local ICP network or remove the duplicate override.",
                source_path.display()
            )))
        }
        Some(_) => Ok(()),
        None => {
            *current = Some(next);
            *current_source = Some(source_path.to_path_buf());
            Ok(())
        }
    }
}

fn display_project_fleet_roots(root: &Path) -> String {
    project_fleet_roots(root)
        .into_iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(" or ")
}

fn sync_canic_sections(
    source: &str,
    canisters: &[String],
    environments: &BTreeMap<String, Vec<String>>,
    local_network: ConfiguredLocalNetwork,
) -> String {
    let without_canisters = remove_top_level_section(source, "canisters:");
    let without_environments = remove_top_level_section(&without_canisters, "environments:");
    let synced_networks = sync_local_network_flags(&without_environments, local_network);
    let (networks, rest) = take_top_level_section(&synced_networks, "networks:");
    let mut sections = vec![render_canisters_section(canisters)];
    if let Some(networks) = networks {
        sections.push(networks);
    }
    sections.push(render_environments_section(environments));
    let rest = rest.trim();
    if !rest.is_empty() {
        sections.push(rest.to_string());
    }

    let mut updated = sections.join("\n\n");
    updated.push('\n');
    updated
}

fn take_top_level_section(source: &str, header: &str) -> (Option<String>, String) {
    let mut lines = source.lines().map(str::to_string).collect::<Vec<_>>();
    let line_refs = lines.iter().map(String::as_str).collect::<Vec<_>>();
    let Some((start, end)) = top_level_section(&line_refs, header) else {
        return (None, source.to_string());
    };

    let section = lines[start..end].join("\n");
    lines.drain(start..end);

    let rest = compact_blank_lines(lines).join("\n");
    (Some(section), rest)
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

    compact_blank_lines(lines).join("\n")
}

fn compact_blank_lines(lines: Vec<String>) -> Vec<String> {
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

    compacted
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

fn sync_local_network_flags(source: &str, local_network: ConfiguredLocalNetwork) -> String {
    let mut updated = source.to_string();
    if let Some(ii) = local_network.ii {
        updated = upsert_local_network_bool(&updated, "ii", ii);
    }
    if let Some(nns) = local_network.nns {
        updated = upsert_local_network_bool(&updated, "nns", nns);
    }
    updated
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
        top_level_section(&line_refs, "networks:")
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

fn upsert_local_network_bool(source: &str, key: &str, value: bool) -> String {
    let port = configured_local_gateway_port_from_source(source);
    let source = upsert_local_gateway_port(source, port);
    let had_trailing_newline = source.ends_with('\n');
    let mut lines = source.lines().map(str::to_string).collect::<Vec<_>>();

    let local_block = {
        let line_refs = lines.iter().map(String::as_str).collect::<Vec<_>>();
        local_network_block(&line_refs)
    };
    let Some((start, end)) = local_block else {
        return source;
    };

    let matcher = format!("{key}:");
    if let Some(index) = (start..end).find(|index| lines[*index].trim().starts_with(&matcher)) {
        let indent = line_indent(&lines[index]);
        lines[index] = format!("{}{}: {value}", " ".repeat(indent), key);
        return join_lines(lines, had_trailing_newline);
    }

    let insert_at = (start + 1..end)
        .find(|index| lines[*index].trim() == "gateway:")
        .unwrap_or(end);
    lines.insert(insert_at, format!("    {key}: {value}"));
    join_lines(lines, had_trailing_newline)
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
    fn ignores_nested_networks_keys_when_reading_local_gateway_port() {
        let source = "canisters:\n  - name: root\n    metadata:\n      networks:\n        - local\n\nnetworks:\n  - name: local\n    mode: managed\n    gateway:\n      bind: 127.0.0.1\n      port: 8010\n";

        assert_eq!(configured_local_gateway_port_from_source(source), 8010);
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

        let updated = sync_canic_sections(
            source,
            &canisters,
            &environments,
            ConfiguredLocalNetwork::default(),
        );

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
        assert!(updated.find("networks:") < updated.find("environments:"));
        assert!(!updated.contains("- name: old"));
    }

    #[test]
    fn syncs_requested_local_network_flags_into_local_icp_network() {
        let source = "canisters:\n  - name: old\n\nnetworks:\n  - name: local\n    mode: managed\n    gateway:\n      bind: 127.0.0.1\n      port: 8009\n\nenvironments:\n  - name: old\n    network: local\n    canisters: [old]\n";
        let canisters = vec!["root".to_string()];
        let environments = BTreeMap::from([("test".to_string(), vec!["root".to_string()])]);

        let updated = sync_canic_sections(
            source,
            &canisters,
            &environments,
            ConfiguredLocalNetwork {
                ii: Some(true),
                nns: Some(true),
            },
        );

        assert!(updated.contains("    ii: true"));
        assert!(updated.contains("    nns: true"));
        assert!(updated.contains("      port: 8009"));
    }

    #[test]
    fn renders_empty_canic_sections_for_empty_project_specs() {
        let updated =
            sync_canic_sections("", &[], &BTreeMap::new(), ConfiguredLocalNetwork::default());

        assert_eq!(updated, "canisters: []\n\nenvironments: []\n");
    }

    #[test]
    fn discovers_root_fleet_configs_for_icp_sync() {
        let root = temp_dir("canic-icp-sync-root-fleets");
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
        assert_eq!(spec.local_network, ConfiguredLocalNetwork::default());
        fs::remove_dir_all(root).expect("clean temp dir");
    }

    #[test]
    fn discovers_local_network_flags_for_icp_sync() {
        let root = temp_dir("canic-icp-sync-local-network");
        let config = root.join("fleets/toko/canic.toml");
        fs::create_dir_all(config.parent().expect("config parent")).expect("create config parent");
        fs::write(
            &config,
            r#"
[fleet]
name = "toko"

[fleet.local]
ii = true
nns = true

[subnets.prime.canisters.root]
kind = "root"
"#,
        )
        .expect("write config");

        let spec = discover_project_spec(&root, Some("toko")).expect("discover spec");

        assert_eq!(
            spec.local_network,
            ConfiguredLocalNetwork {
                ii: Some(true),
                nns: Some(true),
            }
        );
        fs::remove_dir_all(root).expect("clean temp dir");
    }

    #[test]
    fn rejects_conflicting_local_network_flags_across_fleet_configs() {
        let root = temp_dir("canic-icp-sync-conflicting-local-network");
        let demo = root.join("fleets/demo/canic.toml");
        let staging = root.join("fleets/staging/canic.toml");
        fs::create_dir_all(demo.parent().expect("demo parent")).expect("create demo parent");
        fs::create_dir_all(staging.parent().expect("staging parent"))
            .expect("create staging parent");
        fs::write(
            &demo,
            r#"
[fleet]
name = "demo"

[fleet.local]
ii = true

[subnets.prime.canisters.root]
kind = "root"
"#,
        )
        .expect("write demo config");
        fs::write(
            &staging,
            r#"
[fleet]
name = "staging"

[fleet.local]
ii = false

[subnets.prime.canisters.root]
kind = "root"
"#,
        )
        .expect("write staging config");

        let err = discover_project_spec(&root, None).expect_err("conflicting local network flags");

        assert!(
            err.to_string()
                .contains("conflicting [fleet.local].ii values")
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
    fn icp_sync_rejects_missing_fleet_configs() {
        let root = temp_dir("canic-icp-sync-missing");
        fs::create_dir_all(&root).expect("create root");

        let err = discover_project_spec(&root, None).expect_err("missing configs should fail");
        let message = err.to_string();

        assert!(message.contains("no Canic fleet configs found under"));
        assert!(message.contains("fleets/<fleet>/canic.toml"));
        fs::remove_dir_all(root).expect("clean temp dir");
    }
}
