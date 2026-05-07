mod args;
pub mod backup;
pub mod build;
pub mod fleets;
pub mod install;
pub mod list;
pub mod manifest;
mod output;
pub mod release_set;
pub mod restore;
pub mod snapshot;

use crate::args::any_arg_is_version;
use clap::{Arg, ArgAction, Command};
use std::ffi::OsString;
use thiserror::Error as ThisError;

const VERSION_TEXT: &str = concat!("canic ", env!("CARGO_PKG_VERSION"));
const TOP_LEVEL_HELP_TEMPLATE: &str = "{about-with-newline}\n{usage-heading} {usage}\n\n{before-help}Options:\n{options}{after-help}\n";

///
/// CommandScope
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CommandScope {
    MultiFleet,
    SingleFleet,
    SingleCanister,
}

impl CommandScope {
    // Return the heading used in grouped top-level help.
    const fn heading(self) -> &'static str {
        match self {
            Self::MultiFleet => "Multi-fleet commands",
            Self::SingleFleet => "Single-fleet commands",
            Self::SingleCanister => "Single-canister commands",
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
        name: "fleets",
        about: "List installed Canic fleets",
        scope: CommandScope::MultiFleet,
    },
    CommandSpec {
        name: "use",
        about: "Select the current Canic fleet",
        scope: CommandScope::MultiFleet,
    },
    CommandSpec {
        name: "install",
        about: "Install and bootstrap a Canic fleet",
        scope: CommandScope::SingleFleet,
    },
    CommandSpec {
        name: "list",
        about: "Show registry canisters as a tree table",
        scope: CommandScope::SingleFleet,
    },
    CommandSpec {
        name: "backup",
        about: "Verify backup directories and journal status",
        scope: CommandScope::SingleFleet,
    },
    CommandSpec {
        name: "manifest",
        about: "Validate fleet backup manifests",
        scope: CommandScope::SingleFleet,
    },
    CommandSpec {
        name: "release-set",
        about: "Inspect, emit, or stage root release-set artifacts",
        scope: CommandScope::SingleFleet,
    },
    CommandSpec {
        name: "restore",
        about: "Plan or run snapshot restores",
        scope: CommandScope::SingleFleet,
    },
    CommandSpec {
        name: "build",
        about: "Build one Canic canister artifact",
        scope: CommandScope::SingleCanister,
    },
    CommandSpec {
        name: "snapshot",
        about: "Capture and download canister snapshots",
        scope: CommandScope::SingleCanister,
    },
];

///
/// CliError
///

#[derive(Debug, ThisError)]
pub enum CliError {
    #[error("{0}")]
    Usage(String),

