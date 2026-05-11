use crate::cli::globals::{icp_arg, network_arg};
use clap::{Arg, ArgAction, Command};
use std::ffi::OsString;

const TOP_LEVEL_HELP_TEMPLATE: &str = "{name} {version}\n{about-with-newline}\n{usage-heading} {usage}\n\n{before-help}Options:\n{options}{after-help}\n";
const COLOR_RESET: &str = "\x1b[0m";
const COLOR_HEADING: &str = "\x1b[1m";
const COLOR_GROUP: &str = "\x1b[38;5;245m";
const COLOR_COMMAND: &str = "\x1b[38;5;109m";
const COLOR_TIP: &str = "\x1b[38;5;245m";

///
/// CommandScope
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommandScope {
    Global,
    FleetContext,
    BackupRestore,
}

impl CommandScope {
    const fn heading(self) -> &'static str {
        match self {
            Self::Global => "Global commands",
            Self::FleetContext => "Fleet commands",
            Self::BackupRestore => "Backup and restore commands",
        }
    }
}

///
/// CommandSpec
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandSpec {
    pub name: &'static str,
    pub about: &'static str,
    scope: CommandScope,
}

pub const COMMAND_SPECS: &[CommandSpec] = &[
    CommandSpec {
        name: "status",
        about: "Show quick Canic project status",
        scope: CommandScope::Global,
    },
    CommandSpec {
        name: "fleet",
        about: "Manage Canic fleets",
        scope: CommandScope::Global,
    },
    CommandSpec {
        name: "replica",
        about: "Manage the local ICP replica",
        scope: CommandScope::Global,
    },
    CommandSpec {
        name: "install",
        about: "Install and bootstrap a Canic fleet",
        scope: CommandScope::FleetContext,
    },
    CommandSpec {
        name: "config",
        about: "Inspect selected fleet config",
        scope: CommandScope::FleetContext,
    },
    CommandSpec {
        name: "list",
        about: "List deployed fleet canisters",
        scope: CommandScope::FleetContext,
    },
    CommandSpec {
        name: "endpoints",
        about: "List canister Candid endpoints",
        scope: CommandScope::FleetContext,
    },
    CommandSpec {
        name: "medic",
        about: "Diagnose local Canic fleet setup",
        scope: CommandScope::FleetContext,
    },
    CommandSpec {
        name: "cycles",
        about: "Summarize fleet cycle history",
        scope: CommandScope::FleetContext,
    },
    CommandSpec {
        name: "metrics",
        about: "Query Canic runtime telemetry",
        scope: CommandScope::FleetContext,
    },
    CommandSpec {
        name: "snapshot",
        about: "Capture and download canister snapshots",
        scope: CommandScope::BackupRestore,
    },
    CommandSpec {
        name: "backup",
        about: "Verify backup directories and journal status",
        scope: CommandScope::BackupRestore,
    },
    CommandSpec {
        name: "manifest",
        about: "Validate fleet backup manifests",
        scope: CommandScope::BackupRestore,
    },
    CommandSpec {
        name: "restore",
        about: "Plan or run snapshot restores",
        scope: CommandScope::BackupRestore,
    },
];

pub fn is_help_arg(arg: &OsString) -> bool {
    arg.to_str()
        .is_some_and(|arg| matches!(arg, "help" | "--help" | "-h"))
}

pub fn is_version_arg(arg: &OsString) -> bool {
    arg.to_str()
        .is_some_and(|arg| matches!(arg, "version" | "--version" | "-V"))
}

pub fn first_arg_is_help(args: &[OsString]) -> bool {
    args.first().is_some_and(is_help_arg)
}

pub fn first_arg_is_version(args: &[OsString]) -> bool {
    args.first().is_some_and(is_version_arg)
}

pub fn print_help_or_version(
    args: &[OsString],
    usage: impl FnOnce() -> String,
    version_text: &str,
) -> bool {
    if first_arg_is_help(args) {
        println!("{}", usage());
        return true;
    }
    if first_arg_is_version(args) {
        println!("{version_text}");
        return true;
    }
    false
}

#[must_use]
pub fn top_level_command() -> Command {
    let command = Command::new("canic")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Operator CLI for Canic install, backup, and restore workflows")
        .disable_version_flag(true)
        .arg(
            Arg::new("version")
                .short('V')
                .long("version")
                .action(ArgAction::SetTrue)
                .help("Print version"),
        )
        .arg(icp_arg().global(true))
        .arg(network_arg().global(true))
        .subcommand_help_heading("Commands")
        .help_template(TOP_LEVEL_HELP_TEMPLATE)
        .before_help(grouped_command_section(COMMAND_SPECS).join("\n"))
        .after_help("Run `canic <command> help` for command-specific help.");

    COMMAND_SPECS.iter().fold(command, |command, spec| {
        command.subcommand(Command::new(spec.name).about(spec.about))
    })
}

pub fn usage() -> String {
    let mut lines = vec![
        color(
            COLOR_HEADING,
            &format!("Canic Operator CLI v{}", env!("CARGO_PKG_VERSION")),
        ),
        String::new(),
        "Usage: canic [OPTIONS] <COMMAND>".to_string(),
        String::new(),
        color(COLOR_HEADING, "Commands:"),
    ];
    lines.extend(grouped_command_section(COMMAND_SPECS));
    lines.extend([
        String::new(),
        color(COLOR_HEADING, "Options:"),
        "      --icp <path>      Path to the icp executable for ICP-backed commands".to_string(),
        "      --network <name>  ICP CLI network for networked commands".to_string(),
        "  -V, --version  Print version".to_string(),
        "  -h, --help     Print help".to_string(),
        String::new(),
        format!(
            "{}Tip:{} Run {} for command-specific help.",
            COLOR_TIP,
            COLOR_RESET,
            color(COLOR_COMMAND, "`canic <command> help`")
        ),
    ]);
    lines.join("\n")
}

fn grouped_command_section(specs: &[CommandSpec]) -> Vec<String> {
    let mut lines = Vec::new();
    let scopes = [
        CommandScope::Global,
        CommandScope::FleetContext,
        CommandScope::BackupRestore,
    ];
    for scope in scopes {
        let scope_specs = specs
            .iter()
            .filter(|spec| spec.scope == scope)
            .collect::<Vec<_>>();
        if scope_specs.is_empty() {
            continue;
        }
        if !lines.is_empty() {
            lines.push(String::new());
        }
        lines.push(format!("  {}", color(COLOR_GROUP, scope.heading())));
        for spec in scope_specs {
            let command = format!("{:<12}", spec.name);
            lines.push(format!(
                "    {} {}",
                color(COLOR_COMMAND, &command),
                spec.about
            ));
        }
    }
    lines
}

fn color(code: &str, text: &str) -> String {
    format!("{code}{text}{COLOR_RESET}")
}

#[cfg(test)]
pub fn strip_ansi(text: &str) -> String {
    let mut plain = String::new();
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' && chars.peek() == Some(&'[') {
            chars.next();
            for ch in chars.by_ref() {
                if ch == 'm' {
                    break;
                }
            }
            continue;
        }
        plain.push(ch);
    }
    plain
}

#[cfg(test)]
pub const fn color_heading() -> &'static str {
    COLOR_HEADING
}

#[cfg(test)]
pub const fn color_group() -> &'static str {
    COLOR_GROUP
}

#[cfg(test)]
pub const fn color_command() -> &'static str {
    COLOR_COMMAND
}
