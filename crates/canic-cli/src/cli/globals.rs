//! Module: canic_cli::cli::globals
//!
//! Responsibility: define top-level global options and forward them to eligible commands.
//! Does not own: command-specific option semantics, command execution, or help rendering.
//! Boundary: translates public global flags into hidden per-command arguments.

use crate::cli::{clap::value_arg, help::COMMAND_SPECS};
use clap::{Arg, ArgAction, Command};
use std::ffi::OsString;

/// Captured passthrough tail argument id for top-level command dispatch.
pub const DISPATCH_ARGS: &str = "args";

/// Hidden per-command ICP executable option injected from top-level `--icp`.
pub const INTERNAL_ICP_OPTION: &str = "--__canic-icp";

/// Hidden per-command network option injected from top-level `--network`.
pub const INTERNAL_NETWORK_OPTION: &str = "--__canic-network";

/// Build the public top-level ICP executable option.
pub fn icp_arg() -> Arg {
    value_arg("icp")
        .long("icp")
        .value_name("path")
        .help("Path to the icp executable for ICP-backed commands")
}

/// Build the hidden per-command ICP executable option.
pub fn internal_icp_arg() -> Arg {
    value_arg("icp").long("__canic-icp").hide(true)
}

/// Build the public top-level ICP network option.
pub fn network_arg() -> Arg {
    value_arg("network")
        .long("network")
        .value_name("name")
        .help("ICP CLI network for networked commands")
}

/// Build the hidden per-command ICP network option.
pub fn internal_network_arg() -> Arg {
    value_arg("network").long("__canic-network").hide(true)
}

/// Build the top-level dispatch parser used before command-specific parsing.
pub fn top_level_dispatch_command() -> Command {
    let command = Command::new("canic")
        .disable_help_flag(true)
        .disable_version_flag(true)
        .arg(
            Arg::new("version")
                .short('V')
                .long("version")
                .action(ArgAction::SetTrue),
        );
    let command = command
        .arg(icp_arg().global(true))
        .arg(network_arg().global(true));

    COMMAND_SPECS.iter().fold(command, |command, spec| {
        command.subcommand(dispatch_subcommand(spec.name))
    })
}

fn dispatch_subcommand(name: &'static str) -> Command {
    Command::new(name).arg(
        Arg::new(DISPATCH_ARGS)
            .num_args(0..)
            .allow_hyphen_values(true)
            .trailing_var_arg(true)
            .value_parser(clap::value_parser!(OsString)),
    )
}

/// Return a misplaced public global option after a subcommand, if present.
pub fn command_local_global_option(args: &[OsString]) -> Option<&'static str> {
    let mut index = 0;
    while index < args.len() {
        let arg = args[index].to_str()?;
        if COMMAND_SPECS.iter().any(|spec| spec.name == arg) {
            return args[index + 1..]
                .iter()
                .filter_map(|arg| arg.to_str())
                .find_map(global_option_name);
        }
        index += if matches!(arg, "--icp" | "--network") {
            2
        } else {
            1
        };
    }
    None
}

fn global_option_name(arg: &str) -> Option<&'static str> {
    match arg {
        "--icp" => Some("--icp"),
        "--network" => Some("--network"),
        _ if arg.starts_with("--icp=") => Some("--icp"),
        _ if arg.starts_with("--network=") => Some("--network"),
        _ => None,
    }
}

/// Inject top-level `--icp` into commands that accept ICP CLI selection.
pub fn apply_global_icp(command: &str, tail: &mut Vec<OsString>, global_icp: Option<String>) {
    let Some(global_icp) = global_icp else {
        return;
    };
    if tail_has_option(tail, INTERNAL_ICP_OPTION) {
        return;
    }
    if !command_accepts_global_icp(command, tail) {
        return;
    }

    tail.push(OsString::from(INTERNAL_ICP_OPTION));
    tail.push(OsString::from(global_icp));
}

/// Inject top-level `--network` into commands that accept network selection.
pub fn apply_global_network(
    command: &str,
    tail: &mut Vec<OsString>,
    global_network: Option<String>,
) {
    let Some(global_network) = global_network else {
        return;
    };
    if tail_has_option(tail, INTERNAL_NETWORK_OPTION) {
        return;
    }
    if !command_accepts_global_network(command, tail) {
        return;
    }

    tail.push(OsString::from(INTERNAL_NETWORK_OPTION));
    tail.push(OsString::from(global_network));
}

