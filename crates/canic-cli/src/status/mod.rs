//! Module: canic_cli::status
//!
//! Responsibility: render the quick local Canic workspace status summary.
//! Does not own: current Fleet state, replica lifecycle, or App config parsing.
//! Boundary: reads host/workspace state and formats the operator-facing status view.

#[cfg(test)]
mod tests;

use crate::{
    cli::clap::{parse_matches, render_usage, string_option_or_else},
    cli::defaults::{default_icp, local_environment},
    cli::globals::{internal_environment_arg, internal_icp_arg},
    cli::help::print_help_or_version,
    version_text,
};
use canic_host::{
    config_discovery::{ConfigDiscoveryError, discover_workspace_canic_config_choices},
    fleet_ensure::{CurrentFleetSummary, discover_current_fleets},
    icp::IcpCli,
    icp_config::{
        DEFAULT_LOCAL_GATEWAY_PORT, configured_local_gateway_port_from_root,
        inspect_canic_icp_yaml_from_root, resolve_current_canic_icp_root,
    },
    release_set::{AppConfigSnapshot, display_workspace_path},
    replica_query,
    table::{ColumnAlign, render_table},
};
use clap::Command as ClapCommand;
use std::{ffi::OsString, path::Path};
use thiserror::Error as ThisError;

const APP_HEADER: &str = "APP";
const FLEET_HEADER: &str = "FLEET";
const NETWORK_HEADER: &str = "NETWORK";
const DEPLOYED_HEADER: &str = "DEPLOYED";
const CONFIG_HEADER: &str = "CONFIG";
const CANISTERS_HEADER: &str = "CANISTERS";
const COORDINATOR_HEADER: &str = "COORDINATOR";
const STATUS_HELP_AFTER: &str = "\
Examples:
  canic status

Note:
  Fleet rows come from validated terminal Fleet Ensure state.
  This summary does not query live Coordinator or Fleet Subnet Root state.";

///
/// StatusCommandError
///
/// CLI boundary error for status option parsing and host/workspace status reads.
///

#[derive(Debug, ThisError)]
pub enum StatusCommandError {
    #[error("{0}")]
    Usage(String),

    #[error("failed to discover Canic workspace App configs: {0}")]
    ConfigDiscovery(#[from] ConfigDiscoveryError),

    #[error(transparent)]
    Host(#[from] Box<dyn std::error::Error>),
}

/// Parsed `canic status` command options.

#[derive(Clone, Debug, Eq, PartialEq)]
struct StatusOptions {
    environment: String,
    icp: String,
}

/// Render-ready snapshot of independent local App and Fleet status.

#[derive(Clone, Debug, Eq, PartialEq)]
struct StatusReport {
    environment: String,
    replica: ReplicaStatus,
    replica_port: String,
    icp_cli: String,
    icp_config: String,
    canonical_network_id: String,
    apps: Vec<StatusAppRow>,
    fleets: Vec<StatusFleetRow>,
}

/// Local replica state as observed through ICP CLI and HTTP fallback probing.

#[derive(Clone, Debug, Eq, PartialEq)]
enum ReplicaStatus {
    Running,
    RunningHttpFallback,
    Stopped,
    Error(String),
}

/// One independently discovered App config.

#[derive(Clone, Debug, Eq, PartialEq)]
struct StatusAppRow {
    app: String,
    config: String,
    canisters: String,
}

/// One independently discovered terminal Fleet row.

#[derive(Clone, Debug, Eq, PartialEq)]
struct StatusFleetRow {
    fleet: String,
    app: String,
    network: String,
    deployed: String,
    coordinator: String,
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
            environment: string_option_or_else(&matches, "environment", local_environment),
            icp: string_option_or_else(&matches, "icp", default_icp),
        })
    }
}

fn load_status_report(options: &StatusOptions) -> Result<StatusReport, StatusCommandError> {
    let icp_root =
        resolve_current_canic_icp_root().map_err(|err| StatusCommandError::Host(Box::new(err)))?;
    let choices = discover_workspace_canic_config_choices(&icp_root)?;
    let icp_cli = load_icp_cli_version(options);
    let icp_config = load_icp_config_status(&icp_root, &choices);
    let replica = load_replica_status(options, &icp_root);
    let apps = load_status_apps(&icp_root, &choices);
    let discovery = discover_current_fleets(&icp_root, &options.environment)
        .map_err(|error| StatusCommandError::Host(Box::new(error)))?;
    let mut fleets = discovery
        .fleets
        .iter()
        .map(status_fleet_row)
        .collect::<Vec<_>>();
    fleets.sort_by(|left, right| left.fleet.cmp(&right.fleet));

    Ok(StatusReport {
        environment: options.environment.clone(),
        replica,
        replica_port: load_replica_port(&icp_root),
        icp_cli,
        icp_config,
        canonical_network_id: discovery.canonical_network_id.to_string(),
        apps,
        fleets,
    })
}

fn load_icp_cli_version(options: &StatusOptions) -> String {
    match IcpCli::new(&options.icp, None).compatible_version() {
        Ok(version) => version,
        Err(err) => format!("unavailable ({err})"),
    }
}

