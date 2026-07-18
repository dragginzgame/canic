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
        model::{
            BLOB_STORAGE_ERROR_CODE_CANDID_DECODE_FAILED,
            BLOB_STORAGE_ERROR_CODE_CANDID_UNAVAILABLE, BLOB_STORAGE_ERROR_CODE_INVALID_CYCLES,
            BLOB_STORAGE_ERROR_CODE_METHOD_UNAVAILABLE,
            BLOB_STORAGE_ERROR_CODE_READINESS_CHECK_FAILED,
            BLOB_STORAGE_ERROR_CODE_RESPONSE_PARSE_FAILED,
            BLOB_STORAGE_ERROR_CODE_TARGET_RESOLUTION_FAILED,
            BLOB_STORAGE_ERROR_CODE_TRANSPORT_FAILED, BLOB_STORAGE_WARNING_POST_STATUS_UNAVAILABLE,
            BlobStorageActionName, BlobStorageActionResult, BlobStorageErrorResult,
            BlobStorageMethodMode, BlobStorageReadinessState,
        },
        options::{BlobStorageCommand, BlobStorageOptions},
        parse::{BlobStorageParseError, parse_funding_report, parse_status_result},
        render::{render_action_result, render_dry_run_command, render_status_result},
        target::resolve_blob_storage_call_target,
    },
    cli::help::print_help_or_version,
    version_text,
};
use canic_core::protocol::{
    BLOB_STORAGE_FUND_FROM_PROJECT_CYCLES, BLOB_STORAGE_STATUS,
    BLOB_STORAGE_UPDATE_GATEWAY_PRINCIPALS,
};
use canic_host::icp::{IcpCli, IcpJsonResponseError};
use canic_host::{
    candid_endpoints::CandidEndpointError, icp::IcpCommandError, icp_config::IcpConfigError,
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
    InvalidCycles(String),

    #[error("{0}")]
    Usage(String),

    #[error("failed to render JSON output: {0}")]
    Json(#[from] serde_json::Error),

    #[error("failed to read canic deployment state: {0}")]
    IcpRoot(#[source] IcpConfigError),

    #[error(transparent)]
    InstalledDeployment(#[from] InstalledDeploymentError),

    #[error(transparent)]
    Icp(#[from] IcpCommandError),

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

    #[error("failed to decode blob-storage canister response: {0}")]
    Response(#[source] IcpJsonResponseError),

    #[error("blob-storage {response_kind} response field `{field}` exceeds u128")]
    ResponseValueOutOfRange {
        response_kind: &'static str,
        field: &'static str,
    },

    #[error("{message}")]
    ReadinessCheckFailed {
        message: String,
        state: String,
        blockers: Vec<String>,
        warnings: Vec<String>,
    },

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
            Self::Icp(IcpCommandError::Io(_))
            | Self::Usage(_)
            | Self::InvalidCycles(_)
            | Self::Json(_)
            | Self::InstalledDeployment(
                InstalledDeploymentError::Icp(IcpCommandError::Io(_))
                | InstalledDeploymentError::NoInstalledDeployment { .. }
                | InstalledDeploymentError::InstallState(_)
                | InstalledDeploymentError::Registry(_)
                | InstalledDeploymentError::Io(_),
            )
            | Self::IcpRoot(_)
            | Self::UnknownTarget { .. }
            | Self::AmbiguousRole { .. }
            | Self::CandidUnavailable { .. }
            | Self::CandidRead { .. }
            | Self::CandidParse { .. }
            | Self::MethodUnavailable { .. } => 1,
            Self::InstalledDeployment(
                InstalledDeploymentError::ReplicaQuery(_)
                | InstalledDeploymentError::LostLocalDeployment { .. }
                | InstalledDeploymentError::Icp(_),
            )
            | Self::Icp(_) => 2,
            Self::Response(_) | Self::ResponseValueOutOfRange { .. } => 3,
            Self::ReadinessCheckFailed { .. } => 4,
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
            Self::InvalidCycles(_) => BLOB_STORAGE_ERROR_CODE_INVALID_CYCLES,
            Self::Usage(_)
            | Self::Json(_)
            | Self::IcpRoot(_)
            | Self::InstalledDeployment(
                InstalledDeploymentError::NoInstalledDeployment { .. }
                | InstalledDeploymentError::InstallState(_)
                | InstalledDeploymentError::Registry(_)
                | InstalledDeploymentError::Io(_)
                | InstalledDeploymentError::Icp(IcpCommandError::Io(_)),
            )
            | Self::UnknownTarget { .. }
            | Self::AmbiguousRole { .. }
            | Self::Icp(IcpCommandError::Io(_)) => BLOB_STORAGE_ERROR_CODE_TARGET_RESOLUTION_FAILED,
            Self::CandidUnavailable { .. } | Self::CandidRead { .. } => {
                BLOB_STORAGE_ERROR_CODE_CANDID_UNAVAILABLE
            }
            Self::MethodUnavailable { .. } => BLOB_STORAGE_ERROR_CODE_METHOD_UNAVAILABLE,
            Self::InstalledDeployment(
                InstalledDeploymentError::ReplicaQuery(_)
                | InstalledDeploymentError::LostLocalDeployment { .. }
                | InstalledDeploymentError::Icp(_),
            )
            | Self::Icp(_) => BLOB_STORAGE_ERROR_CODE_TRANSPORT_FAILED,
            Self::Response(_) | Self::ResponseValueOutOfRange { .. } => {
                BLOB_STORAGE_ERROR_CODE_RESPONSE_PARSE_FAILED
            }
            Self::CandidParse { .. } => BLOB_STORAGE_ERROR_CODE_CANDID_DECODE_FAILED,
            Self::ReadinessCheckFailed { .. } => BLOB_STORAGE_ERROR_CODE_READINESS_CHECK_FAILED,
            Self::JsonReported { source, .. } => source.command_error_code(),
        }
    }
}

impl From<BlobStorageParseError> for BlobStorageCommandError {
    fn from(error: BlobStorageParseError) -> Self {
        match error {
            BlobStorageParseError::NatOutOfRange { kind, field } => Self::ResponseValueOutOfRange {
                response_kind: kind.label(),
                field,
            },
            BlobStorageParseError::Response(error) => Self::Response(error),
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
        let status = match result.readiness.state {
            BlobStorageReadinessState::Ready => BlobStorageMedicStatus::Ready,
            BlobStorageReadinessState::Warning => BlobStorageMedicStatus::Warning,
            BlobStorageReadinessState::Blocked => BlobStorageMedicStatus::Blocked,
        };
        let mut detail = vec![
            format!("readiness={}", result.readiness.state.label()),
            format!("configured={}", result.configured),
            format!("gateways={}", result.gateways.principal_count),
            format!("funding={}", result.funding.status.label()),
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
    environment: &str,
    icp: &str,
) -> Result<BlobStorageMedicSummary, BlobStorageCommandError> {
    let options = options::CommonOptions {
        environment: environment.to_string(),
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

trait BlobStorageRuntime {
    fn resolve_call_target(
        &self,
        options: &options::CommonOptions,
        deployment: &str,
        canister: &str,
        method: &str,
    ) -> Result<target::BlobStorageCallTarget, BlobStorageCommandError>;

    fn call_output(
        &self,
        options: &options::CommonOptions,
        target: &target::BlobStorageCallTarget,
        method: &str,
        arg: &str,
        output: Option<&str>,
    ) -> Result<String, BlobStorageCommandError>;
}

struct LiveBlobStorageRuntime;

impl BlobStorageRuntime for LiveBlobStorageRuntime {
    fn resolve_call_target(
        &self,
        options: &options::CommonOptions,
        deployment: &str,
        canister: &str,
        method: &str,
    ) -> Result<target::BlobStorageCallTarget, BlobStorageCommandError> {
        resolve_blob_storage_call_target(options, deployment, canister, method)
    }

    fn call_output(
        &self,
        options: &options::CommonOptions,
        target: &target::BlobStorageCallTarget,
        method: &str,
        arg: &str,
        output: Option<&str>,
    ) -> Result<String, BlobStorageCommandError> {
        live_call_output(options, target, method, arg, output)
    }
}

fn run_command_with_json_errors(
    command: BlobStorageCommand,
) -> Result<(), BlobStorageCommandError> {
    let context = json_error_context(&command);
    match run_command(command) {
        Ok(()) => Ok(()),
        Err(err @ BlobStorageCommandError::ReadinessCheckFailed { .. }) => Err(err),
        Err(err) => {
            if let Some((deployment, target)) = context {
                Err(err.with_json_report(&deployment, &target))
            } else {
                Err(err)
            }
        }
    }
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
    write_status_result(options.json, &result)?;
    if options.check_ready {
        check_status_ready_for_upload(&result)?;
    }
    Ok(())
}

fn live_status_result(
    options: &options::CommonOptions,
    deployment: &str,
    canister: &str,
) -> Result<model::BlobStorageStatusResult, BlobStorageCommandError> {
    let runtime = LiveBlobStorageRuntime;
    status_result_with_runtime(&runtime, options, deployment, canister)
}

fn status_result_with_runtime(
    runtime: &impl BlobStorageRuntime,
    options: &options::CommonOptions,
    deployment: &str,
    canister: &str,
) -> Result<model::BlobStorageStatusResult, BlobStorageCommandError> {
    let target = runtime.resolve_call_target(options, deployment, canister, BLOB_STORAGE_STATUS)?;
    let output = runtime.call_output(
        options,
        &target,
        BLOB_STORAGE_STATUS,
        "(record { sync_gateway_principals = false })",
        Some("json"),
    )?;
    parse_status_result(deployment, target.target, &output).map_err(Into::into)
}

fn check_status_ready_for_upload(
    result: &model::BlobStorageStatusResult,
) -> Result<(), BlobStorageCommandError> {
    if result.readiness.ready_for_upload {
        return Ok(());
    }
    Err(BlobStorageCommandError::ReadinessCheckFailed {
        message: readiness_check_failure_message(result),
        state: result.readiness.state.label().to_string(),
        blockers: result.readiness.blockers.clone(),
        warnings: result.readiness.warnings.clone(),
    })
}

fn readiness_check_failure_message(result: &model::BlobStorageStatusResult) -> String {
    let mut parts = vec![format!(
        "readiness check failed: state={}",
        result.readiness.state.label()
    )];
    if !result.readiness.blockers.is_empty() {
        parts.push(format!("blockers={}", result.readiness.blockers.join(",")));
    }
    if !result.readiness.warnings.is_empty() {
        parts.push(format!("warnings={}", result.readiness.warnings.join(",")));
    }
    parts.join("; ")
}

fn run_sync_gateways(
    options: &options::SyncGatewaysOptions,
) -> Result<(), BlobStorageCommandError> {
    let runtime = LiveBlobStorageRuntime;
    let result = sync_gateways_result_with_runtime(&runtime, options)?;
    write_action_result(options.json, &result)
}

fn sync_gateways_result_with_runtime(
    runtime: &impl BlobStorageRuntime,
    options: &options::SyncGatewaysOptions,
) -> Result<BlobStorageActionResult, BlobStorageCommandError> {
    let target = runtime.resolve_call_target(
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
            target.method_mode,
            command,
            None,
        )
    } else {
        let _output = runtime.call_output(
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
            target.method_mode,
            command,
            None,
        );
        with_post_status_diagnostic(
            runtime,
            &options.common,
            &options.deployment,
            &options.canister,
            result,
        )
    };
    Ok(result)
}

fn run_fund(options: &options::FundOptions) -> Result<(), BlobStorageCommandError> {
    let runtime = LiveBlobStorageRuntime;
    let result = fund_result_with_runtime(&runtime, options)?;
    write_action_result(options.json, &result)
}

fn fund_result_with_runtime(
    runtime: &impl BlobStorageRuntime,
    options: &options::FundOptions,
) -> Result<BlobStorageActionResult, BlobStorageCommandError> {
    let target = runtime.resolve_call_target(
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
            target.method_mode,
            command,
            Some(options.cycles),
        )
    } else {
        let call_output = runtime.call_output(
            &options.common,
            &target,
            BLOB_STORAGE_FUND_FROM_PROJECT_CYCLES,
            &arg,
            output,
        )?;
        let report = parse_funding_report(&call_output)?;
        let result = BlobStorageActionResult::completed(
            &options.deployment,
            BlobStorageActionName::Fund,
            target.target,
            BLOB_STORAGE_FUND_FROM_PROJECT_CYCLES,
            target.method_mode,
            command,
            Some(options.cycles),
        )
        .with_funding_report(report);
        with_post_status_diagnostic(
            runtime,
            &options.common,
            &options.deployment,
            &options.canister,
            result,
        )
    };
    Ok(result)
}

fn with_post_status_diagnostic(
    runtime: &impl BlobStorageRuntime,
    options: &options::CommonOptions,
    deployment: &str,
    canister: &str,
    result: BlobStorageActionResult,
) -> BlobStorageActionResult {
    match status_result_with_runtime(runtime, options, deployment, canister) {
        Ok(status) => result.with_post_status(status),
        Err(_) => result.with_warning(BLOB_STORAGE_WARNING_POST_STATUS_UNAVAILABLE),
    }
}

fn icp_cli(options: &options::CommonOptions) -> IcpCli {
    IcpCli::new(&options.icp, Some(options.environment.clone()))
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
    result.map_err(BlobStorageCommandError::from)
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
