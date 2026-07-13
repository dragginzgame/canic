//! Module: canic_cli::auth
//!
//! Responsibility: expose delegated-auth operator commands.
//! Does not own: root renewal scheduling, proof verification, or issuer install policy.
//! Boundary: parses auth CLI commands and renders root canister-call actions.

mod codec;
mod render;
#[cfg(test)]
mod tests;

use crate::{
    cli::{
        clap::{
            flag_arg, parse_matches, render_usage, required_string, string_option_or_else,
            value_arg,
        },
        defaults::{default_icp, local_network},
        globals::{internal_icp_arg, internal_network_arg},
        help::print_help_or_version,
    },
    support::candid::role_candid_path,
    version_text,
};
use canic_core::protocol::{
    CANIC_ACTIVE_DELEGATION_PROOF_STATUS, CANIC_ROOT_ISSUER_RENEWAL_STATUS,
};
use canic_host::{
    candid_endpoints::{CandidEndpointError, EndpointMode, parse_candid_service_endpoints},
    icp::{IcpCli, IcpCommandError},
    icp_config::{IcpConfigError, resolve_current_canic_icp_root},
    install_root::InstallStateError,
    installed_deployment::{
        InstalledDeploymentError, InstalledDeploymentRequest,
        resolve_installed_deployment_from_root,
    },
    registry::{RegistryEntry, RegistryParseError},
    replica_query::ReplicaQueryError,
};
use clap::Command as ClapCommand;
use serde::Serialize;
use std::{
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

use codec::{
    AuthResponseParseError, parse_issuer_observed_status, parse_issuer_principal,
    parse_renewal_status_summary, root_issuer_renewal_status_arg,
};
use render::{render_issuer_observation, write_renewal_status_result};

const COMMAND_NAME: &str = "auth";
const RENEWAL_COMMAND: &str = "renewal";
const STATUS_COMMAND: &str = "status";
const DEPLOYMENT_ARG: &str = "deployment";
const ISSUER_ARG: &str = "issuer";
const JSON_ARG: &str = "json";
const ROOT_ROLE: &str = "root";
const AUTH_RENEWAL_STATUS_SCHEMA_VERSION: u16 = 2;
const ISSUER_NOT_IN_SUBNET_REGISTRY_REASON: &str = "issuer_not_in_subnet_registry";

const HELP_AFTER: &str = "\
Examples:
  canic auth renewal status local --issuer rrkah-fqaaa-aaaaa-aaaaq-cai
  canic auth renewal status local --issuer rrkah-fqaaa-aaaaa-aaaaq-cai --json";

///
/// AuthCommandError
///

#[derive(Debug, ThisError)]
pub enum AuthCommandError {
    #[error("{0}")]
    Usage(String),

    #[error("failed to render JSON output: {0}")]
    Json(#[from] serde_json::Error),

    #[error(
        "deployment target {deployment} is not installed on network {network}; install or register it before using auth renewal commands"
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

    #[error("failed to read canic deployment state: {0}")]
    Registry(#[source] RegistryParseError),

    #[error("failed to read canic deployment state: {0}")]
    Io(#[source] io::Error),

    #[error(transparent)]
    Icp(#[from] IcpCommandError),

    #[error(
        "root target in deployment {deployment} has no local Candid sidecar; rebuild or register local metadata before using auth renewal commands"
    )]
    CandidUnavailable { deployment: String },

    #[error("issuer must be a valid principal: {issuer}")]
    InvalidIssuerPrincipal { issuer: String },

    #[error("failed to read local Candid sidecar {path}: {source}")]
    CandidRead { path: PathBuf, source: io::Error },

    #[error("failed to parse local Candid sidecar {path}: {source}")]
    CandidParse {
        path: PathBuf,
        source: CandidEndpointError,
    },

    #[error("local Candid sidecar {path} does not define auth renewal method {method}")]
    MethodUnavailable { path: PathBuf, method: String },

    #[error(
        "local Candid sidecar {path} declares auth renewal method {method} as {actual}, expected {expected}"
    )]
    MethodModeMismatch {
        path: PathBuf,
        method: String,
        expected: &'static str,
        actual: &'static str,
    },

    #[error("failed to parse auth renewal response: {detail}")]
    ResponseParse { detail: String },
}

impl AuthCommandError {
    pub const fn exit_code(&self) -> u8 {
        match self {
            Self::Icp(IcpCommandError::Io(_))
            | Self::Usage(_)
            | Self::Json(_)
            | Self::NoInstalledDeployment { .. }
            | Self::InstallState(_)
            | Self::IcpRoot(_)
            | Self::Registry(_)
            | Self::Io(_)
            | Self::CandidUnavailable { .. }
            | Self::InvalidIssuerPrincipal { .. }
            | Self::CandidRead { .. }
            | Self::CandidParse { .. }
            | Self::MethodUnavailable { .. }
            | Self::MethodModeMismatch { .. } => 1,
            Self::ReplicaQuery(_) | Self::LostLocalRoot { .. } | Self::Icp(_) => 2,
            Self::ResponseParse { .. } => 3,
        }
    }
}

impl From<AuthResponseParseError> for AuthCommandError {
    fn from(error: AuthResponseParseError) -> Self {
        Self::ResponseParse {
            detail: error.to_string(),
        }
    }
}

///
/// AuthCommand
///

#[derive(Clone, Debug, Eq, PartialEq)]
enum AuthCommand {
    RenewalStatus(RenewalStatusOptions),
}

///
/// CommonOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct CommonOptions {
    network: String,
    icp: String,
}

