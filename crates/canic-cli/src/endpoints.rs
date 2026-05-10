use crate::{
    args::{default_icp, flag_arg, local_network, parse_matches, print_help_or_version, value_arg},
    version_text,
};
use canic_host::{icp::IcpCli, release_set::icp_root};
use clap::Command as ClapCommand;
use serde::Serialize;
use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

const CANDID_SERVICE_METADATA: &str = "candid:service";
const HELP_AFTER: &str = "\
Examples:
  canic endpoints app
  canic endpoints app --environment demo
  canic endpoints tl4x7-vh777-77776-aaacq-cai --role app --environment demo";

///
/// EndpointsCommandError
///

#[derive(Debug, ThisError)]
pub enum EndpointsCommandError {
    #[error("{0}")]
    Usage(String),

    #[error("canister interface did not contain a service block")]
    MissingService,

    #[error(
        "live metadata was unavailable for {canister} and no local Candid artifact could be resolved; pass a canister role with `--role <role>` or a Candid file with `--did <path>`"
    )]
    NoInterfaceArtifact { canister: String },

    #[error("local Candid artifact not found for role {role}; looked under {root}")]
    MissingRoleArtifact { role: String, root: String },

    #[error("failed to read local Candid artifact {path}: {source}")]
    ReadDid {
        path: String,
        source: std::io::Error,
    },

    #[error("failed to render endpoint output: {0}")]
    Json(#[from] serde_json::Error),
}

///
/// EndpointsOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct EndpointsOptions {
    canister: String,
    role: Option<String>,
    did: Option<PathBuf>,
    environment: Option<String>,
    network: Option<String>,
    icp: String,
    json: bool,
}

impl EndpointsOptions {
    fn parse<I>(args: I) -> Result<Self, EndpointsCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches =
            parse_matches(command(), args).map_err(|_| EndpointsCommandError::Usage(usage()))?;
        Ok(Self {
            canister: string_value(&matches, "canister").expect("clap requires canister"),
            role: string_value(&matches, "role"),
            did: string_value(&matches, "did").map(PathBuf::from),
            environment: string_value(&matches, "environment"),
            network: string_value(&matches, "network"),
            icp: string_value(&matches, "icp").unwrap_or_else(default_icp),
            json: matches.get_flag("json"),
        })
    }

    fn artifact_role(&self) -> Option<&str> {
        self.role.as_deref().or_else(|| {
            if is_principal_like(&self.canister) {
                None
            } else {
                Some(self.canister.as_str())
            }
        })
    }
}

///
/// EndpointReport
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct EndpointReport {
    source: String,
    endpoints: Vec<String>,
}

/// Run the canister endpoint listing command.
pub fn run<I>(args: I) -> Result<(), EndpointsCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    let options = EndpointsOptions::parse(args)?;
    let report = endpoint_report(&options)?;
    if options.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("{}", report.endpoints.join("\n"));
    }
    Ok(())
}

fn endpoint_report(options: &EndpointsOptions) -> Result<EndpointReport, EndpointsCommandError> {
    if let Some(path) = &options.did {
        let candid = read_did(path)?;
        return Ok(EndpointReport {
            source: path.display().to_string(),
            endpoints: parse_candid_service_methods(&candid)?,
        });
    }

    if let Ok(candid) = read_live_candid(options) {
        return Ok(EndpointReport {
            source: format!("{} metadata", options.canister),
            endpoints: parse_candid_service_methods(&candid)?,
        });
    }

    let Some(role) = options.artifact_role() else {
        return Err(EndpointsCommandError::NoInterfaceArtifact {
            canister: options.canister.clone(),
        });
    };
    let path = resolve_role_did(options, role)?;
    let candid = read_did(&path)?;
    Ok(EndpointReport {
        source: path.display().to_string(),
        endpoints: parse_candid_service_methods(&candid)?,
    })
}

