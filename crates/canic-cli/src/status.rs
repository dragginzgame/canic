use crate::{
    args::{
        default_icp, local_network, parse_matches, print_help_or_version, string_option, value_arg,
    },
    version_text,
};
use canic_host::{
    icp::IcpCli,
    install_root::{discover_current_canic_config_choices, read_named_fleet_install_state},
    release_set::{configured_fleet_name, configured_fleet_roles, workspace_root},
    replica_query,
    table::WhitespaceTable,
};
use clap::Command as ClapCommand;
use std::{ffi::OsString, path::Path};
use thiserror::Error as ThisError;

const FLEET_HEADER: &str = "FLEET";
const DEPLOYED_HEADER: &str = "DEPLOYED";
const CONFIG_HEADER: &str = "CONFIG";
const CANISTERS_HEADER: &str = "CANISTERS";
const ROOT_HEADER: &str = "ROOT";
const STATUS_HELP_AFTER: &str = "\
Examples:
  canic status
  canic status --network local
  canic status --icp /path/to/icp";

///
/// StatusCommandError
///

#[derive(Debug, ThisError)]
pub enum StatusCommandError {
    #[error("{0}")]
    Usage(String),

    #[error(transparent)]
    Host(#[from] Box<dyn std::error::Error>),
}

///
/// StatusOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct StatusOptions {
    network: String,
    icp: String,
}

///
/// StatusReport
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct StatusReport {
    network: String,
    replica: ReplicaStatus,
    fleets: Vec<StatusFleetRow>,
}

///
/// ReplicaStatus
///

#[derive(Clone, Debug, Eq, PartialEq)]
enum ReplicaStatus {
    Running,
    Stopped,
    Error(String),
}

///
/// StatusFleetRow
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct StatusFleetRow {
    fleet: String,
    deployed: String,
    config: String,
    canisters: String,
    root: String,
}

pub fn run<I>(args: I) -> Result<(), StatusCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    let options = StatusOptions::parse(args)?;
    let report = load_status_report(&options)?;
    println!("{}", render_status_report(&report));
    Ok(())
}

impl StatusOptions {
    fn parse<I>(args: I) -> Result<Self, StatusCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(status_command(), args)
            .map_err(|_| StatusCommandError::Usage(usage()))?;

        Ok(Self {
            network: string_option(&matches, "network").unwrap_or_else(local_network),
            icp: string_option(&matches, "icp").unwrap_or_else(default_icp),
        })
    }
}

fn load_status_report(options: &StatusOptions) -> Result<StatusReport, StatusCommandError> {
    let workspace_root = workspace_root()?;
    let choices = discover_current_canic_config_choices()?;
    let replica = load_replica_status(options);
    let verify_local_roots = replica_query::should_use_local_replica_query(Some(&options.network))
        && matches!(replica, ReplicaStatus::Running);
    let mut fleets = choices
        .iter()
        .map(|path| status_fleet_row(&workspace_root, path, &options.network, verify_local_roots))
        .collect::<Vec<_>>();
    fleets.sort_by(|left, right| left.fleet.cmp(&right.fleet));

    Ok(StatusReport {
        network: options.network.clone(),
        replica,
        fleets,
    })
}

fn load_replica_status(options: &StatusOptions) -> ReplicaStatus {
    match IcpCli::new(&options.icp, None, None).local_replica_ping(false) {
        Ok(true) => ReplicaStatus::Running,
        Ok(false) => ReplicaStatus::Stopped,
        Err(err) => ReplicaStatus::Error(err.to_string()),
    }
}

fn status_fleet_row(
    workspace_root: &Path,
    path: &Path,
    network: &str,
    verify_local_root: bool,
) -> StatusFleetRow {
    let Ok(fleet) = configured_fleet_name(path) else {
        return StatusFleetRow {
            fleet: "invalid config".to_string(),
            deployed: "error".to_string(),
            config: display_workspace_path(workspace_root, path),
            canisters: "invalid".to_string(),
            root: "-".to_string(),
        };
    };
    let install_state = read_named_fleet_install_state(network, &fleet);
    let (deployed, root) = match install_state {
        Ok(Some(state)) => (
            deployed_label(network, &state.root_canister_id, verify_local_root),
            state.root_canister_id,
        ),
        Ok(None) => ("no".to_string(), "-".to_string()),
        Err(_) => ("error".to_string(), "-".to_string()),
    };

    StatusFleetRow {
        canisters: configured_fleet_roles(path)
            .map_or_else(|_| "invalid".to_string(), |roles| roles.len().to_string()),
        config: display_workspace_path(workspace_root, path),
        deployed,
        fleet,
        root,
    }
}

fn deployed_label(network: &str, root: &str, verify_local_root: bool) -> String {
    if !replica_query::should_use_local_replica_query(Some(network)) {
        return "yes".to_string();
    }
    if !verify_local_root {
        return "unknown".to_string();
    }

    match replica_query::query_ready(Some(network), root) {
        Ok(_) => "yes".to_string(),
        Err(err) if is_canister_not_found_error(&err.to_string()) => "stale".to_string(),
        Err(_) => "error".to_string(),
    }
}

fn is_canister_not_found_error(error: &str) -> bool {
    error.contains("Canister ") && error.contains(" not found")
}

