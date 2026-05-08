mod args;
mod backup;
mod build;
mod fleets;
mod install;
mod list;
mod manifest;
mod medic;
mod network;
mod output;
mod restore;
mod scaffold;
mod snapshot;
#[cfg(test)]
mod test_support;

use crate::args::first_arg_is_version;
use clap::{Arg, ArgAction, Command};
use std::ffi::OsString;
use thiserror::Error as ThisError;

const VERSION_TEXT: &str = concat!("canic ", env!("CARGO_PKG_VERSION"));
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
enum CommandScope {
    Global,
    FleetContext,
    WorkspaceFiles,
}

impl CommandScope {
    // Return the heading used in grouped top-level help.
    const fn heading(self) -> &'static str {
        match self {
            Self::Global => "Global commands",
            Self::FleetContext => "Fleet commands",
            Self::WorkspaceFiles => "Workspace and file commands",
        }
    }
}

///
/// CommandSpec
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CommandSpec {
    name: &'static str,
    about: &'static str,
    scope: CommandScope,
}

const COMMAND_SPECS: &[CommandSpec] = &[
    CommandSpec {
        name: "network",
        about: "Show network command guidance",
        scope: CommandScope::Global,
    },
    CommandSpec {
        name: "fleet",
        about: "Manage Canic fleets",
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
        name: "medic",
        about: "Diagnose local Canic fleet setup",
        scope: CommandScope::FleetContext,
    },
    CommandSpec {
        name: "snapshot",
        about: "Capture and download canister snapshots",
        scope: CommandScope::FleetContext,
    },
    CommandSpec {
        name: "build",
        about: "Build one Canic canister artifact",
        scope: CommandScope::WorkspaceFiles,
    },
    CommandSpec {
        name: "backup",
        about: "Verify backup directories and journal status",
        scope: CommandScope::WorkspaceFiles,
    },
    CommandSpec {
        name: "manifest",
        about: "Validate fleet backup manifests",
        scope: CommandScope::WorkspaceFiles,
    },
    CommandSpec {
        name: "restore",
        about: "Plan or run snapshot restores",
        scope: CommandScope::WorkspaceFiles,
    },
];

///
/// CliError
///

#[derive(Debug, ThisError)]
pub enum CliError {
    #[error("{0}")]
    Usage(String),

    #[error("backup: {0}")]
    Backup(String),

    #[error("build: {0}")]
    Build(String),

    #[error("config: {0}")]
    Config(String),

    #[error("install: {0}")]
    Install(String),

    #[error("fleet: {0}")]
    Fleets(String),

    #[error("list: {0}")]
    List(String),

    #[error("manifest: {0}")]
    Manifest(String),

    #[error("medic: {0}")]
    Medic(String),

    #[error("network: {0}")]
    Network(String),

    #[error("snapshot: {0}")]
    Snapshot(String),

    #[error("restore: {0}")]
    Restore(String),
}

impl From<backup::BackupCommandError> for CliError {
    // Keep backup command internals private while preserving operator-facing messages.
    fn from(err: backup::BackupCommandError) -> Self {
        Self::Backup(err.to_string())
    }
}

impl From<build::BuildCommandError> for CliError {
    // Keep build command internals private while preserving operator-facing messages.
    fn from(err: build::BuildCommandError) -> Self {
        Self::Build(err.to_string())
    }
}

impl From<install::InstallCommandError> for CliError {
    // Keep install command internals private while preserving operator-facing messages.
    fn from(err: install::InstallCommandError) -> Self {
        Self::Install(err.to_string())
    }
}

impl From<fleets::FleetCommandError> for CliError {
    // Keep fleet command internals private while preserving operator-facing messages.
    fn from(err: fleets::FleetCommandError) -> Self {
        Self::Fleets(err.to_string())
    }
}

impl From<list::ListCommandError> for CliError {
    // Keep list command internals private while preserving operator-facing messages.
    fn from(err: list::ListCommandError) -> Self {
        Self::List(err.to_string())
    }
}

impl From<manifest::ManifestCommandError> for CliError {
    // Keep manifest command internals private while preserving operator-facing messages.
    fn from(err: manifest::ManifestCommandError) -> Self {
        Self::Manifest(err.to_string())
    }
}

impl From<medic::MedicCommandError> for CliError {
    // Keep medic command internals private while preserving operator-facing messages.
    fn from(err: medic::MedicCommandError) -> Self {
        Self::Medic(err.to_string())
    }
}