fn read_live_candid(
    options: &EndpointsOptions,
) -> Result<String, canic_host::icp::IcpCommandError> {
    IcpCli::new(
        &options.icp,
        options.environment.clone(),
        options.network.clone(),
    )
    .canister_metadata_output(&options.canister, CANDID_SERVICE_METADATA)
}

fn resolve_role_did(
    options: &EndpointsOptions,
    role: &str,
) -> Result<PathBuf, EndpointsCommandError> {
    let root = icp_root().unwrap_or_else(|_| PathBuf::from("."));
    for network in artifact_network_candidates(options) {
        let path = root
            .join(".icp")
            .join(&network)
            .join("canisters")
            .join(role)
            .join(format!("{role}.did"));
        if path.is_file() {
            return Ok(path);
        }
    }

    Err(EndpointsCommandError::MissingRoleArtifact {
        role: role.to_string(),
        root: root.display().to_string(),
    })
}

fn artifact_network_candidates(options: &EndpointsOptions) -> Vec<String> {
    let mut networks = Vec::new();
    if let Some(network) = &options.network {
        networks.push(network.clone());
    }
    if let Some(environment) = &options.environment {
        networks.push(environment.clone());
    }
    networks.push(local_network());
    networks.sort();
    networks.dedup();
    networks
}

fn read_did(path: &Path) -> Result<String, EndpointsCommandError> {
    fs::read_to_string(path).map_err(|source| EndpointsCommandError::ReadDid {
        path: path.display().to_string(),
        source,
    })
}

fn parse_candid_service_methods(candid: &str) -> Result<Vec<String>, EndpointsCommandError> {
    let Some(service_start) = find_service_body_start(candid) else {
        return Err(EndpointsCommandError::MissingService);
    };

    let mut methods = Vec::new();
    let mut depth = 0usize;
    let mut at_line_start = true;
    let mut chars = candid[service_start..].char_indices().peekable();
    while let Some((_, ch)) = chars.next() {
        match ch {
            '{' => {
                depth += 1;
                at_line_start = false;
            }
            '}' => {
                if depth == 0 {
                    break;
                }
                depth -= 1;
                at_line_start = false;
                if depth == 0 {
                    break;
                }
            }
            '\n' => {
                at_line_start = true;
            }
            _ if depth == 1 && at_line_start && !ch.is_whitespace() => {
                let rest = remaining_line(candid, service_start, ch, &mut chars);
                if let Some(method) = parse_service_method_line(&rest) {
                    methods.push(method);
                }
                at_line_start = true;
            }
            _ if depth == 1 && at_line_start && ch.is_whitespace() => {}
            _ => {
                at_line_start = false;
            }
        }
    }

    Ok(methods)
}

fn find_service_body_start(candid: &str) -> Option<usize> {
    let mut search_from = 0usize;
    while let Some(offset) = candid[search_from..].find("service") {
        let service_index = search_from + offset;
        if !is_word_boundary(candid, service_index, "service") {
            search_from = service_index + "service".len();
            continue;
        }
        let service_tail = &candid[service_index..];
        let body_search_start = service_tail
            .find("->")
            .map_or(service_index, |offset| service_index + offset + "->".len());
        let brace_offset = candid[body_search_start..].find('{')?;
        return Some(body_search_start + brace_offset);
    }
    None
}

fn is_word_boundary(text: &str, index: usize, word: &str) -> bool {
    let before = text[..index].chars().next_back();
    let after = text[index + word.len()..].chars().next();
    before.is_none_or(|ch| !is_identifier_char(ch))
        && after.is_none_or(|ch| !is_identifier_char(ch))
}

const fn is_identifier_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

fn remaining_line(
    candid: &str,
    service_start: usize,
    first: char,
    chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
) -> String {
    let mut line = first.to_string();
    while let Some((index, ch)) = chars.peek().copied() {
        if ch == '\n' {
            break;
        }
        chars.next();
        let absolute = service_start + index;
        if candid.is_char_boundary(absolute) {
            line.push(ch);
        }
    }
    line
}

