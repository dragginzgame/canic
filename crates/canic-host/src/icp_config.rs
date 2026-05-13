use crate::workspace_discovery::discover_icp_root_from;
use std::{
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
            Self::Io(err) => write!(formatter, "{err}"),
        }
    }
}

impl Error for IcpConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::NoIcpRoot { .. } => None,
        }
    }
}

impl From<std::io::Error> for IcpConfigError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
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

fn current_icp_root() -> Result<PathBuf, IcpConfigError> {
    let start = std::env::current_dir()?;
    discover_icp_root_from(&start).ok_or(IcpConfigError::NoIcpRoot { start })
}

fn configured_local_gateway_port_from_root(root: &Path) -> Result<u16, IcpConfigError> {
    let source = fs::read_to_string(root.join(ICP_CONFIG_FILE))?;
    Ok(configured_local_gateway_port_from_source(&source))
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
}
