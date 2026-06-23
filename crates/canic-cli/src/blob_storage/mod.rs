//! Module: canic_cli::blob_storage
//!
//! Responsibility: expose operator CLI commands for blob-storage billing readiness.
//! Does not own: canister runtime billing policy, Cashier calls, or product orchestration.
//! Boundary: parses operator commands and renders host canister-call actions.

mod model;
mod options;
mod render;
mod target;

#[cfg(test)]
mod tests;

use crate::{
    blob_storage::{
        model::{BlobStorageActionName, BlobStorageActionResult},
        options::{BlobStorageCommand, BlobStorageOptions},
        render::{render_action_result, render_dry_run_command},
        target::{BlobStorageMethodMode, resolve_blob_storage_call_target},
    },
    cli::help::print_help_or_version,
    version_text,
};
use canic_core::protocol::{
    BLOB_STORAGE_FUND_FROM_PROJECT_CYCLES, BLOB_STORAGE_UPDATE_GATEWAY_PRINCIPALS,
};
use canic_host::icp::IcpCli;
use canic_host::{
    candid_endpoints::CandidEndpointError, installed_deployment::InstalledDeploymentError,
};
use std::{ffi::OsString, io, path::PathBuf};
use thiserror::Error as ThisError;

///
/// BlobStorageCommandError
///

#[derive(Debug, ThisError)]
pub enum BlobStorageCommandError {
    #[error("{0}")]
    Usage(String),

    #[error("failed to render JSON output: {0}")]
    Json(#[from] serde_json::Error),

    #[error(
        "deployment target {deployment} is not installed on network {network}; install or register it before using blob-storage commands"
    )]
    NoInstalledDeployment { network: String, deployment: String },

    #[error("failed to read canic deployment state: {0}")]
    InstallState(String),

    #[error("local replica query failed: {0}")]
    ReplicaQuery(String),

    #[error("icp command failed: {command}\n{stderr}")]
    IcpFailed { command: String, stderr: String },

    #[error("deployment target {deployment} has no canister or role named {target}")]
    UnknownTarget { deployment: String, target: String },

    #[error(
        "role {role} is ambiguous in deployment target {deployment}; use one canister principal"
    )]
    AmbiguousRole { deployment: String, role: String },

    #[error(
        "blob-storage target {target} in deployment {deployment} has no local Candid sidecar; use a role/registered canister with local metadata"
    )]
    CandidUnavailable { deployment: String, target: String },

    #[error("failed to read local Candid sidecar {path}: {source}")]
    CandidRead { path: PathBuf, source: io::Error },

    #[error("failed to parse local Candid sidecar {path}: {source}")]
    CandidParse {
        path: PathBuf,
        source: CandidEndpointError,
    },

    #[error("local Candid sidecar {path} does not define blob-storage method {method}")]
    MethodUnavailable { path: PathBuf, method: String },
}

/// Run the blob-storage operator command group.
pub fn run<I>(args: I) -> Result<(), BlobStorageCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, options::usage, version_text()) {
        return Ok(());
    }

    let command = BlobStorageOptions::parse(args)?;
    run_command(command)
}

fn run_command(command: BlobStorageCommand) -> Result<(), BlobStorageCommandError> {
    match command {
        BlobStorageCommand::Status(_) => Err(BlobStorageCommandError::Usage(format!(
            "blob-storage status live transport is not implemented yet\n\n{}",
            options::status_usage_with_bin_name()
        ))),
        BlobStorageCommand::SyncGateways(options) => run_sync_gateways(&options),
        BlobStorageCommand::Fund(options) => run_fund(&options),
    }
}

