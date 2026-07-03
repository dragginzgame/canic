//! Module: canic_cli::cli::help
//!
//! Responsibility: render top-level CLI help and detect help/version requests.
//! Does not own: command execution, command-specific help text, or global option forwarding.
//! Boundary: defines the top-level command catalog shared by help and dispatch.

use crate::cli::globals::{icp_arg, network_arg};
use clap::{Arg, ArgAction, Command};
use std::ffi::OsString;

const TOP_LEVEL_HELP_TEMPLATE: &str = "{name} {version}\n{about-with-newline}\n{usage-heading} {usage}\n\n{before-help}Options:\n{options}{after-help}\n";
const COLOR_RESET: &str = "\x1b[0m";
const COLOR_HEADING: &str = "\x1b[1m";
const COLOR_GROUP: &str = "\x1b[38;5;245m";
const COLOR_COMMAND: &str = "\x1b[38;5;109m";
const COLOR_TIP: &str = "\x1b[38;5;245m";

/// Top-level help grouping for commands.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CommandScope {
    Project,
    Deployment,
    IcpWallet,
    BackupRestore,
}

impl CommandScope {
    const fn heading(self) -> &'static str {
        match self {
            Self::Project => "Project commands",
            Self::Deployment => "Deployment commands",
            Self::IcpWallet => "ICP wallet commands",
            Self::BackupRestore => "Backup and restore commands",
        }
    }
}

/// One top-level command shown in help and accepted by dispatch.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct CommandSpec {
    pub(super) name: &'static str,
    about: &'static str,
    scope: CommandScope,
}

pub(super) const COMMAND_SPECS: &[CommandSpec] = &[
    CommandSpec {
        name: "status",
        about: "Show quick Canic project status",
        scope: CommandScope::Project,
    },
    CommandSpec {
        name: "medic",
        about: "Diagnose project and deployment preflight readiness",
        scope: CommandScope::Project,
    },
    CommandSpec {
        name: "state",
        about: "Audit declared Canic state metadata",
        scope: CommandScope::Project,
    },
    CommandSpec {
        name: "fleet",
        about: "Manage Canic fleets and roles",
        scope: CommandScope::Project,
    },
    CommandSpec {
        name: "scaffold",
        about: "Scaffold Canic source files",
        scope: CommandScope::Project,
    },
    CommandSpec {
        name: "replica",
        about: "Manage the local ICP replica",
        scope: CommandScope::Project,
    },
    CommandSpec {
        name: "install",
        about: "Install and bootstrap a Canic fleet",
        scope: CommandScope::Deployment,
    },
    CommandSpec {
        name: "blob-storage",
        about: "Inspect and provision blob-storage billing",
        scope: CommandScope::Deployment,
    },
    CommandSpec {
        name: "auth",
        about: "Run delegated-auth operator workflows",
        scope: CommandScope::Deployment,
    },
    CommandSpec {
        name: "build",
        about: "Build one Canic canister artifact",
        scope: CommandScope::Deployment,
    },
    CommandSpec {
        name: "deploy",
        about: "Check, inspect, register, and install deployments",
        scope: CommandScope::Deployment,
    },
    CommandSpec {
        name: "evidence",
        about: "Evaluate stable evidence envelopes",
        scope: CommandScope::Deployment,
    },
    CommandSpec {
        name: "cycles",
        about: "Wrap ICP cycles balance and transfer commands",
        scope: CommandScope::IcpWallet,
    },
    CommandSpec {
        name: "token",
        about: "Wrap ICP token balance and transfer commands",
        scope: CommandScope::IcpWallet,
    },
    CommandSpec {
        name: "info",
        about: "Query deployed canister information",
        scope: CommandScope::Deployment,
    },
    CommandSpec {
        name: "snapshot",
        about: "Capture and download canister snapshots",
        scope: CommandScope::BackupRestore,
    },
    CommandSpec {
        name: "backup",
        about: "Plan, inspect, and verify backups",
        scope: CommandScope::BackupRestore,
    },
    CommandSpec {
        name: "restore",
        about: "Plan or run snapshot restores",
        scope: CommandScope::BackupRestore,
    },
];

fn is_help_arg(arg: &OsString) -> bool {
    arg.to_str()
        .is_some_and(|arg| matches!(arg, "help" | "--help" | "-h"))
}

fn is_version_arg(arg: &OsString) -> bool {
    arg.to_str()
        .is_some_and(|arg| matches!(arg, "version" | "--version" | "-V"))
}

/// Return whether the first CLI argument requests help.
pub fn first_arg_is_help(args: &[OsString]) -> bool {
    args.first().is_some_and(is_help_arg)
}

fn first_arg_is_version(args: &[OsString]) -> bool {
    args.first().is_some_and(is_version_arg)
}

/// Print help or version text when the first CLI argument requests it.
///
/// Returns `true` when the caller should stop command execution.
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
/// Build the top-level Clap command used for public help rendering.
pub fn top_level_command() -> Command {
    let command = Command::new("canic")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Operator CLI for Canic projects, deployments, backups, and ICP wallet workflows")
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

/// Render Canic's custom colorized top-level usage text.
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
        CommandScope::Project,
        CommandScope::Deployment,
        CommandScope::IcpWallet,
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

// -----------------------------------------------------------------------------
// Tests

#[cfg(test)]
mod tests {
    use super::*;

    // Ensure top-level usage keeps the intended color groups.
    #[test]
    fn usage_contains_help_colors() {
        let text = usage();

        assert!(text.contains(COLOR_HEADING));
        assert!(text.contains(COLOR_GROUP));
        assert!(text.contains(COLOR_COMMAND));
    }

    #[test]
    fn first_arg_help_and_version_detection_accepts_aliases() {
        assert!(first_arg_is_help(&[OsString::from("help")]));
        assert!(first_arg_is_help(&[OsString::from("--help")]));
        assert!(first_arg_is_help(&[OsString::from("-h")]));
        assert!(first_arg_is_version(&[OsString::from("version")]));
        assert!(first_arg_is_version(&[OsString::from("--version")]));
        assert!(first_arg_is_version(&[OsString::from("-V")]));
    }
}