///
/// RenewalStatusOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct RenewalStatusOptions {
    deployment: String,
    issuer: String,
    json: bool,
    common: CommonOptions,
}

///
/// AuthOptions
///

struct AuthOptions;

impl AuthOptions {
    fn parse<I>(args: I) -> Result<AuthCommand, AuthCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches =
            parse_matches(auth_command(), args).map_err(|_| AuthCommandError::Usage(usage()))?;
        match matches.subcommand() {
            Some((RENEWAL_COMMAND, matches)) => match matches.subcommand() {
                Some((STATUS_COMMAND, matches)) => {
                    Ok(AuthCommand::RenewalStatus(RenewalStatusOptions {
                        deployment: required_string(matches, DEPLOYMENT_ARG),
                        issuer: required_string(matches, ISSUER_ARG),
                        json: matches.get_flag(JSON_ARG),
                        common: common_options(matches),
                    }))
                }
                _ => Err(AuthCommandError::Usage(usage())),
            },
            _ => Err(AuthCommandError::Usage(usage())),
        }
    }
}

/// Run the auth operator command group.
pub fn run<I>(args: I) -> Result<(), AuthCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    let command = AuthOptions::parse(args)?;
    run_command(command)
}

fn usage() -> String {
    render_usage(auth_command)
}

fn common_options(matches: &clap::ArgMatches) -> CommonOptions {
    CommonOptions {
        network: string_option_or_else(matches, "network", local_network),
        icp: string_option_or_else(matches, "icp", default_icp),
    }
}

fn auth_command() -> ClapCommand {
    ClapCommand::new(COMMAND_NAME)
        .bin_name("canic auth")
        .disable_help_flag(true)
        .about("Run delegated-auth operator workflows")
        .subcommand_required(true)
        .subcommand(renewal_command())
        .after_help(HELP_AFTER)
}

fn renewal_command() -> ClapCommand {
    ClapCommand::new(RENEWAL_COMMAND)
        .disable_help_flag(true)
        .about("Inspect root-managed chain-key delegation proof renewal")
        .subcommand_required(true)
        .subcommand(status_command())
}

fn status_command() -> ClapCommand {
    ClapCommand::new(STATUS_COMMAND)
        .disable_help_flag(true)
        .about("Show chain-key delegation proof renewal state for one issuer")
        .arg(
            value_arg(DEPLOYMENT_ARG)
                .value_name(DEPLOYMENT_ARG)
                .required(true)
                .help("Installed deployment target name"),
        )
        .arg(
            value_arg(ISSUER_ARG)
                .long(ISSUER_ARG)
                .value_name("principal")
                .required(true)
                .help("Issuer canister principal"),
        )
        .arg(flag_arg(JSON_ARG).long(JSON_ARG).help("Print JSON output"))
        .arg(internal_network_arg())
        .arg(internal_icp_arg())
}

