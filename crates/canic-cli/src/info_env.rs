//! Module: canic_cli::info_env
//!
//! Responsibility: render sourceable current-Fleet canister ID exports.
//! Does not own: Fleet state persistence, registry authority, or canister lifecycle changes.
//! Boundary: reads terminal current-Fleet inventory and renders shell or JSON output.

use crate::{
    cli::{
        clap::{
            flag_arg, parse_matches, path_option, render_usage, required_string,
            string_option_or_else, value_arg,
        },
        defaults::{default_icp, local_environment},
        globals::{internal_environment_arg, internal_icp_arg},
        help::print_help_or_version,
    },
    output, version_text,
};
use canic_host::{
    fleet_ensure::{CurrentFleetInventoryError, CurrentFleetResolution, resolve_current_fleet},
    icp::IcpCommandError,
    icp_config::{IcpConfigError, resolve_current_canic_icp_root},
    registry::RegistryEntry,
};
use clap::Command as ClapCommand;
use serde::Serialize;
use std::{
    collections::BTreeMap,
    ffi::OsString,
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

const HELP_AFTER: &str = "\
Examples:
  canic info env demo-local
  canic --environment academic info env demo-local > scripts/canister_ids.sh
  canic info env demo-local --json";

///
/// InfoEnvCommandError
///
/// CLI boundary error for resolving current Fleet state and rendering
/// `canic info env` output.
///

#[derive(Debug, ThisError)]
pub enum InfoEnvCommandError {
    #[error("{0}")]
    Usage(String),

    #[error("failed to resolve ICP project root: {0}")]
    IcpRoot(#[source] IcpConfigError),

    #[error(transparent)]
    CurrentFleet(#[from] CurrentFleetInventoryError),

    #[error(transparent)]
    Icp(#[from] IcpCommandError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

/// Renderable current-Fleet canister ID export payload.

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct InfoEnvReport {
    fleet: String,
    environment: String,
    bindings: Vec<InfoEnvBinding>,
}

/// One sourceable canister ID binding derived from registry state.

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct InfoEnvBinding {
    variable: String,
    role: Option<String>,
    canister_id: String,
    kind: Option<String>,
    parent_pid: Option<String>,
}

/// Parsed `canic info env` command options.

#[derive(Clone, Debug, Eq, PartialEq)]
struct InfoEnvOptions {
    fleet: String,
    json: bool,
    out: Option<PathBuf>,
    environment: String,
    icp: String,
}

impl InfoEnvOptions {
    fn parse<I>(args: I) -> Result<Self, InfoEnvCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(info_env_command(), args)
            .map_err(|_| InfoEnvCommandError::Usage(usage()))?;

        Ok(Self {
            fleet: required_string(&matches, "fleet"),
            json: matches.get_flag("json"),
            out: path_option(&matches, "out"),
            environment: string_option_or_else(&matches, "environment", local_environment),
            icp: string_option_or_else(&matches, "icp", default_icp),
        })
    }
}

/// Run the sourceable current-Fleet canister ID export command.
pub fn run<I>(args: I) -> Result<(), InfoEnvCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    let options = InfoEnvOptions::parse(args)?;
    let report = load_env_report(&options)?;
    write_env_report(&options, &report)
}

fn load_env_report(options: &InfoEnvOptions) -> Result<InfoEnvReport, InfoEnvCommandError> {
    let root = resolve_current_canic_icp_root().map_err(InfoEnvCommandError::IcpRoot)?;
    let resolution = resolve_info_env_fleet(options, &root)?;
    Ok(env_report(options, &resolution))
}

fn resolve_info_env_fleet(
    options: &InfoEnvOptions,
    icp_root: &Path,
) -> Result<CurrentFleetResolution, InfoEnvCommandError> {
    resolve_current_fleet(icp_root, &options.environment, &options.fleet)
        .map_err(InfoEnvCommandError::from)
}

fn env_report(options: &InfoEnvOptions, resolution: &CurrentFleetResolution) -> InfoEnvReport {
    InfoEnvReport {
        fleet: options.fleet.clone(),
        environment: options.environment.clone(),
        bindings: env_bindings(&resolution.registry.entries),
    }
}

fn env_bindings(entries: &[RegistryEntry]) -> Vec<InfoEnvBinding> {
    let mut entries = entries.to_vec();
    entries.sort_by(|left, right| {
        let left_base = binding_variable_base(left);
        let right_base = binding_variable_base(right);
        binding_base_rank(&left_base)
            .cmp(&binding_base_rank(&right_base))
            .then(left_base.cmp(&right_base))
            .then(left.pid.cmp(&right.pid))
    });

    let counts = binding_base_counts(&entries);
    let mut seen = BTreeMap::<String, usize>::new();
    entries
        .into_iter()
        .map(|entry| {
            let base = binding_variable_base(&entry);
            let index = seen.entry(base.clone()).or_default();
            *index += 1;
            InfoEnvBinding {
                variable: if counts.get(&base).copied().unwrap_or_default() > 1 {
                    format!("{base}_{index}")
                } else {
                    base
                },
                role: entry.role,
                canister_id: entry.pid,
                kind: None,
                parent_pid: entry.parent_pid,
            }
        })
        .collect()
}

fn binding_base_counts(entries: &[RegistryEntry]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for entry in entries {
        *counts.entry(binding_variable_base(entry)).or_default() += 1;
    }
    counts
}

fn binding_variable_base(entry: &RegistryEntry) -> String {
    format!(
        "CANIC_{}",
        entry
            .role
            .as_deref()
            .map_or_else(|| "CANISTER".to_string(), role_env_suffix)
    )
}

fn role_env_suffix(role: &str) -> String {
    let mut suffix = String::new();
    let mut last_was_separator = false;
    for ch in role.chars() {
        if ch.is_ascii_alphanumeric() {
            suffix.push(ch.to_ascii_uppercase());
            last_was_separator = false;
        } else if !suffix.is_empty() && !last_was_separator {
            suffix.push('_');
            last_was_separator = true;
        }
    }
    while suffix.ends_with('_') {
        suffix.pop();
    }
    if suffix.is_empty() {
        "CANISTER".to_string()
    } else {
        suffix
    }
}

fn binding_base_rank(base: &str) -> u8 {
    match base {
        "CANIC_FLEET_COORDINATOR" => 0,
        "CANIC_ROOT" => 1,
        _ => 2,
    }
}

fn write_env_report(
    options: &InfoEnvOptions,
    report: &InfoEnvReport,
) -> Result<(), InfoEnvCommandError> {
    if options.json {
        return output::write_pretty_json::<_, InfoEnvCommandError>(options.out.as_deref(), report);
    }

    output::write_text::<InfoEnvCommandError>(options.out.as_deref(), &render_shell_exports(report))
}

fn render_shell_exports(report: &InfoEnvReport) -> String {
    let mut lines = vec![
        format!("# canic info env {}", report.fleet),
        format!("# environment: {}", report.environment),
    ];
    lines.extend(report.bindings.iter().map(|binding| {
        format!(
            "export {}={}",
            binding.variable,
            shell_single_quote(&binding.canister_id)
        )
    }));
    lines.join("\n")
}

fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn usage() -> String {
    render_usage(info_env_command)
}

fn info_env_command() -> ClapCommand {
    ClapCommand::new("env")
        .bin_name("canic info env")
        .about("Print sourceable current Fleet canister ID exports")
        .disable_help_flag(true)
        .arg(
            value_arg("fleet")
                .value_name("fleet")
                .required(true)
                .help("Installed Fleet name to inspect"),
        )
        .arg(flag_arg("json").long("json"))
        .arg(value_arg("out").long("out").value_name("file"))
        .arg(internal_environment_arg())
        .arg(internal_icp_arg())
        .after_help(HELP_AFTER)
}

// -----------------------------------------------------------------------------
// Tests

#[cfg(test)]
mod tests {
    use super::*;

    const ROOT: &str = "aaaaa-aa";
    const COORDINATOR: &str = "rrkah-fqaaa-aaaaa-aaaaq-cai";
    const USER_HUB: &str = "renrk-eyaaa-aaaaa-aaada-cai";
    const USER_SHARD_A: &str = "rno2w-sqaaa-aaaaa-aaacq-cai";
    const USER_SHARD_B: &str = "ryjl3-tyaaa-aaaaa-aaaba-cai";

    fn registry_entry(pid: &str, role: Option<&str>) -> RegistryEntry {
        RegistryEntry {
            pid: pid.to_string(),
            role: role.map(str::to_string),
            parent_pid: None,
            module_hash: None,
            protocol_binding: None,
        }
    }

    #[test]
    fn parses_info_env_options() {
        let options = InfoEnvOptions::parse([
            OsString::from("demo-local"),
            OsString::from("--json"),
            OsString::from("--out"),
            OsString::from("ids.json"),
            OsString::from(crate::cli::globals::INTERNAL_ENVIRONMENT_OPTION),
            OsString::from("academic"),
            OsString::from(crate::cli::globals::INTERNAL_ICP_OPTION),
            OsString::from("/bin/icp"),
        ])
        .expect("parse info env options");

        assert_eq!(options.fleet, "demo-local");
        assert!(options.json);
        assert_eq!(options.out, Some(PathBuf::from("ids.json")));
        assert_eq!(options.environment, "academic");
        assert_eq!(options.icp, "/bin/icp");
    }

    #[test]
    fn usage_uses_fleet_wording() {
        let text = usage();

        assert!(text.contains("Usage: canic info env [OPTIONS] <fleet>"));
        assert!(text.contains("Installed Fleet name to inspect"));
        assert!(text.contains("sourceable current Fleet canister ID exports"));
    }

    #[test]
    fn bindings_use_role_scoped_names_and_number_duplicate_roles() {
        let bindings = env_bindings(&[
            registry_entry(COORDINATOR, Some("fleet_coordinator")),
            registry_entry(USER_SHARD_B, Some("user-shard")),
            registry_entry(USER_HUB, Some("user_hub")),
            registry_entry(ROOT, Some("root")),
            registry_entry(USER_SHARD_A, Some("user-shard")),
        ]);

        assert_eq!(
            bindings
                .iter()
                .map(|binding| (binding.variable.clone(), binding.canister_id.clone()))
                .collect::<Vec<_>>(),
            [
                (
                    "CANIC_FLEET_COORDINATOR".to_string(),
                    COORDINATOR.to_string()
                ),
                ("CANIC_ROOT".to_string(), ROOT.to_string()),
                ("CANIC_USER_HUB".to_string(), USER_HUB.to_string()),
                ("CANIC_USER_SHARD_1".to_string(), USER_SHARD_A.to_string()),
                ("CANIC_USER_SHARD_2".to_string(), USER_SHARD_B.to_string()),
            ]
        );
    }

    #[test]
    fn missing_role_uses_generic_canister_binding_without_inventing_authority() {
        let bindings = env_bindings(&[
            registry_entry(ROOT, None),
            registry_entry(USER_HUB, Some("user_hub")),
        ]);

        assert_eq!(bindings[0].variable, "CANIC_CANISTER");
        assert_eq!(bindings[0].canister_id, ROOT);
        assert_eq!(bindings[0].role, None);
    }

    #[test]
    fn render_shell_exports_is_sourceable() {
        let report = InfoEnvReport {
            fleet: "demo-local".to_string(),
            environment: "academic".to_string(),
            bindings: vec![InfoEnvBinding {
                variable: "CANIC_ROOT".to_string(),
                role: Some("root".to_string()),
                canister_id: "abc'def".to_string(),
                kind: None,
                parent_pid: None,
            }],
        };

        assert_eq!(
            render_shell_exports(&report),
            "# canic info env demo-local\n# environment: academic\nexport CANIC_ROOT='abc'\"'\"'def'"
        );
    }
}
