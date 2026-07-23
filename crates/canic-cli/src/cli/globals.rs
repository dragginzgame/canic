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

/// Hidden per-command environment option injected from top-level `--environment`.
pub const INTERNAL_ENVIRONMENT_OPTION: &str = "--__canic-environment";

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

/// Build the public top-level ICP environment option.
pub fn environment_arg() -> Arg {
    value_arg("environment")
        .long("environment")
        .value_name("name")
        .help("ICP environment for ICP-backed commands")
}

/// Build the hidden per-command ICP environment option.
pub fn internal_environment_arg() -> Arg {
    value_arg("environment")
        .long("__canic-environment")
        .hide(true)
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
        .arg(environment_arg().global(true));

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
        index += if matches!(arg, "--icp" | "--environment") {
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
        "--environment" => Some("--environment"),
        _ if arg.starts_with("--icp=") => Some("--icp"),
        _ if arg.starts_with("--environment=") => Some("--environment"),
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

/// Inject top-level `--environment` into commands that accept environment selection.
pub fn apply_global_environment(
    command: &str,
    tail: &mut Vec<OsString>,
    global_environment: Option<String>,
) {
    let Some(global_environment) = global_environment else {
        return;
    };
    if tail_has_option(tail, INTERNAL_ENVIRONMENT_OPTION) {
        return;
    }
    if !command_accepts_global_environment(command, tail) {
        return;
    }

    tail.push(OsString::from(INTERNAL_ENVIRONMENT_OPTION));
    tail.push(OsString::from(global_environment));
}

fn command_accepts_global_icp(command: &str, tail: &[OsString]) -> bool {
    match command {
        "blob-storage" | "cycles" | "inspect" | "medic" | "status" | "token" => true,
        "auth" => auth_leaf_accepts_globals(tail),
        "info" => info_leaf_accepts_globals(tail),
        "replica" => matches!(
            tail.first().and_then(|arg| arg.to_str()),
            Some("start" | "status" | "stop")
        ),
        "backup" => tail.first().and_then(|arg| arg.to_str()) == Some("create"),
        "restore" => tail.first().and_then(|arg| arg.to_str()) == Some("run"),
        _ => false,
    }
}

fn command_accepts_global_environment(command: &str, tail: &[OsString]) -> bool {
    match command {
        "blob-storage" | "build" | "cycles" | "inspect" | "install" | "medic" | "status"
        | "token" => true,
        "auth" => auth_leaf_accepts_globals(tail),
        "deploy" => deploy_leaf_accepts_global_environment(tail),
        "info" => info_leaf_accepts_globals(tail),
        "app" => tail.first().and_then(|arg| arg.to_str()) == Some("list"),
        "backup" => tail.first().and_then(|arg| arg.to_str()) == Some("create"),
        "restore" => tail.first().and_then(|arg| arg.to_str()) == Some("run"),
        _ => false,
    }
}

fn auth_leaf_accepts_globals(tail: &[OsString]) -> bool {
    if !matches!(tail.first().and_then(|arg| arg.to_str()), Some("renewal")) {
        return false;
    }

    tail.get(1).and_then(|arg| arg.to_str()) == Some("status")
}

fn info_leaf_accepts_globals(tail: &[OsString]) -> bool {
    matches!(
        tail.first().and_then(|arg| arg.to_str()),
        Some("cycles" | "endpoints" | "env" | "list" | "metrics")
    )
}

fn deploy_leaf_accepts_global_environment(tail: &[OsString]) -> bool {
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
                OsString::from("--environment"),
                OsString::from("ic")
            ]),
            Some("--environment")
        );
        assert_eq!(
            command_local_global_option(&[OsString::from("status"), OsString::from("--icp=icp")]),
            Some("--icp")
        );
    }

    #[test]
    fn global_forwarding_does_not_duplicate_hidden_options() {
        let mut tail = vec![
            OsString::from(INTERNAL_ENVIRONMENT_OPTION),
            OsString::from("ic"),
        ];

        apply_global_environment("status", &mut tail, Some("local".to_string()));

        assert_eq!(
            tail,
            [
                OsString::from(INTERNAL_ENVIRONMENT_OPTION),
                OsString::from("ic")
            ]
        );
    }

    #[test]
    fn inspect_accepts_global_icp_and_environment() {
        let mut tail = vec![OsString::from("canister"), OsString::from("aaaaa-aa")];

        apply_global_icp("inspect", &mut tail, Some("icp".to_string()));
        apply_global_environment("inspect", &mut tail, Some("local".to_string()));

        assert_eq!(
            tail,
            [
                OsString::from("canister"),
                OsString::from("aaaaa-aa"),
                OsString::from(INTERNAL_ICP_OPTION),
                OsString::from("icp"),
                OsString::from(INTERNAL_ENVIRONMENT_OPTION),
                OsString::from("local"),
            ]
        );
    }
}
