mod convert;
mod model;
mod options;
mod parse;
mod render;
#[cfg(test)]
mod tests;
mod transport;
mod wallet;

use crate::{
    cli::{clap::parse_required_subcommand, help::print_help_or_version},
    cycles::{
        options::{CyclesOptions, info_usage},
        render::write_cycles_report,
        transport::cycles_report,
        wallet::{cycles_command, cycles_usage},
    },
    version_text,
};
use canic_backup::discovery::DiscoveryError;
use canic_core::{cdk::utils::hash::DecodeHexError, dto::error::ErrorCode};
use canic_host::{
    icp::IcpCommandError, icp_config::IcpConfigError, installed_fleet::InstalledFleetError,
    registry::RegistryParseError,
};
use std::ffi::OsString;
use thiserror::Error as ThisError;

///
/// CyclesCommandError
///

#[derive(Debug, ThisError)]
pub enum CyclesCommandError {
    #[error("{0}")]
    Usage(String),

    #[error("failed to read Canic Fleet state: {0}")]
    IcpRoot(#[source] IcpConfigError),

    #[error(transparent)]
    InstalledFleet(#[from] InstalledFleetError),

    #[error(transparent)]
    Icp(#[from] IcpCommandError),

    #[error("recipient must be a principal or <fleet>/<role-or-canister>")]
    InvalidRecipient,

    #[error("Fleet {fleet} has no canister or role named {target}")]
    UnknownTarget { fleet: String, target: String },

    #[error("role {role} is ambiguous in Fleet {fleet}; use one canister principal")]
    AmbiguousRole { fleet: String, role: String },

    #[error("failed to update pending operation log: {0}")]
    PendingOperationLog(String),

    #[error("failed to decode ICP refill response Candid: {0}")]
    IcpRefillResponseCandid(#[source] candid::Error),

    #[error("failed to decode ICP refill response hex: {0}")]
    IcpRefillResponseHex(#[source] DecodeHexError),

    #[error("ICP refill response operation id mismatch: expected {expected}, got {actual}")]
    IcpRefillOperationIdMismatch { expected: String, actual: String },

    #[error("ICP refill request rejected: [{code:?}] {message}")]
    IcpRefillRejected { code: ErrorCode, message: String },

    #[error("live ICP refill returned a dry-run response")]
    IcpRefillUnexpectedResponse,

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

    let (command, args) = parse_required_subcommand(cycles_command(), args)
        .map_err(|_| CyclesCommandError::Usage(cycles_usage()))?;

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