fn run_command(command: AuthCommand) -> Result<(), AuthCommandError> {
    match command {
        AuthCommand::RenewalStatus(options) => run_renewal_status(&options),
    }
}

fn run_renewal_status(options: &RenewalStatusOptions) -> Result<(), AuthCommandError> {
    let runtime = LiveAuthRenewalRuntime;
    let result = renewal_status_result_with_runtime(&runtime, options)?;
    write_renewal_status_result(options.json, &result)
}

pub fn renewal_medic_summary(
    deployment: &str,
    issuer: &str,
    network: &str,
    icp: &str,
) -> Result<AuthRenewalMedicSummary, AuthCommandError> {
    let runtime = LiveAuthRenewalRuntime;
    let result = renewal_status_result_with_runtime(
        &runtime,
        &RenewalStatusOptions {
            deployment: deployment.to_string(),
            issuer: issuer.to_string(),
            json: true,
            common: CommonOptions {
                network: network.to_string(),
                icp: icp.to_string(),
            },
        },
    )?;
    Ok(auth_renewal_medic_summary_from_result(&result))
}

trait AuthRenewalRuntime {
    fn resolve_root_target(
        &self,
        options: &CommonOptions,
        deployment: &str,
        method: &str,
        expected_mode: AuthRenewalMethodMode,
    ) -> Result<AuthRootCallTarget, AuthCommandError>;

    fn query_output(
        &self,
        options: &CommonOptions,
        target: &AuthRootCallTarget,
        method: &str,
        arg: Option<&str>,
        output: Option<&str>,
    ) -> Result<String, AuthCommandError>;

    fn resolve_issuer_target(
        &self,
        options: &CommonOptions,
        root_target: &AuthRootCallTarget,
        issuer_pid: &str,
        method: &str,
        expected_mode: AuthRenewalMethodMode,
    ) -> Result<Option<AuthIssuerCallTarget>, AuthCommandError>;

    fn query_issuer_output(
        &self,
        options: &CommonOptions,
        target: &AuthIssuerCallTarget,
        method: &str,
        output: Option<&str>,
    ) -> Result<String, AuthCommandError>;
}

fn renewal_status_result_with_runtime(
    runtime: &impl AuthRenewalRuntime,
    options: &RenewalStatusOptions,
) -> Result<AuthRenewalStatusResult, AuthCommandError> {
    let issuer_pid = parse_issuer_principal(&options.issuer)?;
    let target = runtime.resolve_root_target(
        &options.common,
        &options.deployment,
        CANIC_ROOT_ISSUER_RENEWAL_STATUS,
        AuthRenewalMethodMode::Query,
    )?;
    let output = runtime.query_output(
        &options.common,
        &target,
        CANIC_ROOT_ISSUER_RENEWAL_STATUS,
        Some(&root_issuer_renewal_status_arg(&issuer_pid)),
        Some("json"),
    )?;
    let status = parse_renewal_status_summary(&output)?;
    let issuer_observation =
        issuer_observation_with_runtime(runtime, &options.common, &target, &issuer_pid, &status);

    Ok(AuthRenewalStatusResult {
        schema_version: AUTH_RENEWAL_STATUS_SCHEMA_VERSION,
        kind: AuthRenewalReportKind::Status,
        deployment: options.deployment.clone(),
        network: options.common.network.clone(),
        target: target.target,
        issuer_pid,
        status: renewal_status_code(&status, &issuer_observation),
        renewal: status,
        issuer_observation,
    })
}

struct LiveAuthRenewalRuntime;

impl AuthRenewalRuntime for LiveAuthRenewalRuntime {
    fn resolve_root_target(
        &self,
        options: &CommonOptions,
        deployment: &str,
        method: &str,
        expected_mode: AuthRenewalMethodMode,
    ) -> Result<AuthRootCallTarget, AuthCommandError> {
        resolve_auth_root_call_target(options, deployment, method, expected_mode)
    }

    fn query_output(
        &self,
        options: &CommonOptions,
        target: &AuthRootCallTarget,
        method: &str,
        arg: Option<&str>,
        output: Option<&str>,
    ) -> Result<String, AuthCommandError> {
        live_query_output(options, target, method, arg, output)
    }