fn command_accepts_global_icp(command: &str, tail: &[OsString]) -> bool {
    match command {
        "blob-storage" | "cycles" | "status" | "token" => true,
        "auth" => auth_leaf_accepts_globals(tail),
        "info" => info_leaf_accepts_globals(tail),
        "replica" => matches!(
            tail.first().and_then(|arg| arg.to_str()),
            Some("start" | "status" | "stop")
        ),
        "snapshot" => tail.first().and_then(|arg| arg.to_str()) == Some("download"),
        "backup" => tail.first().and_then(|arg| arg.to_str()) == Some("create"),
        "restore" => tail.first().and_then(|arg| arg.to_str()) == Some("run"),
        _ => false,
    }
}

fn command_accepts_global_network(command: &str, tail: &[OsString]) -> bool {
    match command {
        "blob-storage" | "build" | "cycles" | "install" | "status" | "token" => true,
        "auth" => auth_leaf_accepts_globals(tail),
        "deploy" => deploy_leaf_accepts_global_network(tail),
        "info" => info_leaf_accepts_globals(tail),
        "fleet" => tail.first().and_then(|arg| arg.to_str()) == Some("list"),
        "snapshot" => tail.first().and_then(|arg| arg.to_str()) == Some("download"),
        "backup" => tail.first().and_then(|arg| arg.to_str()) == Some("create"),
        "restore" => tail.first().and_then(|arg| arg.to_str()) == Some("run"),
        _ => false,
    }
}

fn auth_leaf_accepts_globals(tail: &[OsString]) -> bool {
    matches!(
        (
            tail.first().and_then(|arg| arg.to_str()),
            tail.get(1).and_then(|arg| arg.to_str())
        ),
        (Some("renewal"), Some("run-once"))
    )
}

fn info_leaf_accepts_globals(tail: &[OsString]) -> bool {
    matches!(
        tail.first().and_then(|arg| arg.to_str()),
        Some("cycles" | "endpoints" | "env" | "list" | "medic" | "metrics")
    )
}

fn deploy_leaf_accepts_global_network(tail: &[OsString]) -> bool {
    let first = tail.first().and_then(|arg| arg.to_str());
    let second = tail.get(1).and_then(|arg| arg.to_str());
    let third = tail.get(2).and_then(|arg| arg.to_str());

    match first {
        Some("check" | "install" | "register") => true,
        Some("authority") => matches!(second, Some("check" | "evidence" | "receipt" | "report")),
        Some("external") => matches!(
            second,
            Some("check" | "critical-fix" | "handoff" | "pending" | "plan" | "proposals")
        ),
        Some("inspect") => match second {
            Some("catalog") => matches!(third, Some("inspect" | "list")),
            Some("diff" | "inventory" | "plan" | "report" | "resume-report") => true,
            _ => false,
        },
        Some("root") => second == Some("verify"),
        _ => false,
    }
}

fn tail_has_option(tail: &[OsString], name: &str) -> bool {
    tail.iter().any(|arg| arg.to_str() == Some(name))
}

// -----------------------------------------------------------------------------
// Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_global_option_detects_command_tail_flags() {
        assert_eq!(
            command_local_global_option(&[
                OsString::from("status"),
                OsString::from("--network"),
                OsString::from("ic")
            ]),
            Some("--network")
        );
        assert_eq!(
            command_local_global_option(&[OsString::from("status"), OsString::from("--icp=icp")]),
            Some("--icp")
        );
    }

    #[test]
    fn global_forwarding_does_not_duplicate_hidden_options() {
        let mut tail = vec![
            OsString::from(INTERNAL_NETWORK_OPTION),
            OsString::from("ic"),
        ];

        apply_global_network("status", &mut tail, Some("local".to_string()));

        assert_eq!(
            tail,
            [
                OsString::from(INTERNAL_NETWORK_OPTION),
                OsString::from("ic")
            ]
        );
    }
}
