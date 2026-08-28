mod admission;
mod apps;
mod auth;
mod backup;
mod blob_storage;
mod build;
mod cli;
mod cycles;
mod diagnostic;
mod endpoints;
mod evidence;
mod evidence_support;
mod fleet;
mod info;
mod info_env;
mod info_subnets;
mod inspect;
mod list;
mod medic;
mod metrics;
mod network;
mod output;
mod replica;
mod restore;
mod scaffold;
mod state;
mod status;
mod support;
#[cfg(test)]
mod test_support;
mod token;
mod toolchain;

use crate::cli::{
    argv::trace_if_enabled,
    clap::parse_matches,
    globals::{
        DISPATCH_ARGS, apply_global_environment, apply_global_icp, global_environment_conflict,
        misplaced_global_option,
    },
};
use clap::error::ErrorKind;
pub use cli::top_level_command;
use std::ffi::OsString;
use thiserror::Error as ThisError;

const VERSION_TEXT: &str = concat!("canic ", env!("CARGO_PKG_VERSION"));

///
/// CliError
///

#[derive(Debug, ThisError)]
pub enum CliError {
    #[error("admission: {0}")]
    Admission(#[source] Box<admission::AdmissionCommandError>),

    #[error(transparent)]
    Clap(#[from] clap::Error),

    #[error("build: {0}")]
    Build(#[from] build::BuildCommandError),

    #[error("cycles: {0}")]
    Cycles(#[source] Box<cycles::CyclesCommandError>),

    #[error("backup: {0}")]
    Backup(#[source] Box<backup::BackupCommandError>),

    #[error("auth: {0}")]
    Auth(#[source] Box<auth::AuthCommandError>),

    #[error("blob-storage: {0}")]
    BlobStorage(#[source] Box<blob_storage::BlobStorageCommandError>),

    #[error("diagnostic: {0}")]
    Diagnostic(#[from] diagnostic::DiagnosticCommandError),

    #[error("evidence: {0}")]
    Evidence(#[from] evidence::EvidenceCommandError),

    #[error("fleet: {0}")]
    Fleet(#[source] Box<fleet::FleetCommandError>),

    #[error("info: {0}")]
    Info(#[from] info::InfoCommandError),

    #[error("inspect: {0}")]
    Inspect(#[source] Box<inspect::InspectCommandError>),

    #[error("medic: {0}")]
    Medic(#[source] Box<medic::MedicCommandError>),

    #[error("network: {0}")]
    Network(#[from] network::NetworkCommandError),

    #[error("app: {0}")]
    Apps(#[source] Box<apps::AppCommandError>),

    #[error("state: {0}")]
    State(#[from] state::StateCommandError),

    #[error("status: {0}")]
    Status(#[from] status::StatusCommandError),

    #[error("token: {0}")]
    Token(#[source] Box<token::TokenCommandError>),

    #[error("toolchain: {0}")]
    Toolchain(#[from] toolchain::ToolchainCommandError),

    #[error("scaffold: {0}")]
    Scaffold(#[from] scaffold::ScaffoldCommandError),

    #[error("replica: {0}")]
    Replica(#[from] replica::ReplicaCommandError),

    #[error("restore: {0}")]
    Restore(#[from] restore::RestoreCommandError),
}

impl From<apps::AppCommandError> for CliError {
    fn from(error: apps::AppCommandError) -> Self {
        Self::Apps(Box::new(error))
    }
}

impl From<admission::AdmissionCommandError> for CliError {
    fn from(error: admission::AdmissionCommandError) -> Self {
        Self::Admission(Box::new(error))
    }
}

impl From<auth::AuthCommandError> for CliError {
    fn from(error: auth::AuthCommandError) -> Self {
        Self::Auth(Box::new(error))
    }
}

impl From<backup::BackupCommandError> for CliError {
    fn from(error: backup::BackupCommandError) -> Self {
        Self::Backup(Box::new(error))
    }
}

impl From<blob_storage::BlobStorageCommandError> for CliError {
    fn from(error: blob_storage::BlobStorageCommandError) -> Self {
        Self::BlobStorage(Box::new(error))
    }
}

impl From<cycles::CyclesCommandError> for CliError {
    fn from(error: cycles::CyclesCommandError) -> Self {
        Self::Cycles(Box::new(error))
    }
}

impl From<fleet::FleetCommandError> for CliError {
    fn from(error: fleet::FleetCommandError) -> Self {
        Self::Fleet(Box::new(error))
    }
}

impl From<inspect::InspectCommandError> for CliError {
    fn from(error: inspect::InspectCommandError) -> Self {
        Self::Inspect(Box::new(error))
    }
}

impl From<medic::MedicCommandError> for CliError {
    fn from(error: medic::MedicCommandError) -> Self {
        Self::Medic(Box::new(error))
    }
}

impl From<token::TokenCommandError> for CliError {
    fn from(error: token::TokenCommandError) -> Self {
        Self::Token(Box::new(error))
    }
}

/// Run the CLI from process arguments.
pub fn run_from_env() -> Result<(), CliError> {
    let argv = std::env::args_os().collect::<Vec<_>>();
    trace_if_enabled(&argv);
    run(argv.into_iter().skip(1))
}

/// Run the CLI from an argument iterator.
pub fn run<I>(args: I) -> Result<(), CliError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if let Some(option) = misplaced_global_option(&args) {
        let error = top_level_command().error(
            ErrorKind::UnknownArgument,
            format!("unexpected argument '{option}' found; put it before the command"),
        );
        return Err(CliError::Clap(error));
    }
    let matches = match parse_matches(top_level_command(), args) {
        Ok(matches) => matches,
        Err(error) if !error.use_stderr() => {
            let _ = error.print();
            return Ok(());
        }
        Err(error) => return Err(CliError::Clap(error)),
    };
    let global_icp = cli::clap::string_option(&matches, "icp");
    let global_environment = cli::clap::string_option(&matches, "environment");

    let (command, subcommand_matches) = matches
        .subcommand()
        .unwrap_or_else(|| unreachable!("Clap requires one top-level command"));
    let mut tail = subcommand_matches
        .get_many::<OsString>(DISPATCH_ARGS)
        .map(|values| values.cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    if let Some(conflict) =
        global_environment_conflict(command, &tail, global_environment.as_deref())
    {
        return Err(CliError::Clap(
            top_level_command().error(ErrorKind::InvalidValue, conflict.to_string()),
        ));
    }
    apply_global_icp(command, &mut tail, global_icp);
    apply_global_environment(command, &mut tail, global_environment);
    let tail = tail.into_iter();

    match command {
        "admission" => admission::run(tail).map_err(CliError::from),
        "app" => apps::run(tail).map_err(CliError::from),
        "auth" => auth::run(tail).map_err(CliError::from),
        "backup" => backup::run(tail).map_err(CliError::from),
        "blob-storage" => blob_storage::run(tail).map_err(CliError::from),
        "build" => build::run(tail).map_err(CliError::from),
        "cycles" => cycles::run(tail).map_err(CliError::from),
        "diagnostic" => diagnostic::run(tail).map_err(CliError::from),
        "evidence" => evidence::run(tail).map_err(CliError::from),
        "fleet" => fleet::run(tail).map_err(CliError::from),
        "info" => info::run(tail).map_err(CliError::from),
        "inspect" => inspect::run(tail).map_err(CliError::from),
        "medic" => medic::run(tail).map_err(CliError::from),
        "network" => network::run(tail).map_err(CliError::from),
        "replica" => replica::run(tail).map_err(CliError::from),
        "restore" => restore::run(tail).map_err(CliError::from),
        "scaffold" => scaffold::run(tail).map_err(CliError::from),
        "state" => state::run(tail).map_err(CliError::from),
        "status" => status::run(tail).map_err(CliError::from),
        "token" => token::run(tail).map_err(CliError::from),
        "toolchain" => toolchain::run(tail).map_err(CliError::from),
        _ => unreachable!("top-level dispatch command only defines known commands"),
    }
}

#[must_use]
pub const fn version_text() -> &'static str {
    VERSION_TEXT
}

#[must_use]
pub fn render_cli_error(error: &CliError) -> String {
    match error {
        CliError::BlobStorage(err) => err.json_error_report().unwrap_or_else(|| error.to_string()),
        CliError::Build(build::BuildCommandError::Clap(err)) | CliError::Clap(err) => {
            err.to_string().trim_end().to_string()
        }
        CliError::Inspect(err) if err.suppress_stderr() => String::new(),
        CliError::Medic(err) if err.suppress_stderr() => String::new(),
        CliError::State(err) if err.suppress_stderr() => String::new(),
        _ => error.to_string(),
    }
}

#[must_use]
pub fn cli_error_exit_code(err: &CliError) -> i32 {
    match err {
        CliError::Auth(err) => i32::from(err.exit_code()),
        CliError::BlobStorage(err) => i32::from(err.exit_code()),
        CliError::Build(err) => err.exit_code(),
        CliError::Clap(err) => err.exit_code(),
        CliError::Info(err) => i32::from(err.exit_code()),
        CliError::Inspect(err) => i32::from(err.exit_code()),
        CliError::Medic(err) => i32::from(err.exit_code()),
        CliError::State(err) => i32::from(err.exit_code()),
        _ => 1,
    }
}

#[cfg(test)]
mod tests;
