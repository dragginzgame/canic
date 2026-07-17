mod model;
mod options;
mod parse;
mod render;
mod transport;

use crate::{
    cli::help::print_help_or_version,
    metrics::{
        options::{MetricsOptions, info_usage},
        render::write_metrics_report,
        transport::metrics_report,
    },
    version_text,
};
use canic_backup::discovery::DiscoveryError;
use canic_host::{
    icp::IcpCommandError, icp_config::IcpConfigError,
    installed_deployment::InstalledDeploymentError, registry::RegistryParseError,
};
use std::ffi::OsString;
use thiserror::Error as ThisError;

const CANIC_METRICS_METHOD: &str = "canic_metrics";

///
/// MetricsCommandError
///

#[derive(Debug, ThisError)]
pub enum MetricsCommandError {
    #[error("{0}")]
    Usage(String),

    #[error("failed to read canic deployment state: {0}")]
    IcpRoot(#[source] IcpConfigError),

    #[error(transparent)]
    InstalledDeployment(#[from] InstalledDeploymentError),

    #[error(transparent)]
    Icp(#[from] IcpCommandError),

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

pub fn run_info<I>(args: I) -> Result<(), MetricsCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, info_usage, version_text()) {
        return Ok(());
    }

    let options = MetricsOptions::parse_info(args)?;
    run_options(&options)
}

fn run_options(options: &MetricsOptions) -> Result<(), MetricsCommandError> {
    let report = metrics_report(options)?;
    write_metrics_report(options, &report)
}

#[cfg(test)]
mod tests;