    fn resolve_issuer_target(
        &self,
        options: &CommonOptions,
        root_target: &AuthRootCallTarget,
        issuer_pid: &str,
        method: &str,
        expected_mode: AuthRenewalMethodMode,
    ) -> Result<Option<AuthIssuerCallTarget>, AuthCommandError> {
        resolve_auth_issuer_call_target(options, root_target, issuer_pid, method, expected_mode)
    }

    fn query_issuer_output(
        &self,
        options: &CommonOptions,
        target: &AuthIssuerCallTarget,
        method: &str,
        output: Option<&str>,
    ) -> Result<String, AuthCommandError> {
        live_query_issuer_output(options, target, method, output)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AuthRenewalMethodMode {
    Query,
}

impl AuthRenewalMethodMode {
    const fn label(self) -> &'static str {
        match self {
            Self::Query => "query",
        }
    }
}

///
/// AuthRenewalReportKind
///

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum AuthRenewalReportKind {
    #[serde(rename = "auth_renewal_status")]
    Status,
}

///
/// AuthRenewalCandidSource
///

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum AuthRenewalCandidSource {
    InstalledDeployment,
}

///
/// AuthRenewalStatusCode
///

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum AuthRenewalStatusCode {
    ActiveAttempt,
    Configured,
    Disabled,
    DriftDetected,
    IssuerUnregistered,
    Missing,
    Unavailable,
}

impl AuthRenewalStatusCode {
    const fn label(self) -> &'static str {
        match self {
            Self::ActiveAttempt => "active_attempt",
            Self::Configured => "configured",
            Self::Disabled => "disabled",
            Self::DriftDetected => "drift_detected",
            Self::IssuerUnregistered => "issuer_unregistered",
            Self::Missing => "missing",
            Self::Unavailable => "unavailable",
        }
    }
}

///
/// AuthRootTarget
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct AuthRootTarget {
    input: String,
    role: String,
    canister_id: String,
    candid_source: AuthRenewalCandidSource,
}

///
/// AuthRootCallTarget
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct AuthRootCallTarget {
    target: AuthRootTarget,
    candid_path: PathBuf,
    icp_root: PathBuf,
    registry_entries: Vec<RegistryEntry>,
}

///
/// AuthIssuerTarget
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct AuthIssuerTarget {
    input: String,
    role: Option<String>,
    canister_id: String,
    candid_source: AuthRenewalCandidSource,
}

///
/// AuthIssuerCallTarget
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct AuthIssuerCallTarget {
    target: AuthIssuerTarget,
    candid_path: PathBuf,
    icp_root: PathBuf,
}

///
/// AuthRenewalTemplateStatus
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct AuthRenewalTemplateStatus {
    present: bool,
    enabled: Option<bool>,
    cert_ttl_ns: Option<String>,
}

///
/// AuthRenewalStateStatus
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct AuthRenewalStateStatus {
    present: bool,
    last_installed_cert_hash: Option<String>,
    last_outcome: Option<String>,
    consecutive_failures: Option<u64>,
    last_installed_expires_at_ns: Option<String>,
    last_installed_refresh_after_ns: Option<String>,
    next_attempt_after_ns: Option<String>,
    active_attempt_id: Option<String>,
}

///
/// AuthRenewalActiveAttemptStatus
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct AuthRenewalActiveAttemptStatus {
    present: bool,
    status: Option<String>,
    batch_id: Option<String>,
    prepared_expires_at_ns: Option<String>,
    failure: Option<String>,
}

///
/// AuthRenewalStatusSummary
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct AuthRenewalStatusSummary {
    template: AuthRenewalTemplateStatus,
    state: AuthRenewalStateStatus,
    active_attempt: AuthRenewalActiveAttemptStatus,
}

///
/// AuthIssuerObservation
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct AuthIssuerObservation {
    available: bool,
    status: String,
    drift_detected: bool,
    reason: Option<String>,
    cert_hash: Option<String>,
    expires_at_ns: Option<String>,
    refresh_after_ns: Option<String>,
}

