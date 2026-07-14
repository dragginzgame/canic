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
    icp::IcpCommandError, icp_config::IcpConfigError, install_root::InstallStateError,
    installed_deployment::InstalledDeploymentError, registry::RegistryParseError,
    replica_query::ReplicaQueryError,
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

    #[error(
        "deployment target {deployment} is not installed on network {network}; run `canic install <fleet-template>` or `canic deploy register {deployment} --fleet-template <fleet-template> --root <principal> --allow-unverified` before using cycles commands"
    )]
    NoInstalledDeployment { network: String, deployment: String },

    #[error("failed to read canic deployment state: {0}")]
    InstallState(#[source] InstallStateError),

    #[error("local replica query failed: {0}")]
    ReplicaQuery(#[source] ReplicaQueryError),

    #[error("failed to read canic deployment state: {0}")]
    IcpRoot(#[source] IcpConfigError),

    #[error("local replica query failed: root canister {root} is not present")]
    LostLocalRoot { root: String },

    #[error(transparent)]
    Icp(#[from] IcpCommandError),

    #[error("invalid duration {0}; use values like 1h, 6h, 24h, 7d, or 30m")]
    InvalidDuration(String),

    #[error("recipient must be a principal or <deployment>/<role-or-canister>")]
    InvalidRecipient,

    #[error("deployment target {deployment} has no canister or role named {target}")]
    UnknownTarget { deployment: String, target: String },

    #[error(
        "role {role} is ambiguous in deployment target {deployment}; use one canister principal"
    )]
    AmbiguousRole { deployment: String, role: String },

    #[error("canic cycles convert --fabricate only supports the local network, got {network}")]
    FabricationRequiresLocal { network: String },

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

fn cycles_installed_deployment_error(error: InstalledDeploymentError) -> CyclesCommandError {
    match error {
        InstalledDeploymentError::NoInstalledDeployment {
            network,
            deployment,
        } => CyclesCommandError::NoInstalledDeployment {
            network,
            deployment,
        },
        InstalledDeploymentError::InstallState(error) => CyclesCommandError::InstallState(error),
        InstalledDeploymentError::ReplicaQuery(error) => CyclesCommandError::ReplicaQuery(error),
        InstalledDeploymentError::Icp(error) => CyclesCommandError::Icp(error),
        InstalledDeploymentError::LostLocalDeployment { root, .. } => {
            CyclesCommandError::LostLocalRoot { root }
        }
        InstalledDeploymentError::Registry(error) => CyclesCommandError::Registry(error),
        InstalledDeploymentError::Io(error) => CyclesCommandError::Io(error),
    }
}
