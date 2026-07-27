//! Module: canic_cli::status
//!
//! Responsibility: render the quick local Canic project status summary.
//! Does not own: installed Fleet state, replica lifecycle, or App config parsing.
//! Boundary: reads host/project state and formats the operator-facing status view.

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
    fleet_catalog::{FleetCatalogEntryV1, FleetCatalogRequest, build_fleet_catalog_report},
    icp::IcpCli,
    icp_config::{
        DEFAULT_LOCAL_GATEWAY_PORT, configured_local_gateway_port_from_root,
        inspect_canic_icp_yaml_from_root, resolve_current_canic_icp_root,
    },
    install_root::{ConfigDiscoveryError, discover_project_canic_config_choices},
    installed_fleet::{
        InstalledFleetError, InstalledFleetRequest, resolve_installed_fleet_from_root,
    },
    registry::RegistryEntry,
    release_set::{AppConfigSnapshot, display_workspace_path},
    replica_query,
    table::{ColumnAlign, render_table},
};
use clap::Command as ClapCommand;
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    path::Path,
};
use thiserror::Error as ThisError;

const APP_HEADER: &str = "APP";
const FLEET_HEADER: &str = "FLEET";
const NETWORK_HEADER: &str = "NETWORK";
const DEPLOYED_HEADER: &str = "DEPLOYED";
const CONFIG_HEADER: &str = "CONFIG";
const CANISTERS_HEADER: &str = "CANISTERS";
const ROOT_HEADER: &str = "ROOT";
const LOCAL_LOST_FLEET: &str = "lost";
const LOCAL_LOST_NOTE: &str = "Note: local ICP CLI replica state is not persistent; a lost local Fleet means the recorded root is gone. Run `canic install <app> <fleet> --fleet-input <path>` to recreate it.";
const STATUS_HELP_AFTER: &str = "\
Examples:
  canic status

Note:
  The local ICP CLI replica does not persist canister state across stop/start.
  If a local Fleet is shown as lost, run `canic install <app> <fleet> --fleet-input <path>` to recreate it.";

///
/// StatusCommandError
///
/// CLI boundary error for status option parsing and host/project status reads.
///

#[derive(Debug, ThisError)]
pub enum StatusCommandError {
    #[error("{0}")]
    Usage(String),

    #[error("failed to discover Canic project configs: {0}")]
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
    icp_project: String,
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

/// App rows plus the explicit App-ID lookup used for Fleet comparison.

struct StatusAppInventory {
    rows: Vec<StatusAppRow>,
    bootstrap_roles_by_app: BTreeMap<String, Vec<String>>,
}

/// One independently discovered canonical Fleet catalog row.

#[derive(Clone, Debug, Eq, PartialEq)]
struct StatusFleetRow {
    fleet: String,
    app: String,
    network: String,
    deployed: String,
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
            environment: string_option_or_else(&matches, "environment", local_environment),
            icp: string_option_or_else(&matches, "icp", default_icp),
        })
    }
}