///
/// AuthRenewalStatusResult
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct AuthRenewalStatusResult {
    schema_version: u16,
    kind: AuthRenewalReportKind,
    deployment: String,
    network: String,
    target: AuthRootTarget,
    issuer_pid: String,
    status: AuthRenewalStatusCode,
    renewal: AuthRenewalStatusSummary,
    issuer_observation: AuthIssuerObservation,
}

///
/// AuthRenewalMedicStatus
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthRenewalMedicStatus {
    Ready,
    Warning,
}

///
/// AuthRenewalMedicSummary
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthRenewalMedicSummary {
    pub status: AuthRenewalMedicStatus,
    pub detail: String,
    pub next: String,
}

fn resolve_auth_root_call_target(
    options: &CommonOptions,
    deployment: &str,
    method: &str,
    expected_mode: AuthRenewalMethodMode,
) -> Result<AuthRootCallTarget, AuthCommandError> {
    let icp_root = resolve_current_canic_icp_root().map_err(AuthCommandError::IcpRoot)?;
    let installed = resolve_installed_deployment_from_root(
        &InstalledDeploymentRequest {
            deployment: deployment.to_string(),
            network: options.network.clone(),
            icp: options.icp.clone(),
            detect_lost_local_root: true,
        },
        &icp_root,
    )
    .map_err(auth_installed_deployment_error)?;
    let candid_path =
        role_candid_path(Some(&icp_root), &options.network, ROOT_ROLE).ok_or_else(|| {
            AuthCommandError::CandidUnavailable {
                deployment: deployment.to_string(),
            }
        })?;
    let candid =
        fs::read_to_string(&candid_path).map_err(|source| AuthCommandError::CandidRead {
            path: candid_path.clone(),
            source,
        })?;
    validate_auth_method_mode(&candid_path, &candid, method, expected_mode)?;

    Ok(AuthRootCallTarget {
        target: AuthRootTarget {
            input: ROOT_ROLE.to_string(),
            role: ROOT_ROLE.to_string(),
            canister_id: installed.state.root_canister_id,
            candid_source: AuthRenewalCandidSource::InstalledDeployment,
        },
        candid_path,
        icp_root,
        registry_entries: installed.registry.entries,
    })
}

fn resolve_auth_issuer_call_target(
    options: &CommonOptions,
    root_target: &AuthRootCallTarget,
    issuer_pid: &str,
    method: &str,
    expected_mode: AuthRenewalMethodMode,
) -> Result<Option<AuthIssuerCallTarget>, AuthCommandError> {
    let Some(entry) = root_target
        .registry_entries
        .iter()
        .find(|entry| entry.pid == issuer_pid)
    else {
        return Ok(None);
    };
    let Some(role) = entry.role.as_deref() else {
        return Ok(None);
    };
    let Some(candid_path) = role_candid_path(Some(&root_target.icp_root), &options.network, role)
    else {
        return Ok(None);
    };
    let candid =
        fs::read_to_string(&candid_path).map_err(|source| AuthCommandError::CandidRead {
            path: candid_path.clone(),
            source,
        })?;
    validate_auth_method_mode(&candid_path, &candid, method, expected_mode)?;

    Ok(Some(AuthIssuerCallTarget {
        target: AuthIssuerTarget {
            input: issuer_pid.to_string(),
            role: entry.role.clone(),
            canister_id: issuer_pid.to_string(),
            candid_source: AuthRenewalCandidSource::InstalledDeployment,
        },
        candid_path,
        icp_root: root_target.icp_root.clone(),
    }))
}

