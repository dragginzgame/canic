use crate::cli::{clap::value_arg, help::COMMAND_SPECS};
use clap::{Arg, ArgAction, Command};
use std::ffi::OsString;

pub const DISPATCH_ARGS: &str = "args";
pub const INTERNAL_ICP_OPTION: &str = "--__canic-icp";
pub const INTERNAL_NETWORK_OPTION: &str = "--__canic-network";

pub fn icp_arg() -> Arg {
    value_arg("icp")
        .long("icp")
        .value_name("path")
        .help("Path to the icp executable for ICP-backed commands")
}

pub fn internal_icp_arg() -> Arg {
    value_arg("icp").long("__canic-icp").hide(true)
}

pub fn network_arg() -> Arg {
    value_arg("network")
        .long("network")
        .value_name("name")
        .help("ICP CLI network for networked commands")
}

pub fn internal_network_arg() -> Arg {
    value_arg("network").long("__canic-network").hide(true)
}

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
        "endpoints" | "medic" | "metrics" | "status" => true,
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
        "build" | "endpoints" | "install" | "medic" | "metrics" | "status" => true,
        "info" => info_leaf_accepts_globals(tail),
        "fleet" => tail.first().and_then(|arg| arg.to_str()) == Some("list"),
        "snapshot" => tail.first().and_then(|arg| arg.to_str()) == Some("download"),
        "backup" => tail.first().and_then(|arg| arg.to_str()) == Some("create"),
        "restore" => tail.first().and_then(|arg| arg.to_str()) == Some("run"),
        _ => false,
    }
}

fn info_leaf_accepts_globals(tail: &[OsString]) -> bool {
    matches!(
        tail.first().and_then(|arg| arg.to_str()),
        Some("cycles" | "list")
    )
}

fn tail_has_option(tail: &[OsString], name: &str) -> bool {
    tail.iter().any(|arg| arg.to_str() == Some(name))
}
