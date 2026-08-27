//! Module: canic_cli::cli::help
//!
//! Responsibility: render top-level CLI help and detect help/version requests.
//! Does not own: command execution, command-specific help text, or global option forwarding.
//! Boundary: defines the top-level command catalog shared by help and dispatch.

use crate::cli::globals::{DISPATCH_ARGS, environment_arg, icp_arg};
use clap::{Arg, ColorChoice, Command};
use std::ffi::OsString;

const TOP_LEVEL_HELP_TEMPLATE: &str = "Canic Operator CLI v{version}\n{about-with-newline}\n{usage-heading} {usage}\n\n{before-help}\x1b[1mOptions:\x1b[0m\n{options}{after-help}\n";
const COLOR_RESET: &str = "\x1b[0m";
const COLOR_HEADING: &str = "\x1b[1m";
const COLOR_COMMAND: &str = "\x1b[38;5;109m";
const COLOR_TIP: &str = "\x1b[38;5;245m";

/// One top-level command shown in help and accepted by dispatch.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct CommandSpec {
    pub(super) name: &'static str,
    about: &'static str,
}

pub(super) const COMMAND_SPECS: &[CommandSpec] = &[
    CommandSpec {
        name: "admission",
        about: "Plan, apply, and inspect Fleet ingress admission",
    },
    CommandSpec {
        name: "app",
        about: "Manage Canic source apps and roles",
    },
    CommandSpec {
        name: "auth",
        about: "Inspect delegated-auth operation state",
    },
    CommandSpec {
        name: "backup",
        about: "Plan, inspect, and verify backups",
    },
    CommandSpec {
        name: "blob-storage",
        about: "Inspect and manage blob-storage billing",
    },
    CommandSpec {
        name: "build",
        about: "Build Canic App and infrastructure artifacts",
    },
    CommandSpec {
        name: "cycles",
        about: "Inspect and transfer cycles for current Fleets",
    },
    CommandSpec {
        name: "diagnostic",
        about: "Look up one compact Canic diagnostic code",
    },
    CommandSpec {
        name: "evidence",
        about: "Evaluate stable evidence envelopes",
    },
    CommandSpec {
        name: "fleet",
        about: "Converge one Fleet from current desired state",
    },
    CommandSpec {
        name: "info",
        about: "Inspect one terminal current Fleet",
    },
    CommandSpec {
        name: "inspect",
        about: "Inspect one current Fleet canister runtime",
    },
    CommandSpec {
        name: "medic",
        about: "Diagnose workspace and current-Fleet readiness",
    },
    CommandSpec {
        name: "network",
        about: "Enroll canonical network trust identities",
    },
    CommandSpec {
        name: "replica",
        about: "Manage the local ICP replica",
    },
    CommandSpec {
        name: "restore",
        about: "Plan or run snapshot restores",
    },
    CommandSpec {
        name: "scaffold",
        about: "Scaffold Canic source roles",
    },
    CommandSpec {
        name: "state",
        about: "Audit declared Canic state metadata",
    },
    CommandSpec {
        name: "status",
        about: "Show quick local workspace status",
    },
    CommandSpec {
        name: "token",
        about: "Wrap ICP token balance and transfer commands",
    },
];

fn is_help_arg(arg: &OsString) -> bool {
    arg.to_str()
        .is_some_and(|arg| matches!(arg, "--help" | "-h"))
}

fn is_version_arg(arg: &OsString) -> bool {
    arg.to_str()
        .is_some_and(|arg| matches!(arg, "--version" | "-V"))
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
        .about("Operator CLI for current Canic Apps and Fleets")
        .color(ColorChoice::Always)
        .subcommand_required(true)
        .arg(icp_arg())
        .arg(environment_arg())
        .subcommand_help_heading("Commands")
        .help_template(TOP_LEVEL_HELP_TEMPLATE)
        .before_help(format!(
            "{}Commands:{}\n{}",
            COLOR_HEADING,
            COLOR_RESET,
            command_section(COMMAND_SPECS).join("\n")
        ))
        .after_help(format!(
            "\n{}Tip:{} Run {} for command-specific help.",
            COLOR_TIP,
            COLOR_RESET,
            color(COLOR_COMMAND, "`canic <command> --help`")
        ));

    COMMAND_SPECS.iter().fold(command, |command, spec| {
        command.subcommand(
            Command::new(spec.name)
                .about(spec.about)
                .disable_help_flag(true)
                .disable_version_flag(true)
                .arg(
                    Arg::new(DISPATCH_ARGS)
                        .num_args(0..)
                        .allow_hyphen_values(true)
                        .trailing_var_arg(true)
                        .value_parser(clap::value_parser!(OsString))
                        .hide(true),
                ),
        )
    })
}

/// Render Canic's custom colorized top-level usage text.
#[cfg(test)]
pub fn usage() -> String {
    let help = top_level_command().render_help();
    help.ansi().to_string()
}

fn command_section(specs: &[CommandSpec]) -> Vec<String> {
    specs
        .iter()
        .map(|spec| {
            let command = format!("{:<12}", spec.name);
            format!("  {} {}", color(COLOR_COMMAND, &command), spec.about)
        })
        .collect()
}

fn color(code: &str, text: &str) -> String {
    format!("{code}{text}{COLOR_RESET}")
}

// -----------------------------------------------------------------------------
// Tests

#[cfg(test)]
mod tests {
    use super::*;

    // Ensure top-level usage keeps the intended help colors.
    #[test]
    fn usage_contains_help_colors() {
        let text = usage();

        assert!(text.contains(COLOR_HEADING));
        assert!(text.contains(COLOR_COMMAND));
    }

    #[test]
    fn first_arg_help_and_version_detection_accepts_flags() {
        assert!(first_arg_is_help(&[OsString::from("--help")]));
        assert!(first_arg_is_help(&[OsString::from("-h")]));
        assert!(first_arg_is_version(&[OsString::from("--version")]));
        assert!(first_arg_is_version(&[OsString::from("-V")]));
    }
}
