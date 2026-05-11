use crate::{
    cli::clap::{parse_matches, string_option},
    cli::defaults::{default_icp, local_network},
    cli::globals::{internal_icp_arg, internal_network_arg},
    cli::help::print_help_or_version,
    version_text,
};
use canic_host::{
    icp::IcpCli,
    install_root::{discover_current_canic_config_choices, read_named_fleet_install_state},
    registry::{RegistryEntry, parse_registry_entries},
    release_set::{
        configured_bootstrap_roles, configured_fleet_name, configured_fleet_roles,
        display_workspace_path, workspace_root,
    },
    replica_query,
    table::{ColumnAlign, render_table},
};
use clap::Command as ClapCommand;
use std::{collections::BTreeSet, ffi::OsString, path::Path};
use thiserror::Error as ThisError;

const FLEET_HEADER: &str = "FLEET";
const DEPLOYED_HEADER: &str = "DEPLOYED";
const CONFIG_HEADER: &str = "CONFIG";
const CANISTERS_HEADER: &str = "CANISTERS";
const ROOT_HEADER: &str = "ROOT";
const STATUS_HELP_AFTER: &str = "\
Examples:
  canic status";

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
    icp_cli: String,
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
    let icp_cli = load_icp_cli_version(options);
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
        icp_cli,
        fleets,
    })
}

fn load_icp_cli_version(options: &StatusOptions) -> String {
    match IcpCli::new(&options.icp, None, None).version() {
        Ok(version) => version,
        Err(err) => format!("unavailable ({err})"),
    }
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
    let configured_roles = configured_fleet_roles(path);
    let bootstrap_roles = configured_bootstrap_roles(path);
    let (deployed, root) = match install_state {
        Ok(Some(state)) => (
            deployed_label(
                network,
                &state.root_canister_id,
                verify_local_root,
                bootstrap_roles.as_deref().unwrap_or(&[]),
            ),
            state.root_canister_id,
        ),
        Ok(None) => ("no".to_string(), "-".to_string()),
        Err(_) => ("error".to_string(), "-".to_string()),
    };

    StatusFleetRow {
        canisters: configured_roles
            .map_or_else(|_| "invalid".to_string(), |roles| roles.len().to_string()),
        config: display_workspace_path(workspace_root, path),
        deployed,
        fleet,
        root,
    }
}

fn deployed_label(
    network: &str,
    root: &str,
    verify_local_root: bool,
    configured_roles: &[String],
) -> String {
    if !replica_query::should_use_local_replica_query(Some(network)) {
        return "yes".to_string();
    }
    if !verify_local_root {
        return "unknown".to_string();
    }

    match replica_query::query_subnet_registry_json(Some(network), root) {
        Ok(registry_json) => match parse_registry_entries(&registry_json) {
            Ok(registry) => classify_local_deployment(configured_roles, &registry).to_string(),
            Err(_) => "error".to_string(),
        },
        Err(err) if is_canister_not_found_error(&err.to_string()) => "stale".to_string(),
        Err(_) => "error".to_string(),
    }
}

fn classify_local_deployment(
    configured_roles: &[String],
    registry: &[RegistryEntry],
) -> &'static str {
    let deployed_roles = registry
        .iter()
        .filter_map(|entry| entry.role.as_deref())
        .collect::<BTreeSet<_>>();

    if configured_roles
        .iter()
        .all(|role| deployed_roles.contains(role.as_str()))
    {
        "yes"
    } else {
        "partial"
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
        format!("ICP CLI: {}", report.icp_cli),
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
    let rows = fleets
        .iter()
        .map(|fleet| {
            [
                fleet.fleet.clone(),
                fleet.deployed.clone(),
                fleet.config.clone(),
                fleet.canisters.clone(),
                fleet.root.clone(),
            ]
        })
        .collect::<Vec<_>>();
    render_table(
        &[
            FLEET_HEADER,
            DEPLOYED_HEADER,
            CONFIG_HEADER,
            CANISTERS_HEADER,
            ROOT_HEADER,
        ],
        &rows,
        &[ColumnAlign::Left; 5],
    )
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

fn status_command() -> ClapCommand {
    ClapCommand::new("status")
        .bin_name("canic status")
        .about("Show quick Canic project status")
        .disable_help_flag(true)
        .arg(internal_network_arg())
        .arg(internal_icp_arg())
        .after_help(STATUS_HELP_AFTER)
}

fn usage() -> String {
    let mut command = status_command();
    command.render_help().to_string()
}

#[cfg(test)]
mod tests;
