mod model;
mod options;
mod parse;
mod render;
mod transport;

use crate::{
    cli::help::print_help_or_version,
    cycles::{
        options::{CyclesOptions, usage},
        render::write_cycles_report,
        transport::cycles_report,
    },
    version_text,
};
use canic_backup::discovery::DiscoveryError;
use canic_host::registry::RegistryParseError;
use std::ffi::OsString;
use thiserror::Error as ThisError;

///
/// CyclesCommandError
///

#[derive(Debug, ThisError)]
pub enum CyclesCommandError {
    #[error("{0}")]
    Usage(String),

    #[error(
        "fleet {fleet} is not installed on network {network}; run `canic install {fleet}` before querying cycles"
    )]
    NoInstalledFleet { network: String, fleet: String },

    #[error("failed to read canic fleet state: {0}")]
    InstallState(String),

    #[error("local replica query failed: {0}")]
    ReplicaQuery(String),

    #[error("icp command failed: {command}\n{stderr}")]
    IcpFailed { command: String, stderr: String },

    #[error("invalid duration {0}; use values like 1h, 6h, 24h, 7d, or 30m")]
    InvalidDuration(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Discovery(#[from] DiscoveryError),

    #[error(transparent)]
    Registry(#[from] RegistryParseError),
}

pub fn run<I>(args: I) -> Result<(), CyclesCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    let options = CyclesOptions::parse(args)?;
    let report = cycles_report(&options)?;
    write_cycles_report(&options, &report)
}

#[cfg(test)]
mod tests;
