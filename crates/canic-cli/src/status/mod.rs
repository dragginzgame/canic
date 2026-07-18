//! Module: canic_cli::status
//!
//! Responsibility: render the quick local Canic project status summary.
//! Does not own: installed deployment state, replica lifecycle, or fleet config parsing.
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
    icp::IcpCli,
    icp_config::{
        DEFAULT_LOCAL_GATEWAY_PORT, configured_local_gateway_port_from_root,
        inspect_canic_icp_yaml_from_root, resolve_current_canic_icp_root,
    },
    install_root::{ConfigDiscoveryError, discover_project_canic_config_choices},
    installed_deployment::{
        InstalledDeploymentError, InstalledDeploymentRequest,
        read_installed_deployment_state_from_root, resolve_installed_deployment_from_root,
    },
    registry::RegistryEntry,
    release_set::{
        configured_bootstrap_roles, configured_deployable_roles, configured_fleet_name,
        display_workspace_path,
    },
    replica_query,
    table::{ColumnAlign, render_table},
};
use clap::Command as ClapCommand;
use std::{collections::BTreeSet, ffi::OsString, path::Path};
use thiserror::Error as ThisError;

const DEPLOYMENT_HEADER: &str = "DEPLOYMENT";
const DEPLOYED_HEADER: &str = "DEPLOYED";
const CONFIG_HEADER: &str = "CONFIG";
const CANISTERS_HEADER: &str = "CANISTERS";
const ROOT_HEADER: &str = "ROOT";
const LOCAL_LOST_DEPLOYMENT: &str = "lost";
const LOCAL_LOST_NOTE: &str = "Note: local ICP CLI replica state is not persistent; a lost local deployment target means the recorded root is gone. Run `canic install <fleet-template>` to recreate it.";
const STATUS_HELP_AFTER: &str = "\
Examples:
  canic status

Note:
  The local ICP CLI replica does not persist canister state across stop/start.
  If a local deployment target is shown as lost, run `canic install <fleet-template>` to recreate it.";

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

/// Render-ready snapshot of local project and deployment status.

#[derive(Clone, Debug, Eq, PartialEq)]
struct StatusReport {
    environment: String,
    replica: ReplicaStatus,
    replica_port: String,
    icp_cli: String,
    icp_project: String,
    deployments: Vec<StatusDeploymentRow>,
}

/// Local replica state as observed through ICP CLI and HTTP fallback probing.

#[derive(Clone, Debug, Eq, PartialEq)]
enum ReplicaStatus {
    Running,
    RunningHttpFallback,
    Stopped,
    Error(String),
}

/// One fleet config row in the status deployment table.

#[derive(Clone, Debug, Eq, PartialEq)]
struct StatusDeploymentRow {
    deployment: String,
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
    let mut deployments = choices
        .iter()
        .map(|path| status_deployment_row(&icp_root, &icp_root, path, options, verify_local_roots))
        .collect::<Vec<_>>();
    deployments.sort_by(|left, right| left.deployment.cmp(&right.deployment));

    Ok(StatusReport {
        environment: options.environment.clone(),
        replica,
        replica_port: load_replica_port(&icp_root),
        icp_cli,
        icp_project,
        deployments,
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
        return "not checked (no Canic fleet configs)".to_string();
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

fn status_deployment_row(
    workspace_root: &Path,
    icp_root: &Path,
    path: &Path,
    options: &StatusOptions,
    verify_local_root: bool,
) -> StatusDeploymentRow {
    let Ok(deployment) = configured_fleet_name(path) else {
        return StatusDeploymentRow {
            deployment: "invalid config".to_string(),
            deployed: "error".to_string(),
            config: display_workspace_path(workspace_root, path),
            canisters: "invalid".to_string(),
            root: "-".to_string(),
        };
    };
    let install_state =
        read_installed_deployment_state_from_root(&options.environment, &deployment, icp_root);
    let configured_roles = configured_deployable_roles(path);
    let bootstrap_roles = configured_bootstrap_roles(path);
    let (deployed, root) = match install_state {
        Ok(state) => (
            deployed_label(
                &deployment,
                &options.environment,
                &options.icp,
                icp_root,
                &state.root_canister_id,
                verify_local_root,
                bootstrap_roles.as_deref().unwrap_or(&[]),
            ),
            state.root_canister_id,
        ),
        Err(InstalledDeploymentError::NoInstalledDeployment { .. }) => {
            ("no".to_string(), "-".to_string())
        }
        Err(_) => ("error".to_string(), "-".to_string()),
    };

    StatusDeploymentRow {
        canisters: configured_roles
            .map_or_else(|_| "invalid".to_string(), |roles| roles.len().to_string()),
        config: display_workspace_path(workspace_root, path),
        deployed,
        deployment,
        root,
    }
}

fn deployed_label(
    deployment: &str,
    environment: &str,
    icp: &str,
    icp_root: &Path,
    root: &str,
    verify_local_root: bool,
    configured_roles: &[String],
) -> String {
    if environment != local_environment() {
        return "yes".to_string();
    }
    if !verify_local_root {
        return "unknown".to_string();
    }

    match resolve_installed_deployment_from_root(
        &InstalledDeploymentRequest {
            deployment: deployment.to_string(),
            environment: environment.to_string(),
            icp: icp.to_string(),
            detect_lost_local_root: true,
        },
        icp_root,
    ) {
        Ok(resolution) if resolution.state.root_canister_id == root => {
            classify_local_deployment(configured_roles, &resolution.registry.entries).to_string()
        }
        Err(InstalledDeploymentError::LostLocalDeployment { .. }) => {
            LOCAL_LOST_DEPLOYMENT.to_string()
        }
        Ok(_) | Err(_) => "error".to_string(),
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

fn render_status_report(report: &StatusReport) -> String {
    let configured = report.deployments.len();
    let deployed = deployed_count(&report.deployments);
    let mut lines = vec![
        format!(
            "Replica: {}",
            render_replica_status(&report.replica, &report.replica_port)
        ),
        format!("ICP CLI: {}", report.icp_cli),
        format!("ICP project: {}", report.icp_project),
        format!(
            "Deployments: {deployed}/{configured} deployed (environment {})",
            report.environment
        ),
    ];

    if !report.deployments.is_empty() {
        lines.push(String::new());
        lines.push(render_deployment_table(&report.deployments));
    }
    if has_lost_local_deployment_target(report) {
        lines.push(String::new());
        lines.push(LOCAL_LOST_NOTE.to_string());
    }

    lines.join("\n")
}

fn has_lost_local_deployment_target(report: &StatusReport) -> bool {
    report.environment == "local"
        && report
            .deployments
            .iter()
            .any(|deployment| deployment.deployed == LOCAL_LOST_DEPLOYMENT)
}

fn deployed_count(deployments: &[StatusDeploymentRow]) -> usize {
    deployments
        .iter()
        .filter(|deployment| deployment.deployed == "yes")
        .count()
}

fn render_deployment_table(deployments: &[StatusDeploymentRow]) -> String {
    let rows = deployments
        .iter()
        .map(|deployment| {
            [
                deployment.deployment.clone(),
                deployment.deployed.clone(),
                deployment.config.clone(),
                deployment.canisters.clone(),
                deployment.root.clone(),
            ]
        })
        .collect::<Vec<_>>();
    render_table(
        &[
            DEPLOYMENT_HEADER,
            DEPLOYED_HEADER,
            CONFIG_HEADER,
            CANISTERS_HEADER,
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
