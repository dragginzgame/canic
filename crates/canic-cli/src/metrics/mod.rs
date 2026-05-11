mod model;
mod options;
mod parse;
mod render;
mod transport;

use crate::{
    cli::help::print_help_or_version,
    metrics::{
        options::{MetricsOptions, usage},
        render::write_metrics_report,
        transport::metrics_report,
    },
    version_text,
};
use canic_backup::discovery::DiscoveryError;
use canic_host::registry::RegistryParseError;
use std::ffi::OsString;
use thiserror::Error as ThisError;

pub const CANIC_METRICS_METHOD: &str = "canic_metrics";

///
/// MetricsCommandError
///

#[derive(Debug, ThisError)]
pub enum MetricsCommandError {
    #[error("{0}")]
    Usage(String),

    #[error(
        "fleet {fleet} is not installed on network {network}; run `canic install {fleet}` before querying metrics"
    )]
    NoInstalledFleet { network: String, fleet: String },

    #[error("failed to read canic fleet state: {0}")]
    InstallState(String),

    #[error("local replica query failed: {0}")]
    ReplicaQuery(String),

    #[error("icp command failed: {command}\n{stderr}")]
    IcpFailed { command: String, stderr: String },

    #[error(
        "invalid metrics kind {0}; use core, placement, platform, runtime, security, or storage"
    )]
    InvalidKind(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Discovery(#[from] DiscoveryError),

    #[error(transparent)]
    Registry(#[from] RegistryParseError),
}

pub fn run<I>(args: I) -> Result<(), MetricsCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    let options = MetricsOptions::parse(args)?;
    let report = metrics_report(&options)?;
    write_metrics_report(&options, &report)
}

#[cfg(test)]
mod tests;
