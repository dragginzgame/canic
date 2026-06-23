//! Module: canic_cli::blob_storage
//!
//! Responsibility: expose operator CLI commands for blob-storage billing readiness.
//! Does not own: canister runtime billing policy, Cashier calls, or product orchestration.
//! Boundary: parses operator commands and renders host canister-call actions.

mod model;
mod options;
mod parse;
mod render;
mod target;

#[cfg(test)]
mod tests;

use crate::{
    blob_storage::{
        model::{BlobStorageActionName, BlobStorageActionResult, BlobStorageErrorResult},
        options::{BlobStorageCommand, BlobStorageOptions},
        parse::{parse_funding_report, parse_status_result},
        render::{render_action_result, render_dry_run_command, render_status_result},
        target::{BlobStorageMethodMode, resolve_blob_storage_call_target},
    },
    cli::help::print_help_or_version,
    version_text,
};
use canic_core::protocol::{
    BLOB_STORAGE_FUND_FROM_PROJECT_CYCLES, BLOB_STORAGE_STATUS,
    BLOB_STORAGE_UPDATE_GATEWAY_PRINCIPALS,
};
use canic_host::icp::IcpCli;
use canic_host::{
    candid_endpoints::CandidEndpointError, icp::IcpCommandError,
    installed_deployment::InstalledDeploymentError,
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

    #[error("failed to parse blob-storage canister response")]
    ResponseParse,

    #[error("{source}")]
    JsonReported {
        source: Box<Self>,
        report_json: String,
        exit_code: u8,
    },
}

impl BlobStorageCommandError {
    pub fn json_error_report(&self) -> Option<String> {
        let Self::JsonReported { report_json, .. } = self else {
            return None;
        };
        Some(report_json.clone())
    }

    pub const fn exit_code(&self) -> u8 {
        match self {
            Self::JsonReported { exit_code, .. } => *exit_code,
            Self::ReplicaQuery(_) | Self::IcpFailed { .. } => 2,
            Self::ResponseParse => 3,
            Self::Usage(_)
            | Self::Json(_)
            | Self::NoInstalledDeployment { .. }
            | Self::InstallState(_)
            | Self::UnknownTarget { .. }
            | Self::AmbiguousRole { .. }
            | Self::CandidUnavailable { .. }
            | Self::CandidRead { .. }
            | Self::CandidParse { .. }
            | Self::MethodUnavailable { .. } => 1,
        }
    }

    fn with_json_report(self, deployment: &str, target: &str) -> Self {
        let code = self.command_error_code();
        let exit_code = self.exit_code();
        let message = self.to_string();
        let report = BlobStorageErrorResult::new(deployment, target, code, message, exit_code);
        Self::JsonReported {
            source: Box::new(self),
            report_json: serde_json::to_string_pretty(&report)
                .expect("blob-storage error report should serialize"),
            exit_code,
        }
    }

    fn command_error_code(&self) -> &'static str {
        match self {
            Self::Usage(message) if message.contains("--cycles") => "invalid_cycles",
            Self::Usage(_)
            | Self::Json(_)
            | Self::InstallState(_)
            | Self::NoInstalledDeployment { .. }
            | Self::UnknownTarget { .. }
            | Self::AmbiguousRole { .. } => "target_resolution_failed",
            Self::CandidUnavailable { .. } | Self::CandidRead { .. } => "candid_unavailable",
            Self::MethodUnavailable { .. } => "method_unavailable",
            Self::ReplicaQuery(_) | Self::IcpFailed { .. } => "transport_failed",
            Self::ResponseParse => "response_parse_failed",
            Self::CandidParse { .. } => "candid_decode_failed",
            Self::JsonReported { source, .. } => source.command_error_code(),
        }
    }
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
    run_command_with_json_errors(command)
}

///
/// BlobStorageMedicStatus
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BlobStorageMedicStatus {
    Ready,
    Warning,
    Blocked,
}

///
/// BlobStorageMedicSummary
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlobStorageMedicSummary {
    pub status: BlobStorageMedicStatus,
    pub detail: String,
    pub next: String,
}

