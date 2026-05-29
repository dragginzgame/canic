mod model;
mod options;
mod parse;
mod render;
mod transport;
mod wallet;

use crate::{
    cli::{clap::parse_subcommand, help::print_help_or_version},
    cycles::{
        options::{CyclesOptions, info_usage},
        render::write_cycles_report,
        transport::cycles_report,
        wallet::{cycles_command, cycles_usage},
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
        "deployment target {deployment} is not installed on network {network}; run `canic install <fleet-template>` or `canic deploy register {deployment} --fleet-template <fleet-template> --root <principal> --allow-unverified` before using cycles commands"
    )]
    NoInstalledDeployment { network: String, deployment: String },

    #[error("failed to read canic deployment state: {0}")]
    InstallState(String),

    #[error("local replica query failed: {0}")]
    ReplicaQuery(String),

    #[error("icp command failed: {command}\n{stderr}")]
    IcpFailed { command: String, stderr: String },

    #[error("invalid duration {0}; use values like 1h, 6h, 24h, 7d, or 30m")]
    InvalidDuration(String),

    #[error("recipient must be either a positional receiver or --to-deployment")]
    InvalidRecipient,

    #[error("--to-role requires --to-deployment")]
    RoleWithoutDeployment,

    #[error("deployment target {deployment} has no canister or role named {target}")]
    UnknownTarget { deployment: String, target: String },

    #[error(
        "role {role} is ambiguous in deployment target {deployment}; use one canister principal"
    )]
    AmbiguousRole { deployment: String, role: String },

    #[error(transparent)]
    RegistryTree(#[from] crate::support::registry_tree::RegistryTreeError),

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
    if print_help_or_version(&args, cycles_usage, version_text()) {
        return Ok(());
    }

    let Some((command, args)) = parse_subcommand(cycles_command(), args)
        .map_err(|_| CyclesCommandError::Usage(cycles_usage()))?
    else {
        return Err(CyclesCommandError::Usage(cycles_usage()));
    };

    wallet::run_cycles_command(&command, args)
}

pub fn run_info<I>(args: I) -> Result<(), CyclesCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, info_usage, version_text()) {
        return Ok(());
    }

    let options = CyclesOptions::parse_info(args)?;
    run_options(&options)
}

fn run_options(options: &CyclesOptions) -> Result<(), CyclesCommandError> {
    let report = cycles_report(options)?;
    write_cycles_report(options, &report)
}

#[cfg(test)]
mod tests;