impl From<network::NetworkCommandError> for CliError {
    // Keep network command internals private while preserving operator-facing messages.
    fn from(err: network::NetworkCommandError) -> Self {
        Self::Network(err.to_string())
    }
}

impl From<snapshot::SnapshotCommandError> for CliError {
    // Keep snapshot command internals private while preserving operator-facing messages.
    fn from(err: snapshot::SnapshotCommandError) -> Self {
        Self::Snapshot(err.to_string())
    }
}

impl From<restore::RestoreCommandError> for CliError {
    // Keep restore command internals private while preserving operator-facing messages.
    fn from(err: restore::RestoreCommandError) -> Self {
        Self::Restore(err.to_string())
    }
}

/// Run the CLI from process arguments.
pub fn run_from_env() -> Result<(), CliError> {
    run(std::env::args_os().skip(1))
}

/// Run the CLI from an argument iterator.
pub fn run<I>(args: I) -> Result<(), CliError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if first_arg_is_version(&args) {
        println!("{}", version_text());
        return Ok(());
    }

    let mut args = args.into_iter();
    let Some(command) = args.next().and_then(|arg| arg.into_string().ok()) else {
        return Err(CliError::Usage(usage()));
    };

    match command.as_str() {
        "backup" => backup::run(args).map_err(CliError::from),
        "build" => build::run(args).map_err(CliError::from),
        "config" => list::run_config(args).map_err(|err| CliError::Config(err.to_string())),
        "fleet" => fleets::run(args).map_err(CliError::from),
        "install" => install::run(args).map_err(CliError::from),
        "list" => list::run(args).map_err(CliError::from),
        "manifest" => manifest::run(args).map_err(CliError::from),
        "medic" => medic::run(args).map_err(CliError::from),
        "network" => network::run(args).map_err(CliError::from),
        "snapshot" => snapshot::run(args).map_err(CliError::from),
        "restore" => restore::run(args).map_err(CliError::from),
        "help" | "--help" | "-h" => {
            println!("{}", usage());
            Ok(())
        }
        _ => Err(CliError::Usage(usage())),
    }
}

/// Build the top-level command metadata.
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
        .subcommand_help_heading("Commands")
        .help_template(TOP_LEVEL_HELP_TEMPLATE)
        .before_help(grouped_command_section(COMMAND_SPECS).join("\n"))
        .after_help("Run `canic <command> help` for command-specific help.");

    COMMAND_SPECS.iter().fold(command, |command, spec| {
        command.subcommand(Command::new(spec.name).about(spec.about))
    })
}

/// Return the CLI version banner.
#[must_use]
pub const fn version_text() -> &'static str {
    VERSION_TEXT
}