impl BlobStorageMedicSummary {
    fn from_status(result: &model::BlobStorageStatusResult) -> Self {
        let status = match result.readiness.state.as_str() {
            "ready" => BlobStorageMedicStatus::Ready,
            "warning" => BlobStorageMedicStatus::Warning,
            _ => BlobStorageMedicStatus::Blocked,
        };
        let mut detail = vec![
            format!("readiness={}", result.readiness.state),
            format!("configured={}", result.configured),
            format!("gateways={}", result.gateways.principal_count),
            format!("funding={}", result.funding.status),
        ];
        if let Some(balance) = &result.cashier.balance_cycles {
            detail.push(format!("cashier_balance={balance}"));
        }
        if !result.readiness.blockers.is_empty() {
            detail.push(format!("blockers={}", result.readiness.blockers.join(",")));
        }
        if !result.readiness.warnings.is_empty() {
            detail.push(format!("warnings={}", result.readiness.warnings.join(",")));
        }

        let next = result
            .next
            .iter()
            .find_map(|action| action.command.clone())
            .unwrap_or_else(|| {
                if status == BlobStorageMedicStatus::Ready {
                    "-".to_string()
                } else {
                    format!(
                        "canic blob-storage status {} {}",
                        result.deployment, result.target.input
                    )
                }
            });

        Self {
            status,
            detail: detail.join("; "),
            next,
        }
    }
}

pub fn medic_summary(
    deployment: &str,
    canister: &str,
    network: &str,
    icp: &str,
) -> Result<BlobStorageMedicSummary, BlobStorageCommandError> {
    let options = options::CommonOptions {
        network: network.to_string(),
        icp: icp.to_string(),
    };
    live_status_result(&options, deployment, canister)
        .map(|status| BlobStorageMedicSummary::from_status(&status))
}

fn run_command(command: BlobStorageCommand) -> Result<(), BlobStorageCommandError> {
    match command {
        BlobStorageCommand::Status(options) => run_status(&options),
        BlobStorageCommand::SyncGateways(options) => run_sync_gateways(&options),
        BlobStorageCommand::Fund(options) => run_fund(&options),
    }
}

fn run_command_with_json_errors(
    command: BlobStorageCommand,
) -> Result<(), BlobStorageCommandError> {
    let context = json_error_context(&command);
    run_command(command).map_err(|err| {
        if let Some((deployment, target)) = context {
            err.with_json_report(&deployment, &target)
        } else {
            err
        }
    })
}

fn json_error_context(command: &BlobStorageCommand) -> Option<(String, String)> {
    match command {
        BlobStorageCommand::Status(options) if options.json => {
            Some((options.deployment.clone(), options.canister.clone()))
        }
        BlobStorageCommand::SyncGateways(options) if options.json => {
            Some((options.deployment.clone(), options.canister.clone()))
        }
        BlobStorageCommand::Fund(options) if options.json => {
            Some((options.deployment.clone(), options.canister.clone()))
        }
        _ => None,
    }
}

fn run_status(options: &options::StatusOptions) -> Result<(), BlobStorageCommandError> {
    let result = live_status_result(&options.common, &options.deployment, &options.canister)?;
    write_status_result(options.json, &result)
}

fn live_status_result(
    options: &options::CommonOptions,
    deployment: &str,
    canister: &str,
) -> Result<model::BlobStorageStatusResult, BlobStorageCommandError> {
    let target =
        resolve_blob_storage_call_target(options, deployment, canister, BLOB_STORAGE_STATUS)?;
    let output = live_call_output(
        options,
        &target,
        BLOB_STORAGE_STATUS,
        "(record { sync_gateway_principals = false })",
        Some("json"),
    )?;
    parse_status_result(deployment, target.target, &output)
        .ok_or(BlobStorageCommandError::ResponseParse)
}

fn run_sync_gateways(
    options: &options::SyncGatewaysOptions,
) -> Result<(), BlobStorageCommandError> {
    let target = resolve_blob_storage_call_target(
        &options.common,
        &options.deployment,
        &options.canister,
        BLOB_STORAGE_UPDATE_GATEWAY_PRINCIPALS,
    )?;
    let output = options.json.then_some("json");
    let command = dry_run_call_display(
        &options.common,
        &target,
        BLOB_STORAGE_UPDATE_GATEWAY_PRINCIPALS,
        "()",
        output,
    );
    let result = if options.dry_run {
        BlobStorageActionResult::dry_run(
            &options.deployment,
            BlobStorageActionName::SyncGateways,
            target.target,
            BLOB_STORAGE_UPDATE_GATEWAY_PRINCIPALS,
            target.method_mode.label(),
            command,
            None,
        )
    } else {
        let _output = live_call_output(
            &options.common,
            &target,
            BLOB_STORAGE_UPDATE_GATEWAY_PRINCIPALS,
            "()",
            output,
        )?;
        let result = BlobStorageActionResult::completed(
            &options.deployment,
            BlobStorageActionName::SyncGateways,
            target.target,
            BLOB_STORAGE_UPDATE_GATEWAY_PRINCIPALS,
            target.method_mode.label(),
            command,
            None,
        );
        match live_status_result(&options.common, &options.deployment, &options.canister) {
            Ok(status) => result.with_post_status(status),
            Err(_) => result.with_warning("post_status_unavailable"),
        }
    };
    write_action_result(options.json, &result)
}

