mod backup;
mod cli;
mod cycles;
mod endpoints;
mod fleets;
mod install;
mod list;
mod manifest;
mod medic;
mod metrics;
mod output;
mod replica;
mod restore;
mod scaffold;
mod snapshot;
mod status;
mod support;
#[cfg(test)]
mod test_support;

use crate::cli::{
    clap::parse_matches,
    globals::{
        DISPATCH_ARGS, apply_global_icp, apply_global_network, command_local_global_option,
        top_level_dispatch_command,
    },
    help::{first_arg_is_help, usage},
};
pub use cli::top_level_command;
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

    #[error("backup: {0}")]
    Backup(#[from] backup::BackupCommandError),

    #[error("config: {0}")]
    Config(String),

    #[error("cycles: {0}")]
    Cycles(#[from] cycles::CyclesCommandError),

    #[error("endpoints: {0}")]
    Endpoints(#[from] endpoints::EndpointsCommandError),

    #[error("install: {0}")]
    Install(#[from] install::InstallCommandError),

    #[error("fleet: {0}")]
    Fleets(#[from] fleets::FleetCommandError),

    #[error("list: {0}")]
    List(#[from] list::ListCommandError),

    #[error("manifest: {0}")]
    Manifest(#[from] manifest::ManifestCommandError),

    #[error("medic: {0}")]
    Medic(#[from] medic::MedicCommandError),

    #[error("metrics: {0}")]
    Metrics(#[from] metrics::MetricsCommandError),

    #[error("snapshot: {0}")]
    Snapshot(#[from] snapshot::SnapshotCommandError),

    #[error("restore: {0}")]
    Restore(#[from] restore::RestoreCommandError),

    #[error("replica: {0}")]
    Replica(#[from] replica::ReplicaCommandError),

    #[error("status: {0}")]
    Status(#[from] status::StatusCommandError),
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
    if first_arg_is_help(&args) {
        println!("{}", usage());
        return Ok(());
    }
    if let Some(option) = command_local_global_option(&args) {
        return Err(CliError::Usage(format!(
            "{option} is a top-level option; put it before the command\n\n{}",
            usage()
        )));
    }

    let matches =
        parse_matches(top_level_dispatch_command(), args).map_err(|_| CliError::Usage(usage()))?;
    if matches.get_flag("version") {
        println!("{}", version_text());
        return Ok(());
    }
    let global_icp = matches.get_one::<String>("icp").cloned();
    let global_network = matches.get_one::<String>("network").cloned();

    let Some((command, subcommand_matches)) = matches.subcommand() else {
        return Err(CliError::Usage(usage()));
    };
    let mut tail = subcommand_matches
        .get_many::<OsString>(DISPATCH_ARGS)
        .map(|values| values.cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    apply_global_icp(command, &mut tail, global_icp);
    apply_global_network(command, &mut tail, global_network);
    let tail = tail.into_iter();

    match command {
        "backup" => backup::run(tail).map_err(CliError::from),
        "config" => list::run_config(tail).map_err(|err| CliError::Config(err.to_string())),
        "cycles" => cycles::run(tail).map_err(CliError::from),
        "endpoints" => endpoints::run(tail).map_err(CliError::from),
        "fleet" => fleets::run(tail).map_err(CliError::from),
        "install" => install::run(tail).map_err(CliError::from),
        "list" => list::run(tail).map_err(CliError::from),
        "manifest" => manifest::run(tail).map_err(CliError::from),
        "medic" => medic::run(tail).map_err(CliError::from),
        "metrics" => metrics::run(tail).map_err(CliError::from),
        "replica" => replica::run(tail).map_err(CliError::from),
        "snapshot" => snapshot::run(tail).map_err(CliError::from),
        "status" => status::run(tail).map_err(CliError::from),
        "restore" => restore::run(tail).map_err(CliError::from),
        _ => unreachable!("top-level dispatch command only defines known commands"),
    }
}

#[must_use]
pub const fn version_text() -> &'static str {
    VERSION_TEXT
}

#[cfg(test)]
mod tests;