fn parse_service_method_line(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    if trimmed.starts_with("//") || trimmed.starts_with('}') {
        return None;
    }
    if let Some(stripped) = trimmed.strip_prefix('"') {
        let end = stripped.find('"')?;
        let name = &stripped[..end];
        let after = stripped[end + 1..].trim_start();
        return after.starts_with(':').then(|| name.to_string());
    }
    let end = trimmed
        .char_indices()
        .find_map(|(index, ch)| (ch.is_whitespace() || ch == ':').then_some(index))?;
    let name = &trimmed[..end];
    let after = trimmed[end..].trim_start();
    (!name.is_empty() && after.starts_with(':')).then(|| name.to_string())
}

fn is_principal_like(value: &str) -> bool {
    value.contains('-')
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
}

fn string_value(matches: &clap::ArgMatches, id: &str) -> Option<String> {
    matches.try_get_one::<String>(id).ok().flatten().cloned()
}

fn command() -> ClapCommand {
    ClapCommand::new("endpoints")
        .bin_name("canic endpoints")
        .disable_help_flag(true)
        .about("List callable methods exposed by a canister Candid interface")
        .arg(
            value_arg("canister")
                .value_name("canister-or-role")
                .required(true)
                .help("Canister name, principal, or local role name to inspect"),
        )
        .arg(
            value_arg("role")
                .long("role")
                .value_name("role")
                .help("Local canister role to use for artifact fallback"),
        )
        .arg(
            value_arg("did")
                .long("did")
                .value_name("path")
                .help("Read endpoints from a specific Candid file"),
        )
        .arg(
            value_arg("environment")
                .long("environment")
                .short('e')
                .value_name("name")
                .help("ICP CLI environment for live metadata lookup"),
        )
        .arg(
            value_arg("network")
                .long("network")
                .value_name("name")
                .help("ICP CLI network for live metadata lookup"),
        )
        .arg(
            value_arg("icp")
                .long("icp")
                .value_name("path")
                .help("Path to the icp executable"),
        )
        .arg(flag_arg("json").long("json").help("Print JSON output"))
        .after_help(HELP_AFTER)
}

fn usage() -> String {
    let mut command = command();
    command.render_help().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const CANDID: &str = r#"
type Nested = record { field : text };
service : (record { init : text }) -> {
  canic_ready : () -> (bool) query;
  "icrc10-supported-standards" : () -> (vec record { text; text }) query;
  canic_update : (Nested) -> (
      variant { Ok; Err : text },
    );
}
"#;

    // Ensure generated Candid service files can be reduced to endpoint names.
    #[test]
    fn parses_candid_service_methods() {
        let methods = super::parse_candid_service_methods(CANDID).expect("parse methods");

        assert_eq!(
            methods,
            vec!["canic_ready", "icrc10-supported-standards", "canic_update"]
        );
    }

    // Ensure principal arguments require an explicit role for local artifact fallback.
    #[test]
    fn principal_arguments_do_not_guess_role() {
        let options = EndpointsOptions {
            canister: "tl4x7-vh777-77776-aaacq-cai".to_string(),
            role: None,
            did: None,
            environment: Some("demo".to_string()),
            network: None,
            icp: "icp".to_string(),
            json: false,
        };

        assert_eq!(options.artifact_role(), None);
    }

    // Ensure endpoint options parse local and live lookup controls.
    #[test]
    fn parses_endpoint_options() {
        let options = EndpointsOptions::parse([
            OsString::from("app"),
            OsString::from("--environment"),
            OsString::from("demo"),
            OsString::from("--network"),
            OsString::from("local"),
            OsString::from("--role"),
            OsString::from("app"),
            OsString::from("--icp"),
            OsString::from("/bin/icp"),
            OsString::from("--json"),
        ])
        .expect("parse options");

        assert_eq!(options.canister, "app");
        assert_eq!(options.environment.as_deref(), Some("demo"));
        assert_eq!(options.network.as_deref(), Some("local"));
        assert_eq!(options.role.as_deref(), Some("app"));
        assert_eq!(options.icp, "/bin/icp");
        assert!(options.json);
    }
}
