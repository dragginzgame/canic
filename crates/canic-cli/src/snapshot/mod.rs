//! Module: canic_cli::snapshot
//!
//! Responsibility: dispatch snapshot operator commands.
//! Does not own: backup artifact planning, ICP snapshot execution, or registry
//! traversal.
//! Boundary: parses the snapshot command family and delegates leaf behavior.

mod download;

use crate::{
    cli::clap::{parse_required_subcommand, passthrough_subcommand, render_usage},
    cli::help::print_help_or_version,
    version_text,
};
use canic_backup::snapshot::SnapshotDownloadError;
use canic_host::{
    icp::IcpCommandError, icp_config::IcpConfigError,
    installed_deployment::InstalledDeploymentError, registry::RegistryParseError,
    replica_query::ReplicaQueryError,
};
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

    #[error("snapshot download needs an installed deployment name")]
    MissingSnapshotSource,

    #[error("cannot combine deployment root {deployment_root} with --root {root}")]
    ConflictingDeploymentRoot {
        deployment_root: String,
        root: String,
    },

    #[error("canister {canister} is not a member of deployment {deployment}")]
    CanisterNotInDeployment {
        deployment: String,
        canister: String,
    },

    #[error(transparent)]
    Icp(#[from] IcpCommandError),

    #[error("failed to read canic deployment state: {0}")]
    IcpRoot(#[source] IcpConfigError),

    #[error(transparent)]
    InstalledDeployment(#[from] InstalledDeploymentError),

    #[error("local replica query failed: {0}")]
    LocalReplicaQuery(#[from] ReplicaQueryError),

    #[error(transparent)]
    Registry(#[from] RegistryParseError),

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

    let (command, args) = parse_required_subcommand(snapshot_command(), args)
        .map_err(|_| SnapshotCommandError::Usage(usage()))?;

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
    render_usage(snapshot_command)
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
