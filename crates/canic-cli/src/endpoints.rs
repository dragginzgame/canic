use crate::{
    args::{
        default_icp, flag_arg, internal_icp_arg, internal_network_arg, local_network,
        parse_matches, print_help_or_version, value_arg,
    },
    version_text,
};
use canic_backup::discovery::{RegistryEntry, parse_registry_entries};
use canic_host::{
    icp::IcpCli, install_root::read_named_fleet_install_state, release_set::icp_root, replica_query,
};
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
  canic endpoints test app
  canic endpoints test scale_hub --json
  canic endpoints test tl4x7-vh777-77776-aaacq-cai";

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
        "live metadata was unavailable for {canister} in fleet {fleet} and no local Candid artifact could be resolved"
    )]
    NoInterfaceArtifact { fleet: String, canister: String },

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
    fleet: String,
    canister: String,
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
            fleet: string_value(&matches, "fleet").expect("clap requires fleet"),
            canister: string_value(&matches, "canister").expect("clap requires canister"),
            network: string_value(&matches, "network"),
            icp: string_value(&matches, "icp").unwrap_or_else(default_icp),
            json: matches.get_flag("json"),
        })
    }
}

///
/// EndpointReport
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct EndpointReport {
    source: String,
    endpoints: Vec<EndpointEntry>,
}

///
/// EndpointEntry
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct EndpointEntry {
    name: String,
    arguments: Vec<String>,
}

impl EndpointEntry {
    fn render(&self) -> String {
        format!("{}({})", self.name, self.arguments.join(", "))
    }
}

