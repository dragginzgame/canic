use crate::{
    args::{
        default_icp, flag_arg, internal_icp_arg, parse_matches, parse_subcommand,
        passthrough_subcommand, print_help_or_version, string_option,
    },
    version_text,
};
use canic_host::icp::{IcpCli, IcpCommandError};
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

    #[error("icp command failed: {command}\n{stderr}")]
    IcpFailed { command: String, stderr: String },

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

///
/// ReplicaOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct ReplicaOptions {
    icp: String,
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
    if options.background
        && icp
            .local_replica_ping(options.debug)
            .map_err(replica_icp_error)?
    {
        println!("Replica already running: local");
        return Ok(());
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
    let output = IcpCli::new(options.icp, None, None)
        .local_replica_status(options.debug)
        .map_err(replica_icp_error)?;
    print_command_output(&output);
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
    let output = IcpCli::new(options.icp, None, None)
        .local_replica_stop(options.debug)
        .map_err(replica_icp_error)?;
    print_command_output(&output);
    println!("Replica stopped: local");
    Ok(())
}

fn print_command_output(output: &str) {
    if !output.trim().is_empty() {
        println!("{output}");
    }
}

fn replica_icp_error(error: IcpCommandError) -> ReplicaCommandError {
    match error {
        IcpCommandError::Io(err) => ReplicaCommandError::Io(err),
        IcpCommandError::Failed { command, stderr } => {
            ReplicaCommandError::IcpFailed { command, stderr }
        }
        IcpCommandError::SnapshotIdUnavailable { output } => ReplicaCommandError::IcpFailed {
            command: "icp canister snapshot".to_string(),
            stderr: output,
        },
    }
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
mod tests {
    use super::*;

    // Ensure replica start defaults to foreground mode while allowing background use.
    #[test]
    fn parses_replica_start_options() {
        let options = ReplicaOptions::parse_start([
            OsString::from("--background"),
            OsString::from(crate::args::INTERNAL_ICP_OPTION),
            OsString::from("/tmp/icp"),
        ])
        .expect("parse replica start");

        assert_eq!(options.icp, "/tmp/icp");
        assert!(options.background);
        assert!(!options.debug);
    }

    // Ensure foreground mode is the default, matching ICP CLI.
    #[test]
    fn replica_start_defaults_to_foreground() {
        let options = ReplicaOptions::parse_start([]).expect("parse replica start");

        assert_eq!(options.icp, "icp");
        assert!(!options.background);
        assert!(!options.debug);
    }

    // Ensure replica lifecycle commands can enable ICP CLI debug logging.
    #[test]
    fn parses_replica_debug_options() {
        let start =
            ReplicaOptions::parse_start([OsString::from("--debug")]).expect("parse replica start");
        let status = ReplicaOptions::parse_status([OsString::from("--debug")])
            .expect("parse replica status");
        let stop =
            ReplicaOptions::parse_stop([OsString::from("--debug")]).expect("parse replica stop");

        assert!(start.debug);
        assert!(status.debug);
        assert!(stop.debug);
    }

    // Ensure status uses the default ICP executable when no override is provided.
    #[test]
    fn parses_replica_status_options() {
        let options = ReplicaOptions::parse_status([]).expect("parse replica status");

        assert_eq!(options.icp, "icp");
        assert!(!options.background);
        assert!(!options.debug);
    }

    // Ensure replica help exposes the native lifecycle commands.
    #[test]
    fn replica_usage_lists_commands() {
        let text = usage();

        assert!(text.contains("Manage the local ICP replica"));
        assert!(text.contains("start"));
        assert!(text.contains("status"));
        assert!(text.contains("stop"));
        assert!(text.contains("canic replica status"));
    }

    // Ensure leaf help documents command-specific options and examples.
    #[test]
    fn replica_leaf_usage_lists_options() {
        let text = start_usage();

        assert!(text.contains("--background"));
        assert!(text.contains("--debug"));
        assert!(!text.contains("--icp"));
        assert!(text.contains("canic replica start --background"));
        assert!(text.contains("canic replica start --debug"));
    }
}