fn load_status_report(options: &StatusOptions) -> Result<StatusReport, StatusCommandError> {
    let icp_root =
        resolve_current_canic_icp_root().map_err(|err| StatusCommandError::Host(Box::new(err)))?;
    let choices = discover_project_canic_config_choices(&icp_root)?;
    let icp_cli = load_icp_cli_version(options);
    let icp_project = load_icp_project_config_status(&icp_root, &choices);
    let replica = load_replica_status(options, &icp_root);
    let verify_local_roots = options.environment == local_environment()
        && matches!(
            replica,
            ReplicaStatus::Running | ReplicaStatus::RunningHttpFallback
        );
    let apps = load_status_apps(&icp_root, &choices);
    let catalog = build_fleet_catalog_report(&FleetCatalogRequest {
        project_root: icp_root.clone(),
        environment: options.environment.clone(),
        generated_at: String::new(),
    })
    .map_err(|error| StatusCommandError::Host(Box::new(error)))?;
    let mut fleets = catalog
        .entries
        .iter()
        .map(|entry| {
            status_fleet_row(
                &icp_root,
                entry,
                options,
                verify_local_roots,
                apps.bootstrap_roles_by_app
                    .get(entry.app.as_str())
                    .map(Vec::as_slice),
            )
        })
        .collect::<Vec<_>>();
    fleets.sort_by(|left, right| left.fleet.cmp(&right.fleet));

    Ok(StatusReport {
        environment: options.environment.clone(),
        replica,
        replica_port: load_replica_port(&icp_root),
        icp_cli,
        icp_project,
        canonical_network_id: catalog.canonical_network_id.to_string(),
        apps: apps.rows,
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
        Ok(false)
            if replica_query::should_use_local_replica_query(Some(&options.environment))
                && replica_query::local_replica_status_reachable_from_root(
                    Some(&options.environment),
                    icp_root,
                ) =>
        {
            ReplicaStatus::RunningHttpFallback
        }
        Ok(false) => ReplicaStatus::Stopped,
        Err(err) => ReplicaStatus::Error(err.to_string()),
    }
}

fn load_replica_port(icp_root: &Path) -> String {
    configured_local_gateway_port_from_root(icp_root)
        .unwrap_or(DEFAULT_LOCAL_GATEWAY_PORT)
        .to_string()
}

fn load_icp_project_config_status(icp_root: &Path, choices: &[std::path::PathBuf]) -> String {
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

fn load_status_apps(workspace_root: &Path, paths: &[std::path::PathBuf]) -> StatusAppInventory {
    let mut rows = Vec::with_capacity(paths.len());
    let mut bootstrap_roles_by_app = BTreeMap::new();
    for path in paths {
        match AppConfigSnapshot::load(path) {
            Ok(config) => {
                let app = config.app_id().to_string();
                rows.push(StatusAppRow {
                    app: app.clone(),
                    config: display_workspace_path(workspace_root, path),
                    canisters: config.deployable_roles().len().to_string(),
                });
                bootstrap_roles_by_app.insert(app, config.bootstrap_roles());
            }
            Err(_) => rows.push(StatusAppRow {
                app: "invalid config".to_string(),
                config: display_workspace_path(workspace_root, path),
                canisters: "invalid".to_string(),
            }),
        }
    }
    rows.sort_by(|left, right| left.app.cmp(&right.app));
    StatusAppInventory {
        rows,
        bootstrap_roles_by_app,
    }
}

fn status_fleet_row(
    icp_root: &Path,
    fleet: &FleetCatalogEntryV1,
    options: &StatusOptions,
    verify_local_root: bool,
    configured_roles: Option<&[String]>,
) -> StatusFleetRow {
    let deployed = deployed_label(
        fleet,
        options,
        icp_root,
        verify_local_root,
        configured_roles,
    );
    StatusFleetRow {
        fleet: fleet.fleet_name.to_string(),
        app: fleet.app.to_string(),
        network: fleet.canonical_network_id.to_string(),
        deployed,
        root: fleet.root_principal.clone(),
    }
}

fn deployed_label(
    fleet: &FleetCatalogEntryV1,
    options: &StatusOptions,
    icp_root: &Path,
    verify_local_root: bool,
    configured_roles: Option<&[String]>,
) -> String {
    if options.environment != local_environment() {
        return "yes".to_string();
    }
    if !verify_local_root {
        return "unknown".to_string();
    }

    match resolve_installed_fleet_from_root(
        &InstalledFleetRequest {
            fleet: fleet.fleet_name.to_string(),
            environment: options.environment.clone(),
            icp: options.icp.clone(),
            detect_lost_local_root: true,
        },
        icp_root,
    ) {
        Ok(resolution) if resolution.fleet.root_principal == fleet.root_principal => {
            configured_roles.map_or_else(
                || "yes".to_string(),
                |roles| classify_local_fleet(roles, &resolution.registry.entries).to_string(),
            )
        }
        Err(InstalledFleetError::LostLocalFleet { .. }) => LOCAL_LOST_FLEET.to_string(),
        Ok(_) | Err(_) => "error".to_string(),
    }
}

fn classify_local_fleet(configured_roles: &[String], registry: &[RegistryEntry]) -> &'static str {
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

fn render_status_report(report: &StatusReport) -> String {
    let deployed = deployed_count(&report.fleets);
    let mut lines = vec![
        format!(
            "Replica: {}",
            render_replica_status(&report.replica, &report.replica_port)
        ),
        format!("ICP CLI: {}", report.icp_cli),
        format!("ICP project: {}", report.icp_project),
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
    if has_lost_local_fleet(report) {
        lines.push(String::new());
        lines.push(LOCAL_LOST_NOTE.to_string());
    }

    lines.join("\n")
}

fn has_lost_local_fleet(report: &StatusReport) -> bool {
    report.environment == "local"
        && report
            .fleets
            .iter()
            .any(|fleet| fleet.deployed == LOCAL_LOST_FLEET)
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
                fleet.root.clone(),
            ]
        })
        .collect::<Vec<_>>();
    render_table(
        &[
            FLEET_HEADER,
            APP_HEADER,
            NETWORK_HEADER,
            DEPLOYED_HEADER,
            ROOT_HEADER,
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
        .about("Show quick Canic project status")
        .disable_help_flag(true)
        .arg(internal_environment_arg())
        .arg(internal_icp_arg())
        .after_help(STATUS_HELP_AFTER)
}

fn usage() -> String {
    render_usage(status_command)
}