fn load_replica_status(options: &StatusOptions, icp_root: &Path) -> ReplicaStatus {
    match IcpCli::new(&options.icp, None).local_replica_project_running_in(icp_root, false) {
        Ok(true) => ReplicaStatus::Running,
        Ok(false) => match replica_query::uses_local_replica_transport(
            Some(&options.environment),
            Some(icp_root),
        ) {
            Ok(true)
                if replica_query::local_replica_status_reachable_from_root(
                    Some(&options.environment),
                    icp_root,
                ) =>
            {
                ReplicaStatus::RunningHttpFallback
            }
            Ok(_) => ReplicaStatus::Stopped,
            Err(err) => ReplicaStatus::Error(err.to_string()),
        },
        Err(err) => ReplicaStatus::Error(err.to_string()),
    }
}

fn load_replica_port(icp_root: &Path) -> String {
    configured_local_gateway_port_from_root(icp_root)
        .unwrap_or(DEFAULT_LOCAL_GATEWAY_PORT)
        .to_string()
}

fn load_icp_config_status(icp_root: &Path, choices: &[std::path::PathBuf]) -> String {
    if choices.is_empty() {
        return "not checked (no Canic App configs)".to_string();
    }

    match inspect_canic_icp_yaml_from_root(icp_root, None) {
        Ok(report) if report.is_ready() => {
            format!("ok ({})", display_workspace_path(icp_root, &report.path))
        }
        Ok(report) => {
            format!("incomplete ({})", report.issues().join("; "))
        }
        Err(err) => format!("error ({err})"),
    }
}

fn load_status_apps(workspace_root: &Path, paths: &[std::path::PathBuf]) -> Vec<StatusAppRow> {
    let mut rows = Vec::with_capacity(paths.len());
    for path in paths {
        match AppConfigSnapshot::load(path) {
            Ok(config) => {
                let app = config.app_id().to_string();
                rows.push(StatusAppRow {
                    app: app.clone(),
                    config: display_workspace_path(workspace_root, path),
                    canisters: config.deployable_roles().len().to_string(),
                });
            }
            Err(_) => rows.push(StatusAppRow {
                app: "invalid config".to_string(),
                config: display_workspace_path(workspace_root, path),
                canisters: "invalid".to_string(),
            }),
        }
    }
    rows.sort_by(|left, right| left.app.cmp(&right.app));
    rows
}

fn status_fleet_row(fleet: &CurrentFleetSummary) -> StatusFleetRow {
    StatusFleetRow {
        fleet: fleet.fleet.to_string(),
        app: fleet.app.to_string(),
        network: fleet.canonical_network_id.to_string(),
        deployed: "yes".to_string(),
        coordinator: fleet.coordinator.clone(),
    }
}

fn render_status_report(report: &StatusReport) -> String {
    let deployed = deployed_count(&report.fleets);
    let mut lines = vec![
        format!(
            "Replica: {}",
            render_replica_status(&report.replica, &report.replica_port)
        ),
        format!("ICP CLI: {}", report.icp_cli),
        format!("ICP config: {}", report.icp_config),
        format!("Apps: {} configured", report.apps.len()),
        format!(
            "Fleets: {deployed}/{} deployed (environment {}, network {})",
            report.fleets.len(),
            report.environment,
            report.canonical_network_id,
        ),
    ];

    if !report.apps.is_empty() {
        lines.push(String::new());
        lines.push(render_app_table(&report.apps));
    }
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

fn render_app_table(apps: &[StatusAppRow]) -> String {
    let rows = apps
        .iter()
        .map(|app| [app.app.clone(), app.config.clone(), app.canisters.clone()])
        .collect::<Vec<_>>();
    render_table(
        &[APP_HEADER, CONFIG_HEADER, CANISTERS_HEADER],
        &rows,
        &[ColumnAlign::Left; 3],
    )
}

fn render_fleet_table(fleets: &[StatusFleetRow]) -> String {
    let rows = fleets
        .iter()
        .map(|fleet| {
            [
                fleet.fleet.clone(),
                fleet.app.clone(),
                fleet.network.clone(),
                fleet.deployed.clone(),
                fleet.coordinator.clone(),
            ]
        })
        .collect::<Vec<_>>();
    render_table(
        &[
            FLEET_HEADER,
            APP_HEADER,
            NETWORK_HEADER,
            DEPLOYED_HEADER,
            COORDINATOR_HEADER,
        ],
        &rows,
        &[ColumnAlign::Left; 5],
    )
}

fn render_replica_status(status: &ReplicaStatus, port: &str) -> String {
    match status {
        ReplicaStatus::Running => format!("running (local, port {port})"),
        ReplicaStatus::RunningHttpFallback => {
            format!("running (local, port {port}, HTTP reachable; ICP CLI status stopped)")
        }
        ReplicaStatus::Stopped => format!("stopped (local, port {port})"),
        ReplicaStatus::Error(err) => format!("unknown (local, port {port}): {err}"),
    }
}

fn status_command() -> ClapCommand {
    ClapCommand::new("status")
        .bin_name("canic status")
        .about("Show quick local workspace status")
        .disable_help_flag(true)
        .arg(internal_environment_arg())
        .arg(internal_icp_arg())
        .after_help(STATUS_HELP_AFTER)
}

fn usage() -> String {
    render_usage(status_command)
}
