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
        defaults::{default_icp, local_environment},
        globals::{internal_environment_arg, internal_icp_arg},
        help::{print_help_or_version, print_nested_help},
    },
    version_text,
};
use canic_contracts::{
    dto::{
        auth::{ActiveDelegationProofStatus, RootIssuerRenewalBatchStatus},
        issuer_proof::ActiveDelegationProofStatusResponse,
        issuer_renewal::{RootIssuerRenewalStatusRequest, RootIssuerRenewalStatusResponse},
    },
    protocol::{issuer_proof::status as proof_status, issuer_renewal as renewal_protocol},
};
use canic_host::{
    fleet_ensure::{CurrentFleetInventoryError, resolve_current_fleet},
    icp::{IcpCli, IcpCommandError},
    icp_config::{IcpConfigError, resolve_current_canic_icp_root},
    protocol_binding::{InstalledDeclarationError, load_installed_wire_schema},
    registry::RegistryEntry,
};
use clap::Command as ClapCommand;
use codec::{issuer_observed_status, parse_issuer_principal, renewal_status_summary};
use render::{render_issuer_observation, write_renewal_status_result};
use serde::Serialize;
use std::{ffi::OsString, path::PathBuf};
use thiserror::Error as ThisError;

const COMMAND_NAME: &str = "auth";
const RENEWAL_COMMAND: &str = "renewal";
const STATUS_COMMAND: &str = "status";
const FLEET_ARG: &str = "fleet";
const ISSUER_ARG: &str = "issuer";
const JSON_ARG: &str = "json";
const ROOT_ROLE: &str = "root";
const AUTH_RENEWAL_STATUS_SCHEMA_VERSION: u16 = 1;

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

    #[error("failed to read Canic Fleet state: {0}")]
    IcpRoot(#[source] IcpConfigError),

    #[error(transparent)]
    CurrentFleet(#[from] CurrentFleetInventoryError),

    #[error(transparent)]
    Icp(#[from] IcpCommandError),

    #[error("failed to resolve installed Root protocol: {0}")]
    RootProtocolBinding(#[source] InstalledDeclarationError),
    #[error("Root rejected renewal status: {0}")]
    RootRejected(canic_contracts::dto::error::Error),
    #[error("Root renewal query failed: {0}")]
    RootQuery(#[source] canic_host::icp::IcpWireQueryError),
    #[error("issuer must be a valid principal: {issuer}")]
    InvalidIssuerPrincipal { issuer: String },

    #[error("failed to resolve exact local protocol metadata for issuer {issuer}: {source}")]
    IssuerProtocolBinding {
        issuer: String,
        #[source]
        source: Box<InstalledDeclarationError>,
    },

    #[error("installed schema does not admit the selected auth method: {0}")]
    ProtocolMethod(#[from] canic_host::protocol_binding::WireArtifactError),

    #[error(transparent)]
    IssuerQuery(#[from] canic_host::icp::IcpWireQueryError),
    #[error("issuer rejected proof-status query: {0}")]
    IssuerRejected(canic_contracts::dto::error::Error),
}

impl AuthCommandError {
    pub const fn exit_code(&self) -> u8 {
        match self {
            Self::Icp(IcpCommandError::Io(_))
            | Self::RootQuery(
                canic_host::icp::IcpWireQueryError::Icp(IcpCommandError::Io(_))
                | canic_host::icp::IcpWireQueryError::File(_),
            )
            | Self::Usage(_)
            | Self::Json(_)
            | Self::IcpRoot(_)
            | Self::RootProtocolBinding(_)
            | Self::InvalidIssuerPrincipal { .. }
            | Self::IssuerProtocolBinding { .. }
            | Self::ProtocolMethod(_)
            | Self::CurrentFleet(_) => 1,
            Self::RootQuery(
                canic_host::icp::IcpWireQueryError::Wire(_)
                | canic_host::icp::IcpWireQueryError::Response(_),
            )
            | Self::RootRejected(_) => 3,
            Self::Icp(_) | Self::IssuerQuery(_) | Self::IssuerRejected(_) | Self::RootQuery(_) => 2,
        }
    }
}

///
/// CommonOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct CommonOptions {
    environment: String,
    icp: String,
}

///
/// RenewalStatusOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct RenewalStatusOptions {
    fleet: String,
    issuer: String,
    json: bool,
    common: CommonOptions,
}

///
/// AuthOptions
///

struct AuthOptions;

impl AuthOptions {
    fn parse<I>(args: I) -> Result<RenewalStatusOptions, AuthCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches =
            parse_matches(auth_command(), args).map_err(|_| AuthCommandError::Usage(usage()))?;
        match matches.subcommand() {
            Some((RENEWAL_COMMAND, matches)) => match matches.subcommand() {
                Some((STATUS_COMMAND, matches)) => Ok(RenewalStatusOptions {
                    fleet: required_string(matches, FLEET_ARG),
                    issuer: required_string(matches, ISSUER_ARG),
                    json: matches.get_flag(JSON_ARG),
                    common: common_options(matches),
                }),
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
    if print_nested_help(&args, auth_command()) {
        return Ok(());
    }

    let options = AuthOptions::parse(args)?;
    run_renewal_status(&options)
}

fn usage() -> String {
    render_usage(auth_command)
}

fn common_options(matches: &clap::ArgMatches) -> CommonOptions {
    CommonOptions {
        environment: string_option_or_else(matches, "environment", local_environment),
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
            value_arg(FLEET_ARG)
                .value_name(FLEET_ARG)
                .required(true)
                .help("Terminal current Fleet name"),
        )
        .arg(
            value_arg(ISSUER_ARG)
                .long(ISSUER_ARG)
                .value_name("principal")
                .required(true)
                .help("Issuer canister principal"),
        )
        .arg(flag_arg(JSON_ARG).long(JSON_ARG).help("Print JSON output"))
        .arg(internal_environment_arg())
        .arg(internal_icp_arg())
}

fn run_renewal_status(options: &RenewalStatusOptions) -> Result<(), AuthCommandError> {
    let runtime = LiveAuthRenewalRuntime;
    let result = renewal_status_result_with_runtime(&runtime, options)?;
    write_renewal_status_result(options.json, &result)
}

pub fn renewal_medic_summary(
    fleet: &str,
    issuer: &str,
    environment: &str,
    icp: &str,
) -> Result<AuthRenewalMedicSummary, AuthCommandError> {
    let runtime = LiveAuthRenewalRuntime;
    let result = renewal_status_result_with_runtime(
        &runtime,
        &RenewalStatusOptions {
            fleet: fleet.to_string(),
            issuer: issuer.to_string(),
            json: true,
            common: CommonOptions {
                environment: environment.to_string(),
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
        fleet: &str,
    ) -> Result<AuthRootCallTarget, AuthCommandError>;

    fn query_renewal_status(
        &self,
        options: &CommonOptions,
        target: &AuthRootCallTarget,
        issuer_pid: &str,
    ) -> Result<RootIssuerRenewalStatusResponse, AuthCommandError>;

    fn resolve_issuer_target(
        &self,
        options: &CommonOptions,
        root_target: &AuthRootCallTarget,
        issuer_pid: &str,
    ) -> Result<Option<AuthIssuerCallTarget>, AuthCommandError>;

    fn query_issuer_status(
        &self,
        options: &CommonOptions,
        target: &AuthIssuerCallTarget,
    ) -> Result<ActiveDelegationProofStatusResponse, AuthCommandError>;
}

fn renewal_status_result_with_runtime(
    runtime: &impl AuthRenewalRuntime,
    options: &RenewalStatusOptions,
) -> Result<AuthRenewalStatusResult, AuthCommandError> {
    let issuer_pid = parse_issuer_principal(&options.issuer)?;
    let target = runtime.resolve_root_target(&options.common, &options.fleet)?;
    let response = runtime.query_renewal_status(&options.common, &target, &issuer_pid)?;
    let status = renewal_status_summary(response);
    let issuer_observation =
        issuer_observation_with_runtime(runtime, &options.common, &target, &issuer_pid, &status);

    Ok(AuthRenewalStatusResult {
        schema_version: AUTH_RENEWAL_STATUS_SCHEMA_VERSION,
        kind: AuthRenewalReportKind::Status,
        fleet: options.fleet.clone(),
        environment: options.common.environment.clone(),
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
        fleet: &str,
    ) -> Result<AuthRootCallTarget, AuthCommandError> {
        resolve_auth_root_call_target(options, fleet)
    }

    fn query_renewal_status(
        &self,
        options: &CommonOptions,
        target: &AuthRootCallTarget,
        issuer_pid: &str,
    ) -> Result<RootIssuerRenewalStatusResponse, AuthCommandError> {
        live_query_renewal_status(options, target, issuer_pid)
    }

    fn resolve_issuer_target(
        &self,
        options: &CommonOptions,
        root_target: &AuthRootCallTarget,
        issuer_pid: &str,
    ) -> Result<Option<AuthIssuerCallTarget>, AuthCommandError> {
        resolve_auth_issuer_call_target(options, root_target, issuer_pid)
    }

    fn query_issuer_status(
        &self,
        options: &CommonOptions,
        target: &AuthIssuerCallTarget,
    ) -> Result<ActiveDelegationProofStatusResponse, AuthCommandError> {
        live_query_issuer_status(options, target)
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
/// AuthRenewalProtocolSource
///

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum AuthRenewalProtocolSource {
    #[serde(rename = "installed_release")]
    InstalledRelease,
}

///
/// AuthRenewalStatusCode
///

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum AuthRenewalStatusCode {
    BatchFailed,
    BatchPending,
    Configured,
    Disabled,
    DriftDetected,
    IssuerUnregistered,
    Missing,
    ProofUnavailable,
    Unavailable,
}

impl AuthRenewalStatusCode {
    const fn label(self) -> &'static str {
        match self {
            Self::BatchFailed => "batch_failed",
            Self::BatchPending => "batch_pending",
            Self::Configured => "configured",
            Self::Disabled => "disabled",
            Self::DriftDetected => "drift_detected",
            Self::IssuerUnregistered => "issuer_unregistered",
            Self::Missing => "missing",
            Self::ProofUnavailable => "proof_unavailable",
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
    protocol_source: AuthRenewalProtocolSource,
}

///
/// AuthRootCallTarget
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct AuthRootCallTarget {
    fleet: String,
    target: AuthRootTarget,
    icp_root: PathBuf,
    registry_entries: Vec<RegistryEntry>,
}

/// Admitted issuer from the exact installed release, using the selected ICP identity.
#[derive(Clone, Debug, Eq, PartialEq)]
struct AuthIssuerCallTarget {
    canister_id: String,
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
    last_installed_expires_at_ns: Option<String>,
    last_installed_refresh_after_ns: Option<String>,
    next_attempt_after_ns: Option<String>,
}

///
/// AuthRenewalBatchStatus
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct AuthRenewalBatchStatus {
    present: bool,
    #[serde(serialize_with = "codec::serialize_batch_status")]
    status: Option<RootIssuerRenewalBatchStatus>,
    batch_id: Option<String>,
    cert_hash: Option<String>,
    proof_epoch: Option<u64>,
    expires_at_ns: Option<String>,
    installed_at_ns: Option<String>,
    retry_after_ns: Option<String>,
    failure: Option<String>,
}

///
/// AuthRenewalStatusSummary
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct AuthRenewalStatusSummary {
    template: AuthRenewalTemplateStatus,
    state: AuthRenewalStateStatus,
    latest_batch: AuthRenewalBatchStatus,
}

///
/// AuthIssuerObservation
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct AuthIssuerObservation {
    available: bool,
    #[serde(serialize_with = "codec::serialize_issuer_status")]
    status: Option<ActiveDelegationProofStatus>,
    drift_detected: bool,
    reason: Option<AuthIssuerUnavailableReason>,
    cert_hash: Option<String>,
    expires_at_ns: Option<String>,
    refresh_after_ns: Option<String>,
}

///
/// AuthIssuerUnavailableReason
///
/// Typed issuer-observation refusal retained until operator report rendering.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum AuthIssuerUnavailableReason {
    IssuerNotInComponentRegistry,
    IssuerStatusMetadataUnavailable,
    IssuerStatusParseFailed,
    IssuerStatusQueryFailed,
}

impl AuthIssuerUnavailableReason {
    const fn label(self) -> &'static str {
        match self {
            Self::IssuerNotInComponentRegistry => "issuer_not_in_component_registry",
            Self::IssuerStatusMetadataUnavailable => "issuer_status_metadata_unavailable",
            Self::IssuerStatusParseFailed => "issuer_status_parse_failed",
            Self::IssuerStatusQueryFailed => "issuer_status_query_failed",
        }
    }
}

///
/// AuthRenewalStatusResult
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct AuthRenewalStatusResult {
    schema_version: u16,
    kind: AuthRenewalReportKind,
    fleet: String,
    environment: String,
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
    fleet: &str,
) -> Result<AuthRootCallTarget, AuthCommandError> {
    let icp_root = resolve_current_canic_icp_root().map_err(AuthCommandError::IcpRoot)?;
    let current = resolve_current_fleet(&icp_root, &options.environment, fleet)?;
    let root_canister_id = current
        .topology
        .unique_fleet_subnet_root(fleet)?
        .to_string();
    let schema =
        load_installed_wire_schema(&icp_root, &options.environment, fleet, &root_canister_id)
            .map_err(AuthCommandError::RootProtocolBinding)?;
    canic_host::protocol_binding::require_wire_method(
        &schema.schema,
        renewal_protocol::METHOD,
        renewal_protocol::METHOD_ID,
        canic_contracts::protocol::wire_declaration::MethodKind::Query,
        renewal_protocol::REQUEST_LIMITS,
        renewal_protocol::RESPONSE_LIMITS,
    )?;
    Ok(AuthRootCallTarget {
        fleet: fleet.to_string(),
        target: AuthRootTarget {
            input: ROOT_ROLE.to_string(),
            role: ROOT_ROLE.to_string(),
            canister_id: root_canister_id,
            protocol_source: AuthRenewalProtocolSource::InstalledRelease,
        },
        icp_root,
        registry_entries: current.registry.entries,
    })
}

fn resolve_auth_issuer_call_target(
    options: &CommonOptions,
    root_target: &AuthRootCallTarget,
    issuer_pid: &str,
) -> Result<Option<AuthIssuerCallTarget>, AuthCommandError> {
    let Some(entry) = root_target
        .registry_entries
        .iter()
        .find(|entry| entry.pid == issuer_pid)
    else {
        return Ok(None);
    };
    // The owner checks retained release hashes and the registry's installed module.
    // An unrelated local sidecar or latest build is never a substitute.
    let schema = load_installed_wire_schema(
        &root_target.icp_root,
        &options.environment,
        &root_target.fleet,
        &entry.pid,
    )
    .map_err(|source| AuthCommandError::IssuerProtocolBinding {
        issuer: issuer_pid.to_string(),
        source: Box::new(source),
    })?;
    canic_host::protocol_binding::require_wire_method(
        &schema.schema,
        proof_status::METHOD,
        proof_status::METHOD_ID,
        canic_contracts::protocol::wire_declaration::MethodKind::Query,
        proof_status::REQUEST_LIMITS,
        proof_status::RESPONSE_LIMITS,
    )?;
    Ok(Some(AuthIssuerCallTarget {
        canister_id: issuer_pid.to_string(),
        icp_root: root_target.icp_root.clone(),
    }))
}

fn icp_cli(options: &CommonOptions) -> IcpCli {
    IcpCli::new(&options.icp, Some(options.environment.clone()))
}

fn live_query_renewal_status(
    options: &CommonOptions,
    target: &AuthRootCallTarget,
    issuer_pid: &str,
) -> Result<RootIssuerRenewalStatusResponse, AuthCommandError> {
    let request = RootIssuerRenewalStatusRequest {
        issuer_pid: ic_platform::Principal::from_text(issuer_pid).map_err(|_| {
            AuthCommandError::InvalidIssuerPrincipal {
                issuer: issuer_pid.to_string(),
            }
        })?,
    };
    let result: Result<RootIssuerRenewalStatusResponse, canic_contracts::dto::error::Error> =
        icp_cli(options)
            .with_cwd(&target.icp_root)
            .canister_query_wire(
                &target.target.canister_id,
                renewal_protocol::METHOD,
                renewal_protocol::METHOD_ID,
                &request,
                renewal_protocol::REQUEST_LIMITS,
                renewal_protocol::RESPONSE_LIMITS,
            )
            .map_err(AuthCommandError::RootQuery)?;
    result.map_err(AuthCommandError::RootRejected)
}

fn live_query_issuer_status(
    options: &CommonOptions,
    target: &AuthIssuerCallTarget,
) -> Result<ActiveDelegationProofStatusResponse, AuthCommandError> {
    let icp = icp_cli(options).with_cwd(&target.icp_root);
    let result: Result<ActiveDelegationProofStatusResponse, canic_contracts::dto::error::Error> =
        icp.canister_query_wire(
            &target.canister_id,
            proof_status::METHOD,
            proof_status::METHOD_ID,
            &(),
            proof_status::REQUEST_LIMITS,
            proof_status::RESPONSE_LIMITS,
        )?;
    result.map_err(AuthCommandError::IssuerRejected)
}

fn issuer_observation_with_runtime(
    runtime: &impl AuthRenewalRuntime,
    options: &CommonOptions,
    root_target: &AuthRootCallTarget,
    issuer_pid: &str,
    status: &AuthRenewalStatusSummary,
) -> AuthIssuerObservation {
    let target = match runtime.resolve_issuer_target(options, root_target, issuer_pid) {
        Ok(Some(target)) => target,
        Ok(None) => {
            return unavailable_issuer_observation(
                AuthIssuerUnavailableReason::IssuerNotInComponentRegistry,
            );
        }
        Err(_) => {
            return unavailable_issuer_observation(
                AuthIssuerUnavailableReason::IssuerStatusMetadataUnavailable,
            );
        }
    };
    let observed = match runtime.query_issuer_status(options, &target) {
        Ok(response) => issuer_observed_status(response),
        Err(
            AuthCommandError::IssuerQuery(
                canic_host::icp::IcpWireQueryError::Wire(_)
                | canic_host::icp::IcpWireQueryError::Response(_),
            )
            | AuthCommandError::IssuerRejected(_),
        ) => {
            return unavailable_issuer_observation(
                AuthIssuerUnavailableReason::IssuerStatusParseFailed,
            );
        }
        Err(_) => {
            return unavailable_issuer_observation(
                AuthIssuerUnavailableReason::IssuerStatusQueryFailed,
            );
        }
    };
    issuer_observation_from_status(status, observed)
}

const fn unavailable_issuer_observation(
    reason: AuthIssuerUnavailableReason,
) -> AuthIssuerObservation {
    AuthIssuerObservation {
        available: false,
        status: None,
        drift_detected: false,
        reason: Some(reason),
        cert_hash: None,
        expires_at_ns: None,
        refresh_after_ns: None,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AuthIssuerObservedStatus {
    status: ActiveDelegationProofStatus,
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
        status: Some(observed.status),
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
    if issuer_observation.reason == Some(AuthIssuerUnavailableReason::IssuerNotInComponentRegistry)
    {
        AuthRenewalStatusCode::IssuerUnregistered
    } else if issuer_observation.drift_detected {
        AuthRenewalStatusCode::DriftDetected
    } else if status.template.enabled == Some(false) {
        AuthRenewalStatusCode::Disabled
    } else if status.latest_batch.failure.is_some()
        || status.latest_batch.status == Some(RootIssuerRenewalBatchStatus::FailedRetryable)
    {
        AuthRenewalStatusCode::BatchFailed
    } else if matches!(
        status.latest_batch.status,
        Some(
            RootIssuerRenewalBatchStatus::Prepared
                | RootIssuerRenewalBatchStatus::Signing
                | RootIssuerRenewalBatchStatus::Signed
                | RootIssuerRenewalBatchStatus::Installing
        )
    ) {
        AuthRenewalStatusCode::BatchPending
    } else if !status.template.present {
        AuthRenewalStatusCode::Missing
    } else if !issuer_observation.available {
        AuthRenewalStatusCode::Unavailable
    } else if !issuer_observation_is_usable(issuer_observation) {
        AuthRenewalStatusCode::ProofUnavailable
    } else {
        AuthRenewalStatusCode::Configured
    }
}

const fn issuer_observation_is_usable(observation: &AuthIssuerObservation) -> bool {
    matches!(
        observation.status,
        Some(ActiveDelegationProofStatus::Valid | ActiveDelegationProofStatus::RefreshNeeded)
    )
}

fn auth_renewal_medic_summary_from_result(
    result: &AuthRenewalStatusResult,
) -> AuthRenewalMedicSummary {
    let observation = &result.issuer_observation;
    let issuer_ready = observation.available
        && !observation.drift_detected
        && issuer_observation_is_usable(observation);
    let status = if result.status == AuthRenewalStatusCode::Configured && issuer_ready {
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
    } else if observation.reason
        == Some(AuthIssuerUnavailableReason::IssuerStatusMetadataUnavailable)
    {
        format!(
            "restore exact local protocol metadata for issuer {} from the selected release, then rerun canic auth renewal status {} --issuer {}",
            result.issuer_pid, result.fleet, result.issuer_pid
        )
    } else if observation.drift_detected {
        format!(
            "run canic auth renewal status {} --issuer {}; if drift persists, wait for root chain-key renewal or retry an issuer login/update so lazy repair can run",
            result.fleet, result.issuer_pid
        )
    } else if result.status == AuthRenewalStatusCode::Disabled {
        format!(
            "review and enable root-managed renewal configuration for issuer {} when issuance is intended",
            result.issuer_pid
        )
    } else if result.status == AuthRenewalStatusCode::Missing {
        format!(
            "configure issuer {} through canic_root_configure_issuer with the intended Fleet audience and grants",
            result.issuer_pid
        )
    } else if result.status == AuthRenewalStatusCode::BatchFailed {
        format!(
            "wait for root chain-key renewal to retry issuer {}; inspect latest_batch.failure if the retry does not progress",
            result.issuer_pid
        )
    } else if result.status == AuthRenewalStatusCode::BatchPending {
        format!(
            "allow the root chain-key renewal batch for issuer {} to finish",
            result.issuer_pid
        )
    } else if result.status == AuthRenewalStatusCode::ProofUnavailable {
        format!(
            "provision issuer {} through the application root readiness facade or wait for root chain-key renewal",
            result.issuer_pid
        )
    } else if observation.available {
        "-".to_string()
    } else {
        format!(
            "run canic auth renewal status {} --issuer {}",
            result.fleet, result.issuer_pid
        )
    };

    AuthRenewalMedicSummary {
        status,
        detail,
        next,
    }
}
