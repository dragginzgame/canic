pub mod backup;
pub mod build;
pub mod fleets;
pub mod install;
pub mod list;
pub mod manifest;
pub mod release_set;
pub mod restore;
pub mod snapshot;

mod args;
mod output;

use crate::args::any_arg_is_version;
use clap::{Arg, ArgAction, Command};
use std::ffi::OsString;
use thiserror::Error as ThisError;

const VERSION_TEXT: &str = concat!("canic ", env!("CARGO_PKG_VERSION"));

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
    Command::new("canic")
        .about("Operator CLI for Canic install, backup, and restore workflows")
        .disable_version_flag(true)
        .arg(
            Arg::new("version")
                .short('V')
                .long("version")
                .action(ArgAction::SetTrue)
                .help("Print version"),
        )
        .subcommand(Command::new("install").about("Install and bootstrap a Canic fleet"))
        .subcommand(Command::new("build").about("Build one Canic canister artifact"))
        .subcommand(Command::new("fleets").about("List installed Canic fleets"))
        .subcommand(Command::new("use").about("Select the current Canic fleet"))
        .subcommand(Command::new("list").about("Show registry canisters as a tree table"))
        .subcommand(Command::new("snapshot").about("Capture and download canister snapshots"))
        .subcommand(Command::new("backup").about("Verify backup directories and journal status"))
        .subcommand(Command::new("manifest").about("Validate fleet backup manifests"))
        .subcommand(
            Command::new("release-set").about("Inspect, emit, or stage root release-set artifacts"),
        )
        .subcommand(Command::new("restore").about("Plan or run snapshot restores"))
        .after_help("Run `canic <command> help` for command-specific help.")
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

#[cfg(test)]
mod tests {
    use super::*;

    // Ensure top-level help stays compact as command surfaces grow.
    #[test]
    fn usage_lists_command_families() {
        let text = usage();

        assert!(text.contains("Usage: canic"));
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
