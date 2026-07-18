//! Module: canic_cli::replica
//!
//! Responsibility: manage local ICP replica start, stop, and status commands.
//! Does not own: ICP process supervision, project config schema, or replica HTTP probing internals.
//! Boundary: validates local project context and delegates lifecycle operations to ICP CLI helpers.

#[cfg(test)]
mod tests;

use crate::{
    cli::clap::{
        flag_arg, parse_matches, parse_subcommand, passthrough_subcommand, render_usage,
        string_option_or_else, typed_option, value_arg,
    },
    cli::defaults::default_icp,
    cli::globals::internal_icp_arg,
    cli::help::print_help_or_version,
    version_text,
};
use canic_host::{
    icp::{IcpCli, IcpCommandError, IcpDiagnostic},
    icp_config::{
        DEFAULT_LOCAL_GATEWAY_PORT, IcpConfigError, IcpProjectConfigReport,
        configured_local_gateway_port_from_root, inspect_canic_icp_yaml,
        resolve_current_canic_icp_root,
    },
    replica_query,
};
use clap::Command as ClapCommand;
use serde::Serialize;
use std::{ffi::OsString, path::Path};
use thiserror::Error as ThisError;

const REPLICA_HELP_AFTER: &str = "\
Examples:
  canic replica status
  canic replica start
  canic replica start --background
  canic replica start --debug
  canic replica stop";
const REPLICA_START_HELP_AFTER: &str = "\
Examples:
  canic replica start
  canic replica start --background
  canic replica start --port 8001 --background
  canic replica start --debug";
const REPLICA_STATUS_HELP_AFTER: &str = "\
Examples:
  canic replica status
  canic replica status --debug";
const REPLICA_STOP_HELP_AFTER: &str = "\
Examples:
  canic replica stop
  canic replica stop --debug";

///
/// ReplicaCommandError
///
/// CLI boundary error for local replica command parsing, project config checks,
/// ownership diagnostics, and delegated ICP CLI execution.
///

#[derive(Debug, ThisError)]
pub enum ReplicaCommandError {
    #[error("{0}")]
    Usage(String),

    #[error(
        "local ICP replica port is already owned by ICP network `{network}` for project: {project}\nCanic targeted ICP network `local` for this project and will not manage a different owner. Stop that exact network from its project root, or change one gateway port, then retry."
    )]
    ForeignLocalReplicaOwner { network: String, project: String },

    #[error(
        "configured local replica port is {current}, but --port requested {requested}\nEdit icp.yaml networks.local.gateway.port, then retry."
    )]
    PortMismatch { current: u16, requested: u16 },

    #[error(
        "this project's local ICP network is not running, but a local ICP replica is reachable. Canic could not identify the owner, so it will not stop an unknown foreground process.\nIf you started `canic replica start` without --background, stop it with Ctrl-C in that terminal. Otherwise stop the owning ICP project/network."
    )]
    ForeignLocalReplicaReachable,

    #[error(
        "ICP project config is missing for this directory.\nCreate or repair icp.yaml from the project root, then run `canic status` to check it."
    )]
    ProjectManifestMissing,

    #[error(
        "ICP project config is incomplete:\n{details}\nEdit icp.yaml, then run `canic status` to check it."
    )]
    ProjectConfigIncomplete { details: String },

    #[error(transparent)]
    Icp(#[from] IcpCommandError),

    #[error(transparent)]
    IcpConfig(#[from] IcpConfigError),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Parsed options shared by local replica lifecycle subcommands.

#[derive(Clone, Debug, Eq, PartialEq)]
struct ReplicaOptions {
    icp: String,
    port: Option<u16>,
    background: bool,
    debug: bool,
    json: bool,
}

impl ReplicaOptions {
    fn parse_start<I>(args: I) -> Result<Self, ReplicaCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(replica_start_command(), args)
            .map_err(|_| ReplicaCommandError::Usage(start_usage()))?;
        Ok(Self {
            icp: string_option_or_else(&matches, "icp", default_icp),
            port: typed_option(&matches, "port"),
            background: matches.get_flag("background"),
            debug: matches.get_flag("debug"),
            json: false,
        })
    }

    fn parse_status<I>(args: I) -> Result<Self, ReplicaCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(replica_status_command(), args)
            .map_err(|_| ReplicaCommandError::Usage(status_usage()))?;
        Ok(Self {
            icp: string_option_or_else(&matches, "icp", default_icp),
            port: None,
            background: false,
            debug: matches.get_flag("debug"),
            json: matches.get_flag("json"),
        })
    }

    fn parse_stop<I>(args: I) -> Result<Self, ReplicaCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(replica_stop_command(), args)
            .map_err(|_| ReplicaCommandError::Usage(stop_usage()))?;
        Ok(Self {
            icp: string_option_or_else(&matches, "icp", default_icp),
            port: None,
            background: false,
            debug: matches.get_flag("debug"),
            json: false,
        })
    }
}