// Return the top-level usage text.
fn usage() -> String {
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

// Render grouped command rows from the same metadata used to build Clap subcommands.
fn grouped_command_section(specs: &[CommandSpec]) -> Vec<String> {
    let mut lines = Vec::new();
    let scopes = [
        CommandScope::Global,
        CommandScope::FleetContext,
        CommandScope::WorkspaceFiles,
    ];
    for (index, scope) in scopes.into_iter().enumerate() {
        lines.push(format!("  {}", color(COLOR_GROUP, scope.heading())));
        for spec in specs.iter().filter(|spec| spec.scope == scope) {
            let command = format!("{:<12}", spec.name);
            lines.push(format!(
                "    {} {}",
                color(COLOR_COMMAND, &command),
                spec.about
            ));
        }
        if index + 1 < scopes.len() {
            lines.push(String::new());
        }
    }
    lines
}

// Wrap one help fragment in an ANSI color sequence.
fn color(code: &str, text: &str) -> String {
    format!("{code}{text}{COLOR_RESET}")
}

#[cfg(test)]
mod tests {
    use super::*;

    // Ensure top-level help stays compact as command surfaces grow.
    #[test]
    fn usage_lists_command_families() {
        let text = usage();
        let plain = strip_ansi(&text);

        assert!(plain.contains(&format!(
            "Canic Operator CLI v{}",
            env!("CARGO_PKG_VERSION")
        )));
        assert!(plain.contains("Usage: canic [OPTIONS] <COMMAND>"));
        assert!(plain.contains("\nCommands:\n"));
        assert!(plain.contains("Global commands"));
        assert!(plain.contains("Fleet commands"));
        assert!(plain.contains("Workspace and file commands"));
        assert!(plain.find("    network") < plain.find("    fleet"));
        assert!(plain.find("    fleet") < plain.find("    install"));
        assert!(plain.find("    install") < plain.find("    config"));
        assert!(plain.find("    config") < plain.find("    list"));
        assert!(plain.contains("Options:"));
        assert!(!plain.contains("    scaffold"));
        assert!(plain.contains("config"));
        assert!(plain.contains("list"));
        assert!(plain.contains("build"));
        assert!(plain.contains("network"));
        assert!(!plain.contains("    defaults"));
        assert!(!plain.contains("    status"));
        assert!(plain.contains("fleet"));
        assert!(plain.contains("install"));
        assert!(plain.contains("snapshot"));
        assert!(plain.contains("backup"));
        assert!(plain.contains("manifest"));
        assert!(plain.contains("medic"));
        assert!(plain.contains("restore"));
        assert!(plain.contains("Tip: Run `canic <command> help`"));
        assert!(text.contains(COLOR_HEADING));
        assert!(text.contains(COLOR_GROUP));
        assert!(text.contains(COLOR_COMMAND));
    }

    // Ensure command-family help paths return successfully instead of erroring.
    #[test]
    fn command_family_help_returns_ok() {
        for args in [
            &["backup", "help"][..],
            &["backup", "list", "help"],
            &["backup", "status", "help"],
            &["backup", "verify", "help"],
            &["build", "help"],
            &["config", "help"],
            &["install", "help"],
            &["fleet"],
            &["fleet", "help"],
            &["fleet", "create", "help"],
            &["fleet", "list", "help"],
            &["fleet", "delete", "help"],
            &["list", "help"],
            &["restore", "help"],
            &["restore", "plan", "help"],
            &["restore", "apply", "help"],
            &["restore", "run", "help"],
            &["manifest", "help"],
            &["manifest", "validate", "help"],
            &["medic", "help"],
            &["network"],
            &["network", "help"],
            &["network", "current", "help"],
            &["snapshot", "help"],
            &["snapshot", "download", "help"],
        ] {
            assert_run_ok(args);
        }
    }

    // Ensure version flags are accepted at the top level and command-family level.
    #[test]
    fn version_flags_return_ok() {
        assert_eq!(version_text(), concat!("canic ", env!("CARGO_PKG_VERSION")));
        assert!(run([OsString::from("--version")]).is_ok());
        assert!(
            run([
                OsString::from("backup"),
                OsString::from("list"),
                OsString::from("--dir"),
                OsString::from("version")
            ])
            .is_ok()
        );
        assert!(run([OsString::from("backup"), OsString::from("--version")]).is_ok());
        assert!(
            run([
                OsString::from("backup"),
                OsString::from("list"),
                OsString::from("--version")
            ])
            .is_ok()
        );
        assert!(run([OsString::from("build"), OsString::from("--version")]).is_ok());
        assert!(run([OsString::from("config"), OsString::from("--version")]).is_ok());
        assert!(run([OsString::from("install"), OsString::from("--version")]).is_ok());
        assert!(run([OsString::from("fleet"), OsString::from("--version")]).is_ok());
        assert!(
            run([
                OsString::from("fleet"),
                OsString::from("create"),
                OsString::from("--version")
            ])
            .is_ok()
        );
        assert!(run([OsString::from("list"), OsString::from("--version")]).is_ok());
        assert!(run([OsString::from("restore"), OsString::from("--version")]).is_ok());
        assert!(run([OsString::from("manifest"), OsString::from("--version")]).is_ok());
        assert!(run([OsString::from("medic"), OsString::from("--version")]).is_ok());
        assert!(run([OsString::from("network"), OsString::from("--version")]).is_ok());
        assert!(
            run([
                OsString::from("network"),
                OsString::from("current"),
                OsString::from("--version")
            ])
            .is_ok()
        );
        assert!(run([OsString::from("snapshot"), OsString::from("--version")]).is_ok());
        assert!(
            run([
                OsString::from("snapshot"),
                OsString::from("download"),
                OsString::from("--version")
            ])
            .is_ok()
        );
    }

    // Remove ANSI color sequences so tests can assert help structure.
    fn strip_ansi(text: &str) -> String {
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

    // Assert that a CLI argv slice returns successfully.
    fn assert_run_ok(raw_args: &[&str]) {
        let args = raw_args.iter().map(OsString::from).collect::<Vec<_>>();
        assert!(
            run(args).is_ok(),
            "expected successful run for {raw_args:?}"
        );
    }
}