fn validate_auth_method_mode(
    path: &Path,
    candid: &str,
    method: &str,
    expected_mode: AuthRenewalMethodMode,
) -> Result<(), AuthCommandError> {
    let endpoints =
        parse_candid_service_endpoints(candid).map_err(|source| AuthCommandError::CandidParse {
            path: path.to_path_buf(),
            source,
        })?;
    let endpoint = endpoints
        .iter()
        .find(|endpoint| endpoint.name == method)
        .ok_or_else(|| AuthCommandError::MethodUnavailable {
            path: path.to_path_buf(),
            method: method.to_string(),
        })?;
    let actual_mode = if endpoint
        .modes
        .iter()
        .any(|mode| matches!(mode, EndpointMode::Query | EndpointMode::CompositeQuery))
    {
        AuthRenewalMethodMode::Query
    } else {
        return Err(AuthCommandError::MethodModeMismatch {
            path: path.to_path_buf(),
            method: method.to_string(),
            expected: expected_mode.label(),
            actual: "update",
        });
    };
    if actual_mode != expected_mode {
        return Err(AuthCommandError::MethodModeMismatch {
            path: path.to_path_buf(),
            method: method.to_string(),
            expected: expected_mode.label(),
            actual: actual_mode.label(),
        });
    }
    Ok(())
}

fn icp_cli(options: &CommonOptions) -> IcpCli {
    IcpCli::new(&options.icp, None, Some(options.network.clone()))
}

fn live_query_output(
    options: &CommonOptions,
    target: &AuthRootCallTarget,
    method: &str,
    arg: Option<&str>,
    output: Option<&str>,
) -> Result<String, AuthCommandError> {
    let icp = icp_cli(options).with_cwd(&target.icp_root);
    let result = if let Some(arg) = arg {
        icp.canister_query_arg_output_with_candid(
            &target.target.canister_id,
            method,
            arg,
            output,
            Some(target.candid_path.as_path()),
        )
    } else {
        icp.canister_query_output_with_candid(
            &target.target.canister_id,
            method,
            output,
            Some(target.candid_path.as_path()),
        )
    };
    result.map_err(AuthCommandError::from)
}

fn live_query_issuer_output(
    options: &CommonOptions,
    target: &AuthIssuerCallTarget,
    method: &str,
    output: Option<&str>,
) -> Result<String, AuthCommandError> {
    let icp = icp_cli(options).with_cwd(&target.icp_root);
    icp.canister_query_output_with_candid(
        &target.target.canister_id,
        method,
        output,
        Some(target.candid_path.as_path()),
    )
    .map_err(AuthCommandError::from)
}

fn issuer_observation_with_runtime(
    runtime: &impl AuthRenewalRuntime,
    options: &CommonOptions,
    root_target: &AuthRootCallTarget,
    issuer_pid: &str,
    status: &AuthRenewalStatusSummary,
) -> AuthIssuerObservation {
    let target = match runtime.resolve_issuer_target(
        options,
        root_target,
        issuer_pid,
        CANIC_ACTIVE_DELEGATION_PROOF_STATUS,
        AuthRenewalMethodMode::Query,
    ) {
        Ok(Some(target)) => target,
        Ok(None) => return unavailable_issuer_observation(ISSUER_NOT_IN_SUBNET_REGISTRY_REASON),
        Err(_) => return unavailable_issuer_observation("issuer_status_metadata_unavailable"),
    };
    let Ok(output) = runtime.query_issuer_output(
        options,
        &target,
        CANIC_ACTIVE_DELEGATION_PROOF_STATUS,
        Some("json"),
    ) else {
        return unavailable_issuer_observation("issuer_status_query_failed");
    };
    let Ok(observed) = parse_issuer_observed_status(&output) else {
        return unavailable_issuer_observation("issuer_status_parse_failed");
    };
    issuer_observation_from_status(status, observed)
}