fn render_status_report(report: &StatusReport) -> String {
    let configured = report.fleets.len();
    let deployed = deployed_count(&report.fleets);
    let mut lines = vec![
        format!("Replica: {}", report.replica.label()),
        format!(
            "Fleets:  {deployed}/{configured} deployed (network {})",
            report.network
        ),
    ];

    if !report.fleets.is_empty() {
        lines.push(String::new());
        lines.push(render_fleet_table(&report.fleets));
    }

    lines.join("\n")
}

fn deployed_count(fleets: &[StatusFleetRow]) -> usize {
    fleets
        .iter()
        .filter(|fleet| fleet.deployed == "yes")
        .count()
}

fn render_fleet_table(fleets: &[StatusFleetRow]) -> String {
    let mut table = WhitespaceTable::new([
        FLEET_HEADER,
        DEPLOYED_HEADER,
        CONFIG_HEADER,
        CANISTERS_HEADER,
        ROOT_HEADER,
    ]);
    for fleet in fleets {
        table.push_row([
            fleet.fleet.clone(),
            fleet.deployed.clone(),
            fleet.config.clone(),
            fleet.canisters.clone(),
            fleet.root.clone(),
        ]);
    }
    table.render()
}

impl ReplicaStatus {
    fn label(&self) -> String {
        match self {
            Self::Running => "running (local)".to_string(),
            Self::Stopped => "stopped (local)".to_string(),
            Self::Error(err) => format!("unknown (local): {err}"),
        }
    }
}

fn display_workspace_path(workspace_root: &Path, path: &Path) -> String {
    path.strip_prefix(workspace_root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn status_command() -> ClapCommand {
    ClapCommand::new("status")
        .bin_name("canic status")
        .about("Show quick Canic project status")
        .disable_help_flag(true)
        .arg(
            value_arg("network")
                .long("network")
                .value_name("name")
                .help("Network to use when checking deployed fleet state"),
        )
        .arg(
            value_arg("icp")
                .long("icp")
                .value_name("path")
                .help("Path to the icp executable"),
        )
        .after_help(STATUS_HELP_AFTER)
}

fn usage() -> String {
    let mut command = status_command();
    command.render_help().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Ensure status defaults to the local network and ordinary `icp` binary.
    #[test]
    fn parses_status_options() {
        let default_options = StatusOptions::parse([]).expect("parse default options");
        assert_eq!(default_options.network, "local");
        assert_eq!(default_options.icp, "icp");

        let options = StatusOptions::parse([
            OsString::from("--network"),
            OsString::from("ic"),
            OsString::from("--icp"),
            OsString::from("/tmp/icp"),
        ])
        .expect("parse explicit options");
        assert_eq!(options.network, "ic");
        assert_eq!(options.icp, "/tmp/icp");
    }

    // Ensure the compact summary includes replica, deployment count, and fleet rows.
    #[test]
    fn renders_status_report() {
        let report = StatusReport {
            network: "local".to_string(),
            replica: ReplicaStatus::Running,
            fleets: vec![
                StatusFleetRow {
                    fleet: "demo".to_string(),
                    deployed: "no".to_string(),
                    config: "fleets/demo/canic.toml".to_string(),
                    canisters: "2".to_string(),
                    root: "-".to_string(),
                },
                StatusFleetRow {
                    fleet: "test".to_string(),
                    deployed: "yes".to_string(),
                    config: "fleets/test/canic.toml".to_string(),
                    canisters: "7".to_string(),
                    root: "aaaaa-aa".to_string(),
                },
            ],
        };

        assert_eq!(
            render_status_report(&report),
            format!(
                "Replica: running (local)\n\
                 Fleets:  1/2 deployed (network local)\n\
                 \n\
                 {:<5}  {:<8}  {:<22}  {:<9}  {}\n\
                 {:<5}  {:<8}  {:<22}  {:<9}  {}\n\
                 {:<5}  {:<8}  {:<22}  {:<9}  {}",
                "FLEET",
                "DEPLOYED",
                "CONFIG",
                "CANISTERS",
                "ROOT",
                "demo",
                "no",
                "fleets/demo/canic.toml",
                "2",
                "-",
                "test",
                "yes",
                "fleets/test/canic.toml",
                "7",
                "aaaaa-aa",
            )
        );
    }

    // Ensure empty fleet workspaces still render a useful quick status.
    #[test]
    fn renders_empty_status_report() {
        let report = StatusReport {
            network: "local".to_string(),
            replica: ReplicaStatus::Stopped,
            fleets: Vec::new(),
        };

        assert_eq!(
            render_status_report(&report),
            "Replica: stopped (local)\nFleets:  0/0 deployed (network local)"
        );
    }

    // Ensure local installed-state rows are not reported as deployed when live roots are unchecked.
    #[test]
    fn local_deployed_label_is_unknown_without_replica_verification() {
        assert_eq!(deployed_label("local", "aaaaa-aa", false), "unknown");
        assert_eq!(deployed_label("ic", "aaaaa-aa", false), "yes");
    }

    // Ensure stale local-root diagnostics match the list command's canister-not-found shape.
    #[test]
    fn detects_canister_not_found_error() {
        assert!(is_canister_not_found_error(
            "local replica rejected query: code=3 message=Canister aaaaa-aa not found"
        ));
        assert!(!is_canister_not_found_error("connection refused"));
    }

    // Ensure status help points to the compact project summary command.
    #[test]
    fn status_usage_lists_options_and_examples() {
        let text = usage();

        assert!(text.contains("Show quick Canic project status"));
        assert!(text.contains("Usage: canic status"));
        assert!(text.contains("--network <name>"));
        assert!(text.contains("--icp <path>"));
        assert!(text.contains("Examples:"));
    }
}
