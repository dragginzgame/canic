mod download;

use crate::{
    cli::clap::{parse_subcommand, passthrough_subcommand},
    cli::help::print_help_or_version,
    version_text,
};
use canic_backup::snapshot::SnapshotDownloadError;
use canic_host::replica_query::ReplicaQueryError;
use clap::Command as ClapCommand;
use std::ffi::OsString;
use thiserror::Error as ThisError;

use download::{SnapshotDownloadOptions, download_snapshots, download_usage};

///
/// SnapshotCommandError
///

#[derive(Debug, ThisError)]
pub enum SnapshotCommandError {
    #[error("{0}")]
    Usage(String),

    #[error("snapshot download needs an installed fleet name")]
    MissingSnapshotSource,

    #[error("cannot combine fleet root {fleet_root} with --root {root}")]
    ConflictingFleetRoot { fleet_root: String, root: String },

    #[error("canister {canister} is not a member of fleet {fleet}")]
    CanisterNotInFleet { fleet: String, canister: String },

    #[error("icp command failed: {command}\n{stderr}")]
    IcpFailed { command: String, stderr: String },

    #[error("failed to read canic fleet state: {0}")]
    InstallState(String),

    #[error("could not parse snapshot id from icp output: {0}")]
    SnapshotIdUnavailable(String),

    #[error("local replica query failed: {0}")]
    LocalReplicaQuery(#[from] ReplicaQueryError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    SnapshotDownload(#[from] SnapshotDownloadError),
}

pub fn run<I>(args: I) -> Result<(), SnapshotCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    let Some((command, args)) = parse_subcommand(snapshot_command(), args)
        .map_err(|_| SnapshotCommandError::Usage(usage()))?
    else {
        return Err(SnapshotCommandError::Usage(usage()));
    };

    match command.as_str() {
        "download" => run_download(args),
        _ => unreachable!("snapshot dispatch command only defines known commands"),
    }
}

fn run_download<I>(args: I) -> Result<(), SnapshotCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, download_usage, version_text()) {
        return Ok(());
    }

    let options = SnapshotDownloadOptions::parse(args)?;
    let result = download_snapshots(&options)?;
    for command in result.planned_commands {
        println!("{command}");
    }
    for artifact in result.artifacts {
        println!(
            "{} {} {}",
            artifact.canister_id,
            artifact.snapshot_id,
            artifact.path.display()
        );
    }
    Ok(())
}

fn usage() -> String {
    let mut command = snapshot_command();
    command.render_help().to_string()
}

fn snapshot_command() -> ClapCommand {
    ClapCommand::new("snapshot")
        .bin_name("canic snapshot")
        .about("Capture and download canister snapshots")
        .disable_help_flag(true)
        .subcommand(passthrough_subcommand(
            ClapCommand::new("download")
                .about("Download canister snapshots for one canister or subtree")
                .disable_help_flag(true),
        ))
}