/// JSON status report for local replica state and the source of that state.

#[derive(Serialize)]
struct ReplicaStatusJsonReport {
    network: &'static str,
    running: bool,
    configured_gateway_port: String,
    status_source: ReplicaStatusSource,
    icp_cli_running: bool,
    local_gateway_reachable: bool,
    icp_status: Option<serde_json::Value>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ReplicaStatusSource {
    IcpCli,
    IcpCliStale,
    HttpStatus,
    None,
}

pub fn run<I>(args: I) -> Result<(), ReplicaCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    match parse_subcommand(replica_command(), args)
        .map_err(|_| ReplicaCommandError::Usage(usage()))?
    {
        None => {
            println!("{}", usage());
            Ok(())
        }
        Some((command, args)) => match command.as_str() {
            "start" => run_start(args),
            "status" => run_status(args),
            "stop" => run_stop(args),
            _ => unreachable!("replica dispatch command only defines known commands"),
        },
    }
}

fn run_start<I>(args: I) -> Result<(), ReplicaCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, start_usage, version_text()) {
        return Ok(());
    }

    let options = ReplicaOptions::parse_start(args)?;
    let report = inspect_canic_icp_yaml(None)?;
    ensure_replica_project_config(&report)?;
    let icp_root = report.icp_root;
    ensure_requested_replica_port(&icp_root, options.port)?;
    let icp = IcpCli::new(options.icp, None);
    let icp_cli_running = icp
        .local_replica_project_running_in(&icp_root, options.debug)
        .map_err(replica_icp_error)?;
    let local_gateway_reachable = local_replica_http_reachable(&icp_root);
    if local_gateway_reachable {
        if !icp_cli_running {
            println!(
                "Replica already running: local (port {}, HTTP reachable; ICP CLI status stopped)",
                replica_port_label(&icp_root)
            );
            return Ok(());
        }
        println!(
            "Replica already running: local (port {})",
            replica_port_label(&icp_root)
        );
        return Ok(());
    }
    if icp_cli_running {
        println!(
            "Replica status is stale: ICP CLI reports local running, but port {} is not reachable. Starting local replica again.",
            replica_port_label(&icp_root)
        );
    }
    let output = icp
        .local_replica_start_in(&icp_root, options.background, options.debug)
        .map_err(replica_icp_error)?;
    print_command_output(&output);
    if options.background {
        println!("Replica started: local");
    }
    Ok(())
}

fn run_status<I>(args: I) -> Result<(), ReplicaCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, status_usage, version_text()) {
        return Ok(());
    }

    let options = ReplicaOptions::parse_status(args)?;
    let icp_root = resolve_current_canic_icp_root()?;
    let port = replica_port_label(&icp_root);
    let icp = IcpCli::new(options.icp, None);
    if options.json {
        return run_status_json(&icp, &icp_root, &port, options.debug);
    }
    match icp.local_replica_status_in(&icp_root, options.debug) {
        Ok(output) => {
            if local_replica_http_reachable(&icp_root) {
                println!("Replica: running (local, port {port})");
                print_command_output(&output);
            } else {
                println!(
                    "Replica: stopped (local, port {port}, ICP CLI status stale; HTTP not reachable)"
                );
            }
        }
        Err(error) if local_network_not_running(&error) => {
            if local_replica_http_reachable(&icp_root) {
                println!(
                    "Replica: running (local, port {port}, HTTP reachable; ICP CLI status stopped)"
                );
            } else {
                println!("Replica: stopped (local, port {port})");
            }
        }
        Err(error) => return Err(replica_icp_error(error)),
    }
    Ok(())
}