fn run_sync_gateways(
    options: &options::SyncGatewaysOptions,
) -> Result<(), BlobStorageCommandError> {
    if !options.dry_run {
        return Err(BlobStorageCommandError::Usage(format!(
            "blob-storage sync-gateways requires --dry-run in this implementation slice\n\n{}",
            options::sync_gateways_usage_with_bin_name()
        )));
    }

    let target = resolve_blob_storage_call_target(
        &options.common,
        &options.deployment,
        &options.canister,
        BLOB_STORAGE_UPDATE_GATEWAY_PRINCIPALS,
    )?;
    let command = dry_run_call_display(
        &options.common,
        &target,
        BLOB_STORAGE_UPDATE_GATEWAY_PRINCIPALS,
        "()",
        options.json.then_some("json"),
    );
    let result = BlobStorageActionResult::dry_run(
        &options.deployment,
        BlobStorageActionName::SyncGateways,
        target.target,
        BLOB_STORAGE_UPDATE_GATEWAY_PRINCIPALS,
        target.method_mode.label(),
        command,
        None,
    );
    write_action_result(options.json, &result)
}

fn run_fund(options: &options::FundOptions) -> Result<(), BlobStorageCommandError> {
    if !options.dry_run {
        return Err(BlobStorageCommandError::Usage(format!(
            "blob-storage fund requires --dry-run in this implementation slice\n\n{}",
            options::fund_usage_with_bin_name()
        )));
    }

    let target = resolve_blob_storage_call_target(
        &options.common,
        &options.deployment,
        &options.canister,
        BLOB_STORAGE_FUND_FROM_PROJECT_CYCLES,
    )?;
    let arg = format!("({} : nat)", options.cycles);
    let command = dry_run_call_display(
        &options.common,
        &target,
        BLOB_STORAGE_FUND_FROM_PROJECT_CYCLES,
        &arg,
        options.json.then_some("json"),
    );
    let result = BlobStorageActionResult::dry_run(
        &options.deployment,
        BlobStorageActionName::Fund,
        target.target,
        BLOB_STORAGE_FUND_FROM_PROJECT_CYCLES,
        target.method_mode.label(),
        command,
        Some(options.cycles),
    );
    write_action_result(options.json, &result)
}

fn icp_cli(options: &options::CommonOptions) -> IcpCli {
    IcpCli::new(&options.icp, None, Some(options.network.clone()))
}

fn dry_run_call_display(
    options: &options::CommonOptions,
    target: &target::BlobStorageCallTarget,
    method: &str,
    arg: &str,
    output: Option<&str>,
) -> String {
    let icp = icp_cli(options).with_cwd(&target.icp_root);
    match target.method_mode {
        BlobStorageMethodMode::Query => icp.canister_query_arg_output_display_with_candid(
            &target.target.canister_id,
            method,
            arg,
            output,
            Some(target.candid_path.as_path()),
        ),
        BlobStorageMethodMode::Update => icp.canister_call_arg_output_display_with_candid(
            &target.target.canister_id,
            method,
            arg,
            output,
            Some(target.candid_path.as_path()),
        ),
    }
}

fn blob_storage_installed_deployment_error(
    error: InstalledDeploymentError,
) -> BlobStorageCommandError {
    match error {
        InstalledDeploymentError::NoInstalledDeployment {
            network,
            deployment,
        } => BlobStorageCommandError::NoInstalledDeployment {
            network,
            deployment,
        },
        InstalledDeploymentError::InstallState(error) => {
            BlobStorageCommandError::InstallState(error)
        }
        InstalledDeploymentError::ReplicaQuery(error) => {
            BlobStorageCommandError::ReplicaQuery(error)
        }
        InstalledDeploymentError::IcpFailed { command, stderr } => {
            BlobStorageCommandError::IcpFailed { command, stderr }
        }
        InstalledDeploymentError::LostLocalDeployment { root, .. } => {
            BlobStorageCommandError::ReplicaQuery(format!("root canister {root} is not present"))
        }
        InstalledDeploymentError::Registry(error) => {
            BlobStorageCommandError::InstallState(error.to_string())
        }
        InstalledDeploymentError::Io(error) => {
            BlobStorageCommandError::InstallState(error.to_string())
        }
    }
}

fn write_action_result(
    json: bool,
    result: &BlobStorageActionResult,
) -> Result<(), BlobStorageCommandError> {
    if json {
        println!("{}", serde_json::to_string_pretty(result)?);
    } else {
        println!("{}", render_action_result(result));
        println!();
        println!("{}", render_dry_run_command(result));
    }
    Ok(())
}
