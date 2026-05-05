pub mod backup;
pub mod list;
pub mod manifest;
pub mod restore;
pub mod snapshot;

use std::ffi::OsString;
use thiserror::Error as ThisError;

///
/// CliError
///

#[derive(Debug, ThisError)]
pub enum CliError {
    #[error("{0}")]
    Usage(&'static str),

    #[error(transparent)]
    Backup(#[from] backup::BackupCommandError),

    #[error(transparent)]
    List(#[from] list::ListCommandError),

    #[error(transparent)]
    Manifest(#[from] manifest::ManifestCommandError),

    #[error(transparent)]
    Snapshot(#[from] snapshot::SnapshotCommandError),

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
    let mut args = args.into_iter();
    let Some(command) = args.next().and_then(|arg| arg.into_string().ok()) else {
        return Err(CliError::Usage(usage()));
    };

    match command.as_str() {
        "backup" => backup::run(args).map_err(CliError::from),
        "list" => list::run(args).map_err(CliError::from),
        "manifest" => manifest::run(args).map_err(CliError::from),
        "snapshot" => snapshot::run(args).map_err(CliError::from),
        "restore" => restore::run(args).map_err(CliError::from),
        "help" | "--help" | "-h" => {
            println!("{}", usage());
            Ok(())
        }
        _ => Err(CliError::Usage(usage())),
    }
}

// Return the top-level usage text.
const fn usage() -> &'static str {
    "usage: canic <command> [<args>]\n\ncommands:\n  list       Show registry canisters as an ASCII tree.\n  snapshot   Capture and download canister snapshots.\n  backup     Inspect, verify, preflight, or smoke-check a backup directory.\n  manifest   Validate fleet backup manifests.\n  restore    Plan, preview, summarize, or run restore journals.\n\nhelp:\n  canic help\n  canic <command> help"
}

#[cfg(test)]
mod tests {
    use super::*;

    // Ensure top-level help stays compact as command surfaces grow.
    #[test]
    fn usage_lists_command_families_without_nested_flags() {
        let text = usage();

        assert!(text.contains("usage: canic <command> [<args>]"));
        assert!(text.contains("list"));
        assert!(text.contains("snapshot"));
        assert!(text.contains("backup"));
        assert!(text.contains("manifest"));
        assert!(text.contains("restore"));
        assert!(text.contains("canic <command> help"));
        assert!(!text.contains("--require-batch-ready-delta"));
        assert!(!text.contains("--require-no-pending-before"));
    }

    // Ensure command-family help paths return successfully instead of erroring.
    #[test]
    fn command_family_help_returns_ok() {
        assert!(run([OsString::from("backup"), OsString::from("help")]).is_ok());
        assert!(run([OsString::from("list"), OsString::from("help")]).is_ok());
        assert!(run([OsString::from("restore"), OsString::from("help")]).is_ok());
        assert!(run([OsString::from("manifest"), OsString::from("help")]).is_ok());
        assert!(run([OsString::from("snapshot"), OsString::from("help")]).is_ok());
    }
}