///
/// EndpointTarget
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct EndpointTarget {
    canister: String,
    role: Option<String>,
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
        println!(
            "{}",
            report
                .endpoints
                .iter()
                .map(EndpointEntry::render)
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
    Ok(())
}

fn endpoint_report(options: &EndpointsOptions) -> Result<EndpointReport, EndpointsCommandError> {
    let target = resolve_endpoint_target(options);
    if let Ok(target) = &target
        && let Ok(candid) = read_live_candid(options, target)
    {
        return Ok(EndpointReport {
            source: format!("{} metadata", options.canister),
            endpoints: parse_candid_service_endpoints(&candid)?,
        });
    }

    let role = target
        .ok()
        .and_then(|target| target.role)
        .or_else(|| (!is_principal_like(&options.canister)).then(|| options.canister.clone()));
    let Some(role) = role else {
        return Err(EndpointsCommandError::NoInterfaceArtifact {
            fleet: options.fleet.clone(),
            canister: options.canister.clone(),
        });
    };
    let path = resolve_role_did(options, &role)?;
    let candid = read_did(&path)?;
    Ok(EndpointReport {
        source: path.display().to_string(),
        endpoints: parse_candid_service_endpoints(&candid)?,
    })
}

fn read_live_candid(
    options: &EndpointsOptions,
    target: &EndpointTarget,
) -> Result<String, Box<dyn std::error::Error>> {
    Ok(IcpCli::new(&options.icp, None, options.network.clone())
        .canister_metadata_output(&target.canister, CANDID_SERVICE_METADATA)?)
}

fn resolve_endpoint_target(
    options: &EndpointsOptions,
) -> Result<EndpointTarget, Box<dyn std::error::Error>> {
    if is_principal_like(&options.canister) {
        let role = load_fleet_registry(options).ok().and_then(|registry| {
            registry
                .into_iter()
                .find(|entry| entry.pid == options.canister)
                .and_then(|entry| entry.role)
        });
        return Ok(EndpointTarget {
            canister: options.canister.clone(),
            role,
        });
    }

    let registry = load_fleet_registry(options)?;
    let entry = registry
        .iter()
        .find(|entry| entry.role.as_deref() == Some(options.canister.as_str()))
        .ok_or_else(|| -> Box<dyn std::error::Error> {
            format!(
                "role {} was not found in fleet {}",
                options.canister, options.fleet
            )
            .into()
        })?;
    Ok(EndpointTarget {
        canister: entry.pid.clone(),
        role: entry.role.clone(),
    })
}

fn load_fleet_registry(
    options: &EndpointsOptions,
) -> Result<Vec<RegistryEntry>, Box<dyn std::error::Error>> {
    let network = state_network(options);
    let state = read_named_fleet_install_state(&network, &options.fleet)?.ok_or_else(|| {
        format!(
            "fleet {} is not installed on network {network}",
            options.fleet
        )
    })?;
    let registry_json = if replica_query::should_use_local_replica_query(options.network.as_deref())
    {
        replica_query::query_subnet_registry_json(
            options.network.as_deref(),
            &state.root_canister_id,
        )?
    } else {
        IcpCli::new(&options.icp, None, options.network.clone()).canister_call_output(
            &state.root_canister_id,
            "canic_subnet_registry",
            Some("json"),
        )?
    };
    Ok(parse_registry_entries(&registry_json)?)
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
    networks.push(local_network());
    networks.sort();
    networks.dedup();
    networks
}

fn state_network(options: &EndpointsOptions) -> String {
    options.network.clone().unwrap_or_else(local_network)
}

fn read_did(path: &Path) -> Result<String, EndpointsCommandError> {
    fs::read_to_string(path).map_err(|source| EndpointsCommandError::ReadDid {
        path: path.display().to_string(),
        source,
    })
}

fn parse_candid_service_endpoints(
    candid: &str,
) -> Result<Vec<EndpointEntry>, EndpointsCommandError> {
    let Some(service_start) = find_service_body_start(candid) else {
        return Err(EndpointsCommandError::MissingService);
    };

    let mut endpoints = Vec::new();
    let mut depth = 0usize;
    let mut declaration = String::new();
    for (index, ch) in candid[service_start..].char_indices() {
        match ch {
            '{' => {
                depth += 1;
                if depth > 1 {
                    declaration.push(ch);
                }
            }
            '}' => {
                if depth == 0 {
                    break;
                }
                if depth > 1 {
                    declaration.push(ch);
                }
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            ';' if depth == 1 => {
                if let Some(endpoint) = parse_service_method_declaration(&declaration) {
                    endpoints.push(endpoint);
                }
                declaration.clear();
            }
            _ if depth >= 1 => {
                let absolute = service_start + index;
                if candid.is_char_boundary(absolute) {
                    declaration.push(ch);
                }
            }
            _ => {}
        }
    }

    Ok(endpoints)
}

fn find_service_body_start(candid: &str) -> Option<usize> {
    let mut brace_depth = 0usize;
    for (service_index, ch) in candid.char_indices() {
        match ch {
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            's' if brace_depth == 0
                && candid[service_index..].starts_with("service")
                && is_word_boundary(candid, service_index, "service") =>
            {
                let service_tail_start = service_index + "service".len();
                let service_tail = &candid[service_tail_start..];
                if !service_tail.trim_start().starts_with(':') {
                    continue;
                }

                if let Some(body_start) = find_service_body_brace(service_tail) {
                    return Some(service_tail_start + body_start);
                }
            }
            _ => {}
        }
    }
    None
}

fn find_service_body_brace(service_tail: &str) -> Option<usize> {
    let mut paren_depth = 0usize;
    for (index, ch) in service_tail.char_indices() {
        match ch {
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '{' if paren_depth == 0 => return Some(index),
            _ => {}
        }
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

fn parse_service_method_declaration(declaration: &str) -> Option<EndpointEntry> {
    let trimmed = declaration.trim_start();
    if trimmed.starts_with("//") || trimmed.starts_with('}') {
        return None;
    }
    let (name, after_name) = if let Some(stripped) = trimmed.strip_prefix('"') {
        let end = stripped.find('"')?;
        let name = &stripped[..end];
        let after = stripped[end + 1..].trim_start();
        (name, after)
    } else {
        let end = trimmed
            .char_indices()
            .find_map(|(index, ch)| (ch.is_whitespace() || ch == ':').then_some(index))?;
        (&trimmed[..end], trimmed[end..].trim_start())
    };
    if name.is_empty() || !after_name.starts_with(':') {
        return None;
    }
    let signature = after_name[1..].trim_start();
    let arguments = parse_argument_list(extract_parenthesized(signature)?)?;
    Some(EndpointEntry {
        name: name.to_string(),
        arguments,
    })
}

fn extract_parenthesized(signature: &str) -> Option<&str> {
    let mut depth = 0usize;
    let mut end = None;
    for (index, ch) in signature.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    end = Some(index + ch.len_utf8());
                    break;
                }
            }
            _ if depth == 0 && !ch.is_whitespace() => return None,
            _ => {}
        }
    }
    end.map(|end| &signature[..end])
}

fn parse_argument_list(parenthesized: &str) -> Option<Vec<String>> {
    let inner = parenthesized.strip_prefix('(')?.strip_suffix(')')?;
    Some(split_top_level_candid_arguments(inner))
}

