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
    "usage: canic snapshot download --canister <id> --out <dir> [--root <id> | --registry-json <file>] [--include-children] [--recursive] [--dry-run] [--stop-before-snapshot] [--resume-after-snapshot] [--network <name>]\n       canic manifest validate --manifest <file>\n       canic restore plan --manifest <file> [--mapping <file>] [--out <file>]"
}