fn run_fund(options: &options::FundOptions) -> Result<(), BlobStorageCommandError> {
    let target = resolve_blob_storage_call_target(
        &options.common,
        &options.deployment,
        &options.canister,
        BLOB_STORAGE_FUND_FROM_PROJECT_CYCLES,
    )?;
    let arg = format!("({} : nat)", options.cycles);
    let output = if options.dry_run {
        options.json.then_some("json")
    } else {
        Some("json")
    };
    let command = dry_run_call_display(
        &options.common,
        &target,
        BLOB_STORAGE_FUND_FROM_PROJECT_CYCLES,
        &arg,
        output,
    );
    let result = if options.dry_run {
        BlobStorageActionResult::dry_run(
            &options.deployment,
            BlobStorageActionName::Fund,
            target.target,
            BLOB_STORAGE_FUND_FROM_PROJECT_CYCLES,
            target.method_mode.label(),
            command,
            Some(options.cycles),
        )
    } else {
        let call_output = live_call_output(
            &options.common,
            &target,
            BLOB_STORAGE_FUND_FROM_PROJECT_CYCLES,
            &arg,
            output,
        )?;
        let report =
            parse_funding_report(&call_output).ok_or(BlobStorageCommandError::ResponseParse)?;
        let result = BlobStorageActionResult::completed(
            &options.deployment,
            BlobStorageActionName::Fund,
            target.target,
            BLOB_STORAGE_FUND_FROM_PROJECT_CYCLES,
            target.method_mode.label(),
            command,
            Some(options.cycles),
        )
        .with_funding_report(report);
        match live_status_result(&options.common, &options.deployment, &options.canister) {
            Ok(status) => result.with_post_status(status),
            Err(_) => result.with_warning("post_status_unavailable"),
        }
    };
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

fn live_call_output(
    options: &options::CommonOptions,
    target: &target::BlobStorageCallTarget,
    method: &str,
    arg: &str,
    output: Option<&str>,
) -> Result<String, BlobStorageCommandError> {
    let icp = icp_cli(options).with_cwd(&target.icp_root);
    let result = match target.method_mode {
        BlobStorageMethodMode::Query => icp.canister_query_arg_output_with_candid(
            &target.target.canister_id,
            method,
            arg,
            output,
            Some(target.candid_path.as_path()),
        ),
        BlobStorageMethodMode::Update => icp.canister_call_arg_output_with_candid(
            &target.target.canister_id,
            method,
            arg,
            output,
            Some(target.candid_path.as_path()),
        ),
    };
    result.map_err(blob_storage_icp_error)
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

fn blob_storage_icp_error(error: IcpCommandError) -> BlobStorageCommandError {
    match error {
        IcpCommandError::Io(err) => BlobStorageCommandError::InstallState(err.to_string()),
        IcpCommandError::Failed { command, stderr }
        | IcpCommandError::Json {
            command,
            output: stderr,
            ..
        } => BlobStorageCommandError::IcpFailed { command, stderr },
        IcpCommandError::SnapshotIdUnavailable { output } => BlobStorageCommandError::IcpFailed {
            command: "icp canister call".to_string(),
            stderr: output,
        },
        error @ (IcpCommandError::MissingCli { .. }
        | IcpCommandError::IncompatibleCliVersion { .. }) => BlobStorageCommandError::IcpFailed {
            command: "icp --version".to_string(),
            stderr: error.to_string(),
        },
    }
}

fn write_status_result(
    json: bool,
    result: &model::BlobStorageStatusResult,
) -> Result<(), BlobStorageCommandError> {
    if json {
        println!("{}", serde_json::to_string_pretty(result)?);
    } else {
        println!("{}", render_status_result(result));
    }
    Ok(())
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
