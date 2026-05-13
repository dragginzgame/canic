use crate::{
    cli::clap::{
        flag_arg, parse_matches, parse_subcommand, passthrough_subcommand, string_option, value_arg,
    },
    cli::defaults::default_icp,
    cli::globals::internal_icp_arg,
    cli::help::print_help_or_version,
    version_text,
};
use canic_host::{
    icp::{IcpCli, IcpCommandError},
    icp_config::{
        DEFAULT_LOCAL_GATEWAY_PORT, IcpConfigError, configured_local_gateway_port,
        set_configured_local_gateway_port,
    },
};
use clap::Command as ClapCommand;
use std::ffi::OsString;
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

#[derive(Debug, ThisError)]
pub enum ReplicaCommandError {
    #[error("{0}")]
    Usage(String),

    #[error(
        "local ICP replica port is already owned by ICP network `{network}` for project: {project}\nCanic targeted ICP network `local` for this project and will not manage a different owner. Stop that exact network from its project root, or change one gateway port, then retry."
    )]
    ForeignLocalReplicaOwner { network: String, project: String },

    #[error("invalid replica port `{value}`; expected 1..65535")]
    InvalidPort { value: String },

    #[error(
        "cannot change local replica port while this project's local ICP network is running (current {current}, requested {requested}); stop it first, then retry"
    )]
    PortChangeRequiresStopped { current: u16, requested: u16 },

    #[error(
        "this project's local ICP network is not running, but a local ICP replica is reachable. Canic could not identify the owner, so it will not stop it.\nRun `icp network start -e local --background` from this project to ask ICP for the owning project/environment, then stop that exact network."
    )]
    ForeignLocalReplicaReachable,

    #[error("icp command failed: {command}\n{stderr}")]
    IcpFailed { command: String, stderr: String },

    #[error(transparent)]
    IcpConfig(#[from] IcpConfigError),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

///
/// ReplicaOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct ReplicaOptions {
    icp: String,
    port: Option<u16>,
    background: bool,
    debug: bool,
}