fn unavailable_issuer_observation(reason: &str) -> AuthIssuerObservation {
    AuthIssuerObservation {
        available: false,
        status: AuthRenewalStatusCode::Unavailable.label().to_string(),
        drift_detected: false,
        reason: Some(reason.to_string()),
        cert_hash: None,
        expires_at_ns: None,
        refresh_after_ns: None,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AuthIssuerObservedStatus {
    status: String,
    cert_hash: Option<String>,
    expires_at_ns: Option<String>,
    refresh_after_ns: Option<String>,
}
fn issuer_observation_from_status(
    status: &AuthRenewalStatusSummary,
    observed: AuthIssuerObservedStatus,
) -> AuthIssuerObservation {
    let drift_detected = issuer_observation_drift_detected(status, &observed);
    AuthIssuerObservation {
        available: true,
        status: observed.status,
        drift_detected,
        reason: None,
        cert_hash: observed.cert_hash,
        expires_at_ns: observed.expires_at_ns,
        refresh_after_ns: observed.refresh_after_ns,
    }
}

fn issuer_observation_drift_detected(
    status: &AuthRenewalStatusSummary,
    observed: &AuthIssuerObservedStatus,
) -> bool {
    let root_has_installed = status.state.last_installed_cert_hash.is_some()
        || status.state.last_installed_expires_at_ns.is_some();
    let issuer_has_active = observed.cert_hash.is_some() || observed.expires_at_ns.is_some();

    if root_has_installed != issuer_has_active {
        return true;
    }
    if status.state.last_installed_cert_hash.is_some()
        && status.state.last_installed_cert_hash != observed.cert_hash
    {
        return true;
    }
    status.state.last_installed_expires_at_ns.is_some()
        && status.state.last_installed_expires_at_ns != observed.expires_at_ns
}
fn renewal_status_code(
    status: &AuthRenewalStatusSummary,
    issuer_observation: &AuthIssuerObservation,
) -> AuthRenewalStatusCode {
    if issuer_observation.reason.as_deref() == Some(ISSUER_NOT_IN_SUBNET_REGISTRY_REASON) {
        AuthRenewalStatusCode::IssuerUnregistered
    } else if issuer_observation.drift_detected {
        AuthRenewalStatusCode::DriftDetected
    } else if status.active_attempt.present {
        AuthRenewalStatusCode::ActiveAttempt
    } else if status.template.enabled == Some(false) {
        AuthRenewalStatusCode::Disabled
    } else if status.template.present {
        AuthRenewalStatusCode::Configured
    } else {
        AuthRenewalStatusCode::Missing
    }
}

fn auth_renewal_medic_summary_from_result(
    result: &AuthRenewalStatusResult,
) -> AuthRenewalMedicSummary {
    let observation = &result.issuer_observation;
    let status = if observation.available && !observation.drift_detected {
        AuthRenewalMedicStatus::Ready
    } else {
        AuthRenewalMedicStatus::Warning
    };
    let root_cert_hash = result
        .renewal
        .state
        .last_installed_cert_hash
        .as_deref()
        .unwrap_or("-");
    let issuer_cert_hash = observation.cert_hash.as_deref().unwrap_or("-");
    let detail = format!(
        "status={}; issuer_observation={}; root_cert_hash={}; issuer_cert_hash={}; drift_detected={}",
        result.status.label(),
        render_issuer_observation(observation),
        root_cert_hash,
        issuer_cert_hash,
        observation.drift_detected
    );
    let next = if result.status == AuthRenewalStatusCode::IssuerUnregistered {
        format!(
            "restore the registered topology for issuer {} by reinstalling the affected dependency closure; do not provision delegation proof state manually",
            result.issuer_pid
        )
    } else if observation.drift_detected {
        format!(
            "run canic auth renewal status {} --issuer {}; if drift persists, wait for root chain-key renewal or retry an issuer login/update so lazy repair can run",
            result.deployment, result.issuer_pid
        )
    } else if observation.available {
        "-".to_string()
    } else {
        format!(
            "run canic auth renewal status {} --issuer {}",
            result.deployment, result.issuer_pid
        )
    };

    AuthRenewalMedicSummary {
        status,
        detail,
        next,
    }
}
fn auth_installed_deployment_error(error: InstalledDeploymentError) -> AuthCommandError {
    match error {
        InstalledDeploymentError::NoInstalledDeployment {
            network,
            deployment,
        } => AuthCommandError::NoInstalledDeployment {
            network,
            deployment,
        },
        InstalledDeploymentError::InstallState(error) => AuthCommandError::InstallState(error),
        InstalledDeploymentError::ReplicaQuery(error) => AuthCommandError::ReplicaQuery(error),
        InstalledDeploymentError::Icp(error) => AuthCommandError::Icp(error),
        InstalledDeploymentError::LostLocalDeployment { root, .. } => {
            AuthCommandError::LostLocalRoot { root }
        }
        InstalledDeploymentError::Registry(error) => AuthCommandError::Registry(error),
        InstalledDeploymentError::Io(error) => AuthCommandError::Io(error),
    }
}