fn run_status_json(
    icp: &IcpCli,
    icp_root: &Path,
    port: &str,
    debug: bool,
) -> Result<(), ReplicaCommandError> {
    let report = match icp.local_replica_status_json_in(icp_root, debug) {
        Ok(status) => {
            let local_gateway_reachable = local_replica_http_reachable(icp_root);
            ReplicaStatusJsonReport {
                network: "local",
                running: local_gateway_reachable,
                configured_gateway_port: port.to_string(),
                status_source: if local_gateway_reachable {
                    ReplicaStatusSource::IcpCli
                } else {
                    ReplicaStatusSource::IcpCliStale
                },
                icp_cli_running: true,
                local_gateway_reachable,
                icp_status: Some(status),
            }
        }
        Err(error) if local_network_not_running(&error) => {
            let local_gateway_reachable = local_replica_http_reachable(icp_root);
            ReplicaStatusJsonReport {
                network: "local",
                running: local_gateway_reachable,
                configured_gateway_port: port.to_string(),
                status_source: if local_gateway_reachable {
                    ReplicaStatusSource::HttpStatus
                } else {
                    ReplicaStatusSource::None
                },
                icp_cli_running: false,
                local_gateway_reachable,
                icp_status: None,
            }
        }
        Err(error) => return Err(replica_icp_error(error)),
    };
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn run_stop<I>(args: I) -> Result<(), ReplicaCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, stop_usage, version_text()) {
        return Ok(());
    }

    let options = ReplicaOptions::parse_stop(args)?;
    let icp_root = resolve_current_canic_icp_root()?;
    let icp = IcpCli::new(options.icp, None);
    match icp.local_replica_stop_in(&icp_root, options.debug) {
        Ok(output) => {
            print_command_output(&output);
            println!("Replica stopped: local");
        }
        Err(error) if local_network_not_running(&error) => {
            if icp.local_replica_ping(options.debug).unwrap_or(false) {
                if let Some(owner) =
                    probe_reachable_local_replica_owner(&icp, &icp_root, options.debug)
                {
                    return Err(ReplicaCommandError::ForeignLocalReplicaOwner {
                        network: owner.network,
                        project: owner.project,
                    });
                }
                return Err(ReplicaCommandError::ForeignLocalReplicaReachable);
            }
            println!("Replica already stopped: local");
        }
        Err(error) => return Err(replica_icp_error(error)),
    }
    Ok(())
}

fn ensure_replica_project_config(
    report: &IcpProjectConfigReport,
) -> Result<(), ReplicaCommandError> {
    if report.is_ready() {
        return Ok(());
    }

    Err(ReplicaCommandError::ProjectConfigIncomplete {
        details: format_config_issues(report),
    })
}

fn ensure_requested_replica_port(
    icp_root: &Path,
    requested: Option<u16>,
) -> Result<(), ReplicaCommandError> {
    let Some(requested) = requested else {
        return Ok(());
    };

    let current =
        configured_local_gateway_port_from_root(icp_root).unwrap_or(DEFAULT_LOCAL_GATEWAY_PORT);
    if current != requested {
        return Err(ReplicaCommandError::PortMismatch { current, requested });
    }
    Ok(())
}