impl ReplicaOptions {
    fn parse_start<I>(args: I) -> Result<Self, ReplicaCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(replica_start_command(), args)
            .map_err(|_| ReplicaCommandError::Usage(start_usage()))?;
        Ok(Self {
            icp: string_option(&matches, "icp").unwrap_or_else(default_icp),
            port: parse_port_option(&matches)?,
            background: matches.get_flag("background"),
            debug: matches.get_flag("debug"),
        })
    }

    fn parse_status<I>(args: I) -> Result<Self, ReplicaCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(replica_status_command(), args)
            .map_err(|_| ReplicaCommandError::Usage(status_usage()))?;
        Ok(Self {
            icp: string_option(&matches, "icp").unwrap_or_else(default_icp),
            port: None,
            background: false,
            debug: matches.get_flag("debug"),
        })
    }

    fn parse_stop<I>(args: I) -> Result<Self, ReplicaCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(replica_stop_command(), args)
            .map_err(|_| ReplicaCommandError::Usage(stop_usage()))?;
        Ok(Self {
            icp: string_option(&matches, "icp").unwrap_or_else(default_icp),
            port: None,
            background: false,
            debug: matches.get_flag("debug"),
        })
    }
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
    let icp = IcpCli::new(options.icp, None, None);
    if icp
        .local_replica_project_running(options.debug)
        .map_err(replica_icp_error)?
    {
        if let Some(requested) = options.port {
            let current = configured_local_gateway_port().unwrap_or(DEFAULT_LOCAL_GATEWAY_PORT);
            if current != requested {
                return Err(ReplicaCommandError::PortChangeRequiresStopped { current, requested });
            }
        }
        println!(
            "Replica already running: local (port {})",
            replica_port_label()
        );
        return Ok(());
    }
    if let Some(port) = options.port {
        let path = set_configured_local_gateway_port(port)?;
        println!("Replica port configured: {port} ({})", path.display());
    }

    let output = icp
        .local_replica_start(options.background, options.debug)
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
    let port = replica_port_label();
    let icp = IcpCli::new(options.icp, None, None);
    match icp.local_replica_status(options.debug) {
        Ok(output) => {
            println!("Replica: running (local, port {port})");
            print_command_output(&output);
        }
        Err(error) if local_network_not_running(&error) => {
            println!("Replica: stopped (local, port {port})");
        }
        Err(error) => return Err(replica_icp_error(error)),
    }
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
    let icp = IcpCli::new(options.icp, None, None);
    match icp.local_replica_stop(options.debug) {
        Ok(output) => {
            print_command_output(&output);
            println!("Replica stopped: local");
        }
        Err(error) if local_network_not_running(&error) => {
            if icp.local_replica_ping(options.debug).unwrap_or(false) {
                if let Some(owner) = probe_reachable_local_replica_owner(&icp, options.debug) {
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

fn print_command_output(output: &str) {
    if !output.trim().is_empty() {
        println!("{output}");
    }
}

fn parse_port_option(matches: &clap::ArgMatches) -> Result<Option<u16>, ReplicaCommandError> {
    let Some(value) = string_option(matches, "port") else {
        return Ok(None);
    };
    let Ok(port) = value.parse::<u16>() else {
        return Err(ReplicaCommandError::InvalidPort { value });
    };
    if port == 0 {
        return Err(ReplicaCommandError::InvalidPort { value });
    }
    Ok(Some(port))
}

fn replica_port_label() -> String {
    configured_local_gateway_port()
        .unwrap_or(DEFAULT_LOCAL_GATEWAY_PORT)
        .to_string()
}

fn replica_icp_error(error: IcpCommandError) -> ReplicaCommandError {
    match error {
        IcpCommandError::Io(err) => ReplicaCommandError::Io(err),
        IcpCommandError::Failed { command, stderr } => {
            if let Some(owner) = extract_foreign_local_owner(&stderr) {
                return ReplicaCommandError::ForeignLocalReplicaOwner {
                    network: owner.network,
                    project: owner.project,
                };
            }
            ReplicaCommandError::IcpFailed { command, stderr }
        }
        IcpCommandError::SnapshotIdUnavailable { output } => ReplicaCommandError::IcpFailed {
            command: "icp canister snapshot".to_string(),
            stderr: output,
        },
    }
}

fn local_network_not_running(error: &IcpCommandError) -> bool {
    matches!(
        error,
        IcpCommandError::Failed { stderr, .. }
            if stderr.contains("network 'local' is not running")
                || stderr.contains("the local network for this project is not running")
    )
}

fn probe_reachable_local_replica_owner(icp: &IcpCli, debug: bool) -> Option<LocalReplicaOwner> {
    match icp.local_replica_start(true, debug) {
        Err(IcpCommandError::Failed { stderr, .. }) => extract_foreign_local_owner(&stderr),
        Err(_) | Ok(_) => None,
    }
}

///
/// LocalReplicaOwner
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct LocalReplicaOwner {
    network: String,
    project: String,
}

fn extract_foreign_local_owner(stderr: &str) -> Option<LocalReplicaOwner> {
    let marker = " network of the project at '";
    let marker_start = stderr.find(marker)?;
    let network = stderr[..marker_start]
        .split_whitespace()
        .last()?
        .to_string();
    let start = marker_start + marker.len();
    let rest = &stderr[start..];
    let end = rest.find('\'')?;
    Some(LocalReplicaOwner {
        network,
        project: rest[..end].to_string(),
    })
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
            .help("Set the local gateway port in icp.yaml before starting"),
    )
    .after_help(REPLICA_START_HELP_AFTER)
}

fn replica_status_command() -> ClapCommand {
    replica_leaf_command(
        "status",
        "canic replica status",
        "Show local ICP replica status",
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
    let mut command = replica_command();
    command.render_help().to_string()
}

fn start_usage() -> String {
    let mut command = replica_start_command();
    command.render_help().to_string()
}

fn status_usage() -> String {
    let mut command = replica_status_command();
    command.render_help().to_string()
}

fn stop_usage() -> String {
    let mut command = replica_stop_command();
    command.render_help().to_string()
}

#[cfg(test)]
mod tests;