fn split_top_level_candid_arguments(fragment: &str) -> Vec<String> {
    let mut arguments = Vec::new();
    let mut start = 0usize;
    let mut paren_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (index, ch) in fragment.char_indices() {
        if in_string {
            escaped = ch == '\\' && !escaped;
            if ch == '"' && !escaped {
                in_string = false;
            }
            if ch != '\\' {
                escaped = false;
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            ',' if paren_depth == 0 && brace_depth == 0 && bracket_depth == 0 => {
                push_candid_argument(&mut arguments, &fragment[start..index]);
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }
    push_candid_argument(&mut arguments, &fragment[start..]);
    arguments
}

fn push_candid_argument(arguments: &mut Vec<String>, fragment: &str) {
    let argument = normalize_candid_fragment(fragment.trim());
    if !argument.is_empty() {
        arguments.push(argument);
    }
}

fn normalize_candid_fragment(fragment: &str) -> String {
    fragment.split_whitespace().collect::<Vec<_>>().join(" ")
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
            value_arg("fleet")
                .value_name("fleet")
                .required(true)
                .help("Fleet name to inspect"),
        )
        .arg(
            value_arg("canister")
                .value_name("canister-or-role")
                .required(true)
                .help("Canister principal or role name to inspect"),
        )
        .arg(internal_network_arg())
        .arg(internal_icp_arg())
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

    // Ensure generated Candid service files can be reduced to endpoint signatures.
    #[test]
    fn parses_candid_service_endpoints() {
        let endpoints = super::parse_candid_service_endpoints(CANDID).expect("parse endpoints");

        assert_eq!(
            endpoints,
            vec![
                EndpointEntry {
                    name: "canic_ready".to_string(),
                    arguments: Vec::new(),
                },
                EndpointEntry {
                    name: "icrc10-supported-standards".to_string(),
                    arguments: Vec::new(),
                },
                EndpointEntry {
                    name: "canic_update".to_string(),
                    arguments: vec!["Nested".to_string()],
                },
            ]
        );
    }

    // Ensure multiline argument lists remain attached to the endpoint.
    #[test]
    fn parses_multiline_endpoint_arguments() {
        let candid = r"
service : {
  import : (
    record {
      payload : text;
    },
  ) -> (variant { Ok; Err : text });
}
";

        let endpoints = super::parse_candid_service_endpoints(candid).expect("parse endpoints");

        assert_eq!(
            endpoints,
            vec![EndpointEntry {
                name: "import".to_string(),
                arguments: vec!["record { payload : text; }".to_string()],
            }]
        );
    }

    // Ensure multiple arguments are split without breaking nested Candid types.
    #[test]
    fn parses_multiple_endpoint_arguments() {
        let candid = r"
service : {
  update : (opt text, record { items : vec record { id : nat; label : text } }, PageRequest) -> ();
}
";

        let endpoints = super::parse_candid_service_endpoints(candid).expect("parse endpoints");

        assert_eq!(
            endpoints,
            vec![EndpointEntry {
                name: "update".to_string(),
                arguments: vec![
                    "opt text".to_string(),
                    "record { items : vec record { id : nat; label : text } }".to_string(),
                    "PageRequest".to_string(),
                ],
            }]
        );
    }

    // Ensure fields named service before the top-level service do not confuse discovery.
    #[test]
    fn ignores_service_named_record_fields() {
        let candid = r#"
type Envelope = record {
  "service" : text;
  payload : text;
};
service : {
  ready : () -> (bool) query;
}
"#;

        let endpoints = super::parse_candid_service_endpoints(candid).expect("parse endpoints");

        assert_eq!(
            endpoints,
            vec![EndpointEntry {
                name: "ready".to_string(),
                arguments: Vec::new(),
            }]
        );
    }

    // Ensure plain output still renders a compact method signature.
    #[test]
    fn renders_plain_endpoint_signature() {
        let endpoint = EndpointEntry {
            name: "canic_log".to_string(),
            arguments: vec![
                "opt text".to_string(),
                "opt text".to_string(),
                "Level".to_string(),
                "PageRequest".to_string(),
            ],
        };

        assert_eq!(
            endpoint.render(),
            "canic_log(opt text, opt text, Level, PageRequest)"
        );
    }

    // Ensure endpoint options parse local and live lookup controls.
    #[test]
    fn parses_endpoint_options() {
        let options = EndpointsOptions::parse([
            OsString::from("test"),
            OsString::from("app"),
            OsString::from(crate::args::INTERNAL_NETWORK_OPTION),
            OsString::from("local"),
            OsString::from(crate::args::INTERNAL_ICP_OPTION),
            OsString::from("/bin/icp"),
            OsString::from("--json"),
        ])
        .expect("parse options");

        assert_eq!(options.fleet, "test");
        assert_eq!(options.canister, "app");
        assert_eq!(options.network.as_deref(), Some("local"));
        assert_eq!(options.icp, "/bin/icp");
        assert!(options.json);
    }

    // Ensure direct Candid-file selection is not part of fleet-scoped endpoint lookup.
    #[test]
    fn rejects_did_option() {
        let err = EndpointsOptions::parse([
            OsString::from("test"),
            OsString::from("app"),
            OsString::from("--did"),
            OsString::from("app.did"),
        ])
        .expect_err("did override should be removed");

        assert!(matches!(err, EndpointsCommandError::Usage(_)));
    }

    // Ensure explicit role fallback is not part of fleet-scoped endpoint lookup.
    #[test]
    fn rejects_role_option() {
        let err = EndpointsOptions::parse([
            OsString::from("test"),
            OsString::from("tl4x7-vh777-77776-aaacq-cai"),
            OsString::from("--role"),
            OsString::from("scale_hub"),
        ])
        .expect_err("role override should be removed");

        assert!(matches!(err, EndpointsCommandError::Usage(_)));
    }
}