fn format_config_issues(report: &IcpProjectConfigReport) -> String {
    report
        .issues()
        .into_iter()
        .map(|issue| format!("  - {issue}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn print_command_output(output: &str) {
    if !output.trim().is_empty() {
        println!("{output}");
    }
}

fn replica_port_label(icp_root: &Path) -> String {
    configured_local_gateway_port_from_root(icp_root)
        .unwrap_or(DEFAULT_LOCAL_GATEWAY_PORT)
        .to_string()
}

fn parse_replica_port(value: &str) -> Result<u16, String> {
    value
        .parse::<u16>()
        .ok()
        .filter(|port| *port > 0)
        .ok_or_else(|| "expected 1..65535".to_string())
}

fn local_replica_http_reachable(icp_root: &Path) -> bool {
    replica_query::local_replica_status_reachable_from_root(Some("local"), icp_root)
}

fn replica_icp_error(error: IcpCommandError) -> ReplicaCommandError {
    match error.diagnostic() {
        Some(IcpDiagnostic::ForeignLocalReplicaOwner { network, project }) => {
            return ReplicaCommandError::ForeignLocalReplicaOwner { network, project };
        }
        Some(IcpDiagnostic::ProjectManifestMissing) => {
            return ReplicaCommandError::ProjectManifestMissing;
        }
        _ => {}
    }

    ReplicaCommandError::Icp(error)
}

fn local_network_not_running(error: &IcpCommandError) -> bool {
    matches!(
        error.diagnostic(),
        Some(IcpDiagnostic::LocalNetworkNotRunning)
    )
}

fn probe_reachable_local_replica_owner(
    icp: &IcpCli,
    icp_root: &Path,
    debug: bool,
) -> Option<LocalReplicaOwner> {
    match icp.local_replica_start_in(icp_root, true, debug) {
        Err(error) => match error.diagnostic() {
            Some(IcpDiagnostic::ForeignLocalReplicaOwner { network, project }) => {
                Some(LocalReplicaOwner { network, project })
            }
            _ => None,
        },
        Ok(_) => None,
    }
}

/// Project/network owner parsed from ICP CLI local-port conflict diagnostics.

#[derive(Clone, Debug, Eq, PartialEq)]
struct LocalReplicaOwner {
    network: String,
    project: String,
}

fn replica_command() -> ClapCommand {
    ClapCommand::new("replica")
        .bin_name("canic replica")
        .about("Manage the local ICP replica")
        .disable_help_flag(true)
        .subcommand(passthrough_subcommand(
            ClapCommand::new("start")
                .about("Start the local ICP replica")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("status")
                .about("Show local ICP replica status")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("stop")
                .about("Stop the local ICP replica")
                .disable_help_flag(true),
        ))
        .after_help(REPLICA_HELP_AFTER)
}

fn replica_start_command() -> ClapCommand {
    replica_leaf_command(
        "start",
        "canic replica start",
        "Start the local ICP replica",
    )
    .arg(
        flag_arg("background")
            .long("background")
            .help("Run the replica in the background"),
    )
    .arg(
        value_arg("port")
            .long("port")
            .value_name("PORT")
            .value_parser(clap::builder::ValueParser::new(parse_replica_port))
            .help("Require icp.yaml to use this local gateway port"),
    )
    .after_help(REPLICA_START_HELP_AFTER)
}

fn replica_status_command() -> ClapCommand {
    replica_leaf_command(
        "status",
        "canic replica status",
        "Show local ICP replica status",
    )
    .arg(
        flag_arg("json")
            .long("json")
            .help("Emit JSON status output"),
    )
    .after_help(REPLICA_STATUS_HELP_AFTER)
}

fn replica_stop_command() -> ClapCommand {
    replica_leaf_command("stop", "canic replica stop", "Stop the local ICP replica")
        .after_help(REPLICA_STOP_HELP_AFTER)
}

fn replica_leaf_command(
    name: &'static str,
    bin_name: &'static str,
    about: &'static str,
) -> ClapCommand {
    ClapCommand::new(name)
        .bin_name(bin_name)
        .about(about)
        .disable_help_flag(true)
        .arg(internal_icp_arg())
        .arg(
            flag_arg("debug")
                .long("debug")
                .help("Enable ICP CLI debug logging"),
        )
}

fn usage() -> String {
    render_usage(replica_command)
}

fn start_usage() -> String {
    render_usage(replica_start_command)
}

fn status_usage() -> String {
    render_usage(replica_status_command)
}

fn stop_usage() -> String {
    render_usage(replica_stop_command)
}