    #[error(transparent)]
    Backup(#[from] backup::BackupCommandError),

    #[error(transparent)]
    Build(#[from] build::BuildCommandError),

    #[error(transparent)]
    Install(#[from] install::InstallCommandError),

    #[error(transparent)]
    Fleets(#[from] fleets::FleetCommandError),

    #[error(transparent)]
    List(#[from] list::ListCommandError),

    #[error(transparent)]
    Manifest(#[from] manifest::ManifestCommandError),

    #[error(transparent)]
    Snapshot(#[from] snapshot::SnapshotCommandError),

    #[error(transparent)]
    ReleaseSet(#[from] release_set::ReleaseSetCommandError),

    #[error(transparent)]
    Restore(#[from] restore::RestoreCommandError),
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
    if any_arg_is_version(&args) {
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
        "fleets" => fleets::run(args).map_err(CliError::from),
        "install" => install::run(args).map_err(CliError::from),
        "list" => list::run(args).map_err(CliError::from),
        "manifest" => manifest::run(args).map_err(CliError::from),
        "release-set" => release_set::run(args).map_err(CliError::from),
        "snapshot" => snapshot::run(args).map_err(CliError::from),
        "restore" => restore::run(args).map_err(CliError::from),
        "use" => fleets::run_use(args).map_err(CliError::from),
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
        .before_help(grouped_command_section(COMMAND_SPECS))
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
    let mut command = top_level_command();
    command.render_help().to_string()
}

// Render grouped command rows from the same metadata used to build Clap subcommands.
fn grouped_command_section(specs: &[CommandSpec]) -> String {
    let mut lines = Vec::new();
    let scopes = [
        CommandScope::MultiFleet,
        CommandScope::SingleFleet,
        CommandScope::SingleCanister,
    ];
    for (index, scope) in scopes.into_iter().enumerate() {
        lines.push(format!("{}:", scope.heading()));
        for spec in specs.iter().filter(|spec| spec.scope == scope) {
            lines.push(format!("  {:<11} {}", spec.name, spec.about));
        }
        if index + 1 < scopes.len() {
            lines.push(String::new());
        }
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    // Ensure top-level help stays compact as command surfaces grow.
    #[test]
    fn usage_lists_command_families() {
        let text = usage();

        assert!(text.contains("Usage: canic"));
        assert!(text.contains("Multi-fleet commands"));
        assert!(text.contains("Single-fleet commands"));
        assert!(text.contains("Single-canister commands"));
        assert!(!text.contains("\nCommands:\n"));
        assert!(text.find("Multi-fleet commands") < text.find("Single-fleet commands"));
        assert!(text.find("Single-fleet commands") < text.find("Single-canister commands"));
        assert!(text.contains("list"));
        assert!(text.contains("build"));
        assert!(text.contains("fleets"));
        assert!(text.contains("use"));
        assert!(text.contains("install"));
        assert!(text.contains("snapshot"));
        assert!(text.contains("backup"));
        assert!(text.contains("manifest"));
        assert!(text.contains("release-set"));
        assert!(text.contains("restore"));
        assert!(text.contains("canic <command> help"));
    }

    // Ensure command-family help paths return successfully instead of erroring.
    #[test]
    fn command_family_help_returns_ok() {
        assert!(run([OsString::from("backup"), OsString::from("help")]).is_ok());
        assert!(run([OsString::from("build"), OsString::from("help")]).is_ok());
        assert!(run([OsString::from("install"), OsString::from("help")]).is_ok());
        assert!(run([OsString::from("fleets"), OsString::from("help")]).is_ok());
        assert!(run([OsString::from("list"), OsString::from("help")]).is_ok());
        assert!(run([OsString::from("restore"), OsString::from("help")]).is_ok());
        assert!(run([OsString::from("manifest"), OsString::from("help")]).is_ok());
        assert!(run([OsString::from("release-set"), OsString::from("help")]).is_ok());
        assert!(run([OsString::from("snapshot"), OsString::from("help")]).is_ok());
        assert!(run([OsString::from("use"), OsString::from("help")]).is_ok());
    }

    // Ensure version flags are accepted at the top level and command-family level.
    #[test]
    fn version_flags_return_ok() {
        assert_eq!(version_text(), concat!("canic ", env!("CARGO_PKG_VERSION")));
        assert!(run([OsString::from("--version")]).is_ok());
        assert!(run([OsString::from("backup"), OsString::from("--version")]).is_ok());
        assert!(run([OsString::from("build"), OsString::from("--version")]).is_ok());
        assert!(run([OsString::from("install"), OsString::from("--version")]).is_ok());
        assert!(run([OsString::from("fleets"), OsString::from("--version")]).is_ok());
        assert!(run([OsString::from("list"), OsString::from("--version")]).is_ok());
        assert!(run([OsString::from("restore"), OsString::from("--version")]).is_ok());
        assert!(run([OsString::from("manifest"), OsString::from("--version")]).is_ok());
        assert!(run([OsString::from("release-set"), OsString::from("--version")]).is_ok());
        assert!(run([OsString::from("snapshot"), OsString::from("--version")]).is_ok());
        assert!(run([OsString::from("use"), OsString::from("--version")]).is_ok());
    }
}
