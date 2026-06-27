//! Module: canic_cli::auth
//!
//! Responsibility: expose delegated-auth operator commands.
//! Does not own: root renewal scheduling, proof verification, or issuer install policy.
//! Boundary: parses auth CLI commands and renders root canister-call actions.

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
use candid::Principal;
use canic_core::protocol::{
    CANIC_ACTIVE_DELEGATION_PROOF_STATUS, CANIC_DELEGATION_RENEWAL_PROVISIONERS,
    CANIC_DELEGATION_RENEWAL_WORK, CANIC_GET_DELEGATION_RENEWAL_PROOF_BATCH,
    CANIC_INSTALL_DELEGATION_PROOF_BATCH, CANIC_ROOT_ISSUER_RENEWAL_STATUS,
    CANIC_UPSERT_DELEGATION_RENEWAL_PROVISIONER,
};
use canic_host::{
    candid_endpoints::{CandidEndpointError, EndpointMode, parse_candid_service_endpoints},
    icp::IcpCli,
    icp_config::resolve_current_canic_icp_root,
    installed_deployment::{
        InstalledDeploymentError, InstalledDeploymentRequest,
        resolve_installed_deployment_from_root,
    },
    registry::RegistryEntry,
    response_parse::{
        candid_record_blocks, field_value_after_equals, find_field, parse_json_u64,
        parse_u64_digits, response_candid,
    },
};
use clap::Command as ClapCommand;
use serde::Serialize;
use std::{
    collections::BTreeSet,
    ffi::OsString,
    fmt::Write as _,
    fs, io,
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

const COMMAND_NAME: &str = "auth";
const RENEWAL_COMMAND: &str = "renewal";
const RUN_ONCE_COMMAND: &str = "run-once";
const STATUS_COMMAND: &str = "status";
const PROVISIONER_COMMAND: &str = "provisioner";
const LIST_COMMAND: &str = "list";
const ENABLE_COMMAND: &str = "enable";
const DISABLE_COMMAND: &str = "disable";
const DEPLOYMENT_ARG: &str = "deployment";
const ISSUER_ARG: &str = "issuer";
const PRINCIPAL_ARG: &str = "principal";
const JSON_ARG: &str = "json";
const ROOT_ROLE: &str = "root";
const AUTH_RENEWAL_RUN_ONCE_SCHEMA_VERSION: u16 = 1;
const AUTH_RENEWAL_STATUS_SCHEMA_VERSION: u16 = 2;
const AUTH_RENEWAL_PROVISIONER_SCHEMA_VERSION: u16 = 1;
const AUTH_RENEWAL_RUN_ONCE_KIND: &str = "auth_renewal_run_once_result";
const AUTH_RENEWAL_STATUS_KIND: &str = "auth_renewal_status";
const AUTH_RENEWAL_PROVISIONER_LIST_KIND: &str = "auth_renewal_provisioners";
const AUTH_RENEWAL_PROVISIONER_UPSERT_KIND: &str = "auth_renewal_provisioner_upsert_result";
const AUTH_RENEWAL_STATUS_NO_WORK: &str = "no_work";
const AUTH_RENEWAL_STATUS_INSTALLED: &str = "installed";
const AUTH_RENEWAL_STATUS_ACTIVE_ATTEMPT: &str = "active_attempt";
const AUTH_RENEWAL_STATUS_CONFIGURED: &str = "configured";
const AUTH_RENEWAL_STATUS_DISABLED: &str = "disabled";
const AUTH_RENEWAL_STATUS_MISSING: &str = "missing";
const AUTH_RENEWAL_STATUS_UNAVAILABLE: &str = "unavailable";
const AUTH_RENEWAL_STATUS_DRIFT_DETECTED: &str = "drift_detected";
const AUTH_RENEWAL_CANDID_SOURCE_INSTALLED_DEPLOYMENT: &str = "installed_deployment";

const HELP_AFTER: &str = "\
Examples:
  canic auth renewal run-once local
  canic auth renewal run-once local --json
  canic auth renewal status local --issuer rrkah-fqaaa-aaaaa-aaaaq-cai
  canic auth renewal status local --issuer rrkah-fqaaa-aaaaa-aaaaq-cai --json
  canic auth renewal provisioner list local
  canic auth renewal provisioner enable local r7inp-6aaaa-aaaaa-aaabq-cai";

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
    InstallState(String),

    #[error("local replica query failed: {0}")]
    ReplicaQuery(String),

    #[error("icp command failed: {command}\n{stderr}")]
    IcpFailed { command: String, stderr: String },

    #[error(
        "root target in deployment {deployment} has no local Candid sidecar; rebuild or register local metadata before using auth renewal commands"
    )]
    CandidUnavailable { deployment: String },

    #[error("issuer must be a valid principal: {issuer}")]
    InvalidIssuerPrincipal { issuer: String },

    #[error("principal must be valid: {principal}")]
    InvalidPrincipal { principal: String },

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

    #[error("failed to parse root delegation renewal work response")]
    ResponseParse,
}

impl AuthCommandError {
    pub const fn exit_code(&self) -> u8 {
        match self {
            Self::ReplicaQuery(_) | Self::IcpFailed { .. } => 2,
            Self::ResponseParse => 3,
            Self::Usage(_)
            | Self::Json(_)
            | Self::NoInstalledDeployment { .. }
            | Self::InstallState(_)
            | Self::CandidUnavailable { .. }
            | Self::InvalidIssuerPrincipal { .. }
            | Self::InvalidPrincipal { .. }
            | Self::CandidRead { .. }
            | Self::CandidParse { .. }
            | Self::MethodUnavailable { .. }
            | Self::MethodModeMismatch { .. } => 1,
        }
    }
}

///
/// AuthCommand
///

#[derive(Clone, Debug, Eq, PartialEq)]
enum AuthCommand {
    RenewalRunOnce(RenewalRunOnceOptions),
    RenewalStatus(RenewalStatusOptions),
    RenewalProvisionerList(RenewalProvisionerListOptions),
    RenewalProvisionerUpsert(RenewalProvisionerUpsertOptions),
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
/// RenewalRunOnceOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct RenewalRunOnceOptions {
    deployment: String,
    json: bool,
    common: CommonOptions,
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
/// RenewalProvisionerListOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct RenewalProvisionerListOptions {
    deployment: String,
    json: bool,
    common: CommonOptions,
}

///
/// RenewalProvisionerUpsertOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct RenewalProvisionerUpsertOptions {
    deployment: String,
    principal: String,
    enabled: bool,
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
                Some((RUN_ONCE_COMMAND, matches)) => {
                    Ok(AuthCommand::RenewalRunOnce(RenewalRunOnceOptions {
                        deployment: required_string(matches, DEPLOYMENT_ARG),
                        json: matches.get_flag(JSON_ARG),
                        common: common_options(matches),
                    }))
                }
                Some((STATUS_COMMAND, matches)) => {
                    Ok(AuthCommand::RenewalStatus(RenewalStatusOptions {
                        deployment: required_string(matches, DEPLOYMENT_ARG),
                        issuer: required_string(matches, ISSUER_ARG),
                        json: matches.get_flag(JSON_ARG),
                        common: common_options(matches),
                    }))
                }
                Some((PROVISIONER_COMMAND, matches)) => match matches.subcommand() {
                    Some((LIST_COMMAND, matches)) => Ok(AuthCommand::RenewalProvisionerList(
                        RenewalProvisionerListOptions {
                            deployment: required_string(matches, DEPLOYMENT_ARG),
                            json: matches.get_flag(JSON_ARG),
                            common: common_options(matches),
                        },
                    )),
                    Some((ENABLE_COMMAND, matches)) => Ok(AuthCommand::RenewalProvisionerUpsert(
                        RenewalProvisionerUpsertOptions {
                            deployment: required_string(matches, DEPLOYMENT_ARG),
                            principal: required_string(matches, PRINCIPAL_ARG),
                            enabled: true,
                            json: matches.get_flag(JSON_ARG),
                            common: common_options(matches),
                        },
                    )),
                    Some((DISABLE_COMMAND, matches)) => Ok(AuthCommand::RenewalProvisionerUpsert(
                        RenewalProvisionerUpsertOptions {
                            deployment: required_string(matches, DEPLOYMENT_ARG),
                            principal: required_string(matches, PRINCIPAL_ARG),
                            enabled: false,
                            json: matches.get_flag(JSON_ARG),
                            common: common_options(matches),
                        },
                    )),
                    _ => Err(AuthCommandError::Usage(usage())),
                },
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
        .about("Run root-managed delegation proof renewal workflows")
        .subcommand_required(true)
        .subcommand(run_once_command())
        .subcommand(status_command())
        .subcommand(provisioner_command())
}

fn run_once_command() -> ClapCommand {
    ClapCommand::new(RUN_ONCE_COMMAND)
        .disable_help_flag(true)
        .about("Retrieve and install currently scheduled root delegation renewal proofs")
        .arg(
            value_arg(DEPLOYMENT_ARG)
                .value_name(DEPLOYMENT_ARG)
                .required(true)
                .help("Installed deployment target name"),
        )
        .arg(flag_arg(JSON_ARG).long(JSON_ARG).help("Print JSON output"))
        .arg(internal_network_arg())
        .arg(internal_icp_arg())
}

fn status_command() -> ClapCommand {
    ClapCommand::new(STATUS_COMMAND)
        .disable_help_flag(true)
        .about("Show root-managed delegation proof renewal state for one issuer")
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

fn provisioner_command() -> ClapCommand {
    ClapCommand::new(PROVISIONER_COMMAND)
        .disable_help_flag(true)
        .about("Manage constrained delegation renewal provisioners")
        .subcommand_required(true)
        .subcommand(provisioner_list_command())
        .subcommand(provisioner_enable_command())
        .subcommand(provisioner_disable_command())
}

fn provisioner_list_command() -> ClapCommand {
    ClapCommand::new(LIST_COMMAND)
        .disable_help_flag(true)
        .about("List principals allowed to complete scheduled renewal work")
        .arg(
            value_arg(DEPLOYMENT_ARG)
                .value_name(DEPLOYMENT_ARG)
                .required(true)
                .help("Installed deployment target name"),
        )
        .arg(flag_arg(JSON_ARG).long(JSON_ARG).help("Print JSON output"))
        .arg(internal_network_arg())
        .arg(internal_icp_arg())
}

fn provisioner_enable_command() -> ClapCommand {
    provisioner_upsert_command(ENABLE_COMMAND, "Enable a delegation renewal provisioner")
}

fn provisioner_disable_command() -> ClapCommand {
    provisioner_upsert_command(DISABLE_COMMAND, "Disable a delegation renewal provisioner")
}

fn provisioner_upsert_command(name: &'static str, about: &'static str) -> ClapCommand {
    ClapCommand::new(name)
        .disable_help_flag(true)
        .about(about)
        .arg(
            value_arg(DEPLOYMENT_ARG)
                .value_name(DEPLOYMENT_ARG)
                .required(true)
                .help("Installed deployment target name"),
        )
        .arg(
            value_arg(PRINCIPAL_ARG)
                .value_name(PRINCIPAL_ARG)
                .required(true)
                .help("Provisioner principal"),
        )
        .arg(flag_arg(JSON_ARG).long(JSON_ARG).help("Print JSON output"))
        .arg(internal_network_arg())
        .arg(internal_icp_arg())
}

fn run_command(command: AuthCommand) -> Result<(), AuthCommandError> {
    match command {
        AuthCommand::RenewalRunOnce(options) => run_renewal_once(&options),
        AuthCommand::RenewalStatus(options) => run_renewal_status(&options),
        AuthCommand::RenewalProvisionerList(options) => run_renewal_provisioner_list(&options),
        AuthCommand::RenewalProvisionerUpsert(options) => run_renewal_provisioner_upsert(&options),
    }
}

fn run_renewal_once(options: &RenewalRunOnceOptions) -> Result<(), AuthCommandError> {
    let runtime = LiveAuthRenewalRuntime;
    let result = renewal_once_result_with_runtime(&runtime, options)?;
    write_renewal_once_result(options.json, &result)
}

fn run_renewal_status(options: &RenewalStatusOptions) -> Result<(), AuthCommandError> {
    let runtime = LiveAuthRenewalRuntime;
    let result = renewal_status_result_with_runtime(&runtime, options)?;
    write_renewal_status_result(options.json, &result)
}

fn run_renewal_provisioner_list(
    options: &RenewalProvisionerListOptions,
) -> Result<(), AuthCommandError> {
    let runtime = LiveAuthRenewalRuntime;
    let result = renewal_provisioner_list_result_with_runtime(&runtime, options)?;
    write_renewal_provisioner_list_result(options.json, &result)
}

fn run_renewal_provisioner_upsert(
    options: &RenewalProvisionerUpsertOptions,
) -> Result<(), AuthCommandError> {
    let runtime = LiveAuthRenewalRuntime;
    let result = renewal_provisioner_upsert_result_with_runtime(&runtime, options)?;
    write_renewal_provisioner_upsert_result(options.json, &result)
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

    fn call_output(
        &self,
        options: &CommonOptions,
        target: &AuthRootCallTarget,
        method: &str,
        arg: &str,
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
    let status = parse_renewal_status_summary(&output).ok_or(AuthCommandError::ResponseParse)?;
    let issuer_observation =
        issuer_observation_with_runtime(runtime, &options.common, &target, &issuer_pid, &status);

    Ok(AuthRenewalStatusResult {
        schema_version: AUTH_RENEWAL_STATUS_SCHEMA_VERSION,
        kind: AUTH_RENEWAL_STATUS_KIND.to_string(),
        deployment: options.deployment.clone(),
        network: options.common.network.clone(),
        target: target.target,
        issuer_pid,
        status: renewal_status_code(&status, &issuer_observation).to_string(),
        renewal: status,
        issuer_observation,
    })
}

fn renewal_provisioner_list_result_with_runtime(
    runtime: &impl AuthRenewalRuntime,
    options: &RenewalProvisionerListOptions,
) -> Result<AuthRenewalProvisionerListResult, AuthCommandError> {
    let target = runtime.resolve_root_target(
        &options.common,
        &options.deployment,
        CANIC_DELEGATION_RENEWAL_PROVISIONERS,
        AuthRenewalMethodMode::Query,
    )?;
    let output = runtime.query_output(
        &options.common,
        &target,
        CANIC_DELEGATION_RENEWAL_PROVISIONERS,
        None,
        Some("json"),
    )?;
    let provisioners =
        parse_renewal_provisioners(&output).ok_or(AuthCommandError::ResponseParse)?;

    Ok(AuthRenewalProvisionerListResult {
        schema_version: AUTH_RENEWAL_PROVISIONER_SCHEMA_VERSION,
        kind: AUTH_RENEWAL_PROVISIONER_LIST_KIND.to_string(),
        deployment: options.deployment.clone(),
        network: options.common.network.clone(),
        target: target.target,
        provisioners,
    })
}

fn renewal_provisioner_upsert_result_with_runtime(
    runtime: &impl AuthRenewalRuntime,
    options: &RenewalProvisionerUpsertOptions,
) -> Result<AuthRenewalProvisionerUpsertResult, AuthCommandError> {
    let principal = parse_principal_text(&options.principal)?;
    let target = runtime.resolve_root_target(
        &options.common,
        &options.deployment,
        CANIC_UPSERT_DELEGATION_RENEWAL_PROVISIONER,
        AuthRenewalMethodMode::Update,
    )?;
    let output = runtime.call_output(
        &options.common,
        &target,
        CANIC_UPSERT_DELEGATION_RENEWAL_PROVISIONER,
        &renewal_provisioner_upsert_arg(&principal, options.enabled),
        Some("json"),
    )?;
    let provisioner =
        parse_renewal_provisioner_response(&output).ok_or(AuthCommandError::ResponseParse)?;

    Ok(AuthRenewalProvisionerUpsertResult {
        schema_version: AUTH_RENEWAL_PROVISIONER_SCHEMA_VERSION,
        kind: AUTH_RENEWAL_PROVISIONER_UPSERT_KIND.to_string(),
        deployment: options.deployment.clone(),
        network: options.common.network.clone(),
        target: target.target,
        provisioner,
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

    fn call_output(
        &self,
        options: &CommonOptions,
        target: &AuthRootCallTarget,
        method: &str,
        arg: &str,
        output: Option<&str>,
    ) -> Result<String, AuthCommandError> {
        live_call_output(options, target, method, arg, output)
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

fn renewal_once_result_with_runtime(
    runtime: &impl AuthRenewalRuntime,
    options: &RenewalRunOnceOptions,
) -> Result<AuthRenewalRunOnceResult, AuthCommandError> {
    let work_target = runtime.resolve_root_target(
        &options.common,
        &options.deployment,
        CANIC_DELEGATION_RENEWAL_WORK,
        AuthRenewalMethodMode::Query,
    )?;
    let work_output = runtime.query_output(
        &options.common,
        &work_target,
        CANIC_DELEGATION_RENEWAL_WORK,
        None,
        Some("json"),
    )?;
    let work_batches = parse_work_batches(&work_output).ok_or(AuthCommandError::ResponseParse)?;

    let mut batches = Vec::with_capacity(work_batches.len());
    for work in work_batches {
        let get_target = runtime.resolve_root_target(
            &options.common,
            &options.deployment,
            CANIC_GET_DELEGATION_RENEWAL_PROOF_BATCH,
            AuthRenewalMethodMode::Query,
        )?;
        let batch_arg = root_delegation_renewal_batch_get_arg(work.batch_id);
        let proof_arg = runtime.query_output(
            &options.common,
            &get_target,
            CANIC_GET_DELEGATION_RENEWAL_PROOF_BATCH,
            Some(&batch_arg),
            None,
        )?;
        let install_target = runtime.resolve_root_target(
            &options.common,
            &options.deployment,
            CANIC_INSTALL_DELEGATION_PROOF_BATCH,
            AuthRenewalMethodMode::Update,
        )?;
        let _install_output = runtime.call_output(
            &options.common,
            &install_target,
            CANIC_INSTALL_DELEGATION_PROOF_BATCH,
            proof_arg.trim(),
            Some("json"),
        )?;
        batches.push(AuthRenewalBatchRunResult {
            batch_id: hex_bytes(&work.batch_id),
            attempt_count: work.attempt_count,
            status: AUTH_RENEWAL_STATUS_INSTALLED.to_string(),
            retrieved: true,
            installed: true,
        });
    }

    Ok(AuthRenewalRunOnceResult {
        schema_version: AUTH_RENEWAL_RUN_ONCE_SCHEMA_VERSION,
        kind: AUTH_RENEWAL_RUN_ONCE_KIND.to_string(),
        deployment: options.deployment.clone(),
        network: options.common.network.clone(),
        target: work_target.target,
        status: if batches.is_empty() {
            AUTH_RENEWAL_STATUS_NO_WORK.to_string()
        } else {
            AUTH_RENEWAL_STATUS_INSTALLED.to_string()
        },
        batches,
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AuthRenewalMethodMode {
    Query,
    Update,
}

impl AuthRenewalMethodMode {
    const fn label(self) -> &'static str {
        match self {
            Self::Query => "query",
            Self::Update => "update",
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
    candid_source: String,
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
    candid_source: String,
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
/// AuthRenewalBatchWork
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct AuthRenewalBatchWork {
    batch_id: [u8; 32],
    attempt_count: Option<u64>,
}

///
/// AuthRenewalBatchRunResult
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct AuthRenewalBatchRunResult {
    batch_id: String,
    attempt_count: Option<u64>,
    status: String,
    retrieved: bool,
    installed: bool,
}

///
/// AuthRenewalRunOnceResult
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct AuthRenewalRunOnceResult {
    schema_version: u16,
    kind: String,
    deployment: String,
    network: String,
    target: AuthRootTarget,
    status: String,
    batches: Vec<AuthRenewalBatchRunResult>,
}

///
/// AuthRenewalProvisioner
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct AuthRenewalProvisioner {
    principal: String,
    enabled: bool,
}

///
/// AuthRenewalProvisionerListResult
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct AuthRenewalProvisionerListResult {
    schema_version: u16,
    kind: String,
    deployment: String,
    network: String,
    target: AuthRootTarget,
    provisioners: Vec<AuthRenewalProvisioner>,
}

///
/// AuthRenewalProvisionerUpsertResult
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct AuthRenewalProvisionerUpsertResult {
    schema_version: u16,
    kind: String,
    deployment: String,
    network: String,
    target: AuthRootTarget,
    provisioner: AuthRenewalProvisioner,
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
    kind: String,
    deployment: String,
    network: String,
    target: AuthRootTarget,
    issuer_pid: String,
    status: String,
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
    let icp_root = resolve_current_canic_icp_root()
        .map_err(|err| AuthCommandError::InstallState(err.to_string()))?;
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
            candid_source: AUTH_RENEWAL_CANDID_SOURCE_INSTALLED_DEPLOYMENT.to_string(),
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
            candid_source: AUTH_RENEWAL_CANDID_SOURCE_INSTALLED_DEPLOYMENT.to_string(),
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
        AuthRenewalMethodMode::Update
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
    result.map_err(auth_icp_error)
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
    .map_err(auth_icp_error)
}

fn live_call_output(
    options: &CommonOptions,
    target: &AuthRootCallTarget,
    method: &str,
    arg: &str,
    output: Option<&str>,
) -> Result<String, AuthCommandError> {
    let icp = icp_cli(options).with_cwd(&target.icp_root);
    icp.canister_call_arg_output_with_candid(
        &target.target.canister_id,
        method,
        arg,
        output,
        Some(target.candid_path.as_path()),
    )
    .map_err(auth_icp_error)
}

fn parse_work_batches(output: &str) -> Option<Vec<AuthRenewalBatchWork>> {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(output) {
        if let Some(batches) = find_field(&value, "batches").and_then(serde_json::Value::as_array) {
            let parsed = parse_work_batches_json(batches)?;
            return Some(dedupe_work_batches(parsed));
        }
        if let Some(candid) = response_candid(&value) {
            return parse_work_batches_candid(candid);
        }
    }
    parse_work_batches_candid(output)
}

fn parse_issuer_principal(issuer: &str) -> Result<String, AuthCommandError> {
    Principal::from_text(issuer)
        .map(|principal| principal.to_text())
        .map_err(|_| AuthCommandError::InvalidIssuerPrincipal {
            issuer: issuer.to_string(),
        })
}

fn parse_principal_text(principal: &str) -> Result<String, AuthCommandError> {
    Principal::from_text(principal)
        .map(|principal| principal.to_text())
        .map_err(|_| AuthCommandError::InvalidPrincipal {
            principal: principal.to_string(),
        })
}

fn parse_renewal_provisioners(output: &str) -> Option<Vec<AuthRenewalProvisioner>> {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(output) {
        let payload = find_field(&value, "Ok").unwrap_or(&value);
        if let Some(values) =
            find_field(payload, "provisioners").and_then(serde_json::Value::as_array)
        {
            let mut provisioners = values
                .iter()
                .map(parse_renewal_provisioner_json)
                .collect::<Option<Vec<_>>>()?;
            sort_provisioners(&mut provisioners);
            return Some(provisioners);
        }
        if let Some(candid) = response_candid(&value) {
            return parse_renewal_provisioners_candid(candid);
        }
    }
    parse_renewal_provisioners_candid(output)
}

fn parse_renewal_provisioner_response(output: &str) -> Option<AuthRenewalProvisioner> {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(output) {
        let payload = find_field(&value, "Ok").unwrap_or(&value);
        if let Some(provisioner) =
            find_field(payload, "provisioner").and_then(parse_renewal_provisioner_json)
        {
            return Some(provisioner);
        }
        if let Some(candid) = response_candid(&value) {
            return parse_renewal_provisioners_candid(candid)?
                .into_iter()
                .next();
        }
    }
    parse_renewal_provisioners_candid(output)?
        .into_iter()
        .next()
}

fn parse_renewal_provisioner_json(value: &serde_json::Value) -> Option<AuthRenewalProvisioner> {
    Some(AuthRenewalProvisioner {
        principal: find_field(value, "principal").and_then(parse_principal_json)?,
        enabled: find_field(value, "enabled").and_then(serde_json::Value::as_bool)?,
    })
}

fn parse_principal_json(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(value) => Principal::from_text(value)
            .ok()
            .map(|principal| principal.to_text()),
        serde_json::Value::Array(values) => values.iter().find_map(parse_principal_json),
        serde_json::Value::Object(map) => map.values().find_map(parse_principal_json),
        _ => None,
    }
}

fn parse_renewal_provisioners_candid(output: &str) -> Option<Vec<AuthRenewalProvisioner>> {
    if !output.contains("principal") || !output.contains("enabled") {
        return None;
    }
    let mut provisioners = candid_record_blocks(output)
        .into_iter()
        .filter(|block| block.contains("principal") && block.contains("enabled"))
        .filter_map(parse_renewal_provisioner_candid)
        .collect::<Vec<_>>();
    sort_provisioners(&mut provisioners);
    provisioners.dedup_by(|left, right| left.principal == right.principal);
    Some(provisioners)
}

fn parse_renewal_provisioner_candid(block: &str) -> Option<AuthRenewalProvisioner> {
    Some(AuthRenewalProvisioner {
        principal: field_value_after_equals(block, "principal").and_then(parse_candid_principal)?,
        enabled: field_value_after_equals(block, "enabled").and_then(parse_candid_bool)?,
    })
}

fn parse_candid_principal(value: &str) -> Option<String> {
    let value = value.trim_start().strip_prefix("principal")?.trim_start();
    let value = value.strip_prefix('"')?;
    let end = value.find('"')?;
    Principal::from_text(&value[..end])
        .ok()
        .map(|principal| principal.to_text())
}

fn parse_candid_bool(value: &str) -> Option<bool> {
    let value = value.trim_start();
    if value.starts_with("true") {
        Some(true)
    } else if value.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

fn sort_provisioners(provisioners: &mut [AuthRenewalProvisioner]) {
    provisioners.sort_by(|left, right| left.principal.cmp(&right.principal));
}

fn parse_renewal_status_summary(output: &str) -> Option<AuthRenewalStatusSummary> {
    let value = serde_json::from_str::<serde_json::Value>(output).ok()?;
    let payload = find_field(&value, "Ok").unwrap_or(&value);
    let template = find_field(payload, "template").and_then(option_payload);
    let state = find_field(payload, "state").and_then(option_payload);
    let active_attempt = find_field(payload, "active_attempt").and_then(option_payload);

    Some(AuthRenewalStatusSummary {
        template: AuthRenewalTemplateStatus {
            present: template.is_some(),
            enabled: template
                .and_then(|value| find_field(value, "enabled"))
                .and_then(serde_json::Value::as_bool),
            cert_ttl_ns: template
                .and_then(|value| find_field(value, "cert_ttl_ns"))
                .and_then(parse_u64_deep)
                .map(|value| value.to_string()),
        },
        state: AuthRenewalStateStatus {
            present: state.is_some(),
            last_installed_cert_hash: state
                .and_then(|value| find_field(value, "last_installed_cert_hash"))
                .and_then(parse_optional_bytes32_hex),
            last_outcome: state
                .and_then(|value| find_field(value, "last_outcome"))
                .and_then(parse_variant_code),
            consecutive_failures: state
                .and_then(|value| find_field(value, "consecutive_failures"))
                .and_then(parse_u64_deep),
            last_installed_expires_at_ns: state
                .and_then(|value| find_field(value, "last_installed_expires_at_ns"))
                .and_then(parse_optional_u64_deep)
                .map(|value| value.to_string()),
            last_installed_refresh_after_ns: state
                .and_then(|value| find_field(value, "last_installed_refresh_after_ns"))
                .and_then(parse_optional_u64_deep)
                .map(|value| value.to_string()),
            next_attempt_after_ns: state
                .and_then(|value| find_field(value, "next_attempt_after_ns"))
                .and_then(parse_u64_deep)
                .map(|value| value.to_string()),
            active_attempt_id: state
                .and_then(|value| find_field(value, "active_attempt_id"))
                .and_then(parse_optional_bytes32_hex),
        },
        active_attempt: AuthRenewalActiveAttemptStatus {
            present: active_attempt.is_some(),
            status: active_attempt
                .and_then(|value| find_field(value, "status"))
                .and_then(parse_variant_code),
            batch_id: active_attempt
                .and_then(|value| find_field(value, "batch_id"))
                .and_then(parse_bytes32_hex_deep),
            prepared_expires_at_ns: active_attempt
                .and_then(|value| find_field(value, "prepared_expires_at_ns"))
                .and_then(parse_u64_deep)
                .map(|value| value.to_string()),
            failure: active_attempt
                .and_then(|value| find_field(value, "failure"))
                .and_then(parse_optional_variant_code),
        },
    })
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
        Ok(None) => return unavailable_issuer_observation("issuer_not_in_local_registry"),
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
    let Some(observed) = parse_issuer_observed_status(&output) else {
        return unavailable_issuer_observation("issuer_status_parse_failed");
    };
    issuer_observation_from_status(status, observed)
}

fn unavailable_issuer_observation(reason: &str) -> AuthIssuerObservation {
    AuthIssuerObservation {
        available: false,
        status: AUTH_RENEWAL_STATUS_UNAVAILABLE.to_string(),
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

fn parse_issuer_observed_status(output: &str) -> Option<AuthIssuerObservedStatus> {
    let value = serde_json::from_str::<serde_json::Value>(output).ok()?;
    let payload = find_field(&value, "Ok").unwrap_or(&value);

    Some(AuthIssuerObservedStatus {
        status: find_field(payload, "status").and_then(parse_variant_code)?,
        cert_hash: find_field(payload, "cert_hash").and_then(parse_optional_bytes32_hex),
        expires_at_ns: find_field(payload, "expires_at_ns")
            .and_then(parse_optional_u64_deep)
            .map(|value| value.to_string()),
        refresh_after_ns: find_field(payload, "refresh_after_ns")
            .and_then(parse_optional_u64_deep)
            .map(|value| value.to_string()),
    })
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

fn option_payload(value: &serde_json::Value) -> Option<&serde_json::Value> {
    match value {
        serde_json::Value::Null => None,
        serde_json::Value::Array(values) => values.first().and_then(option_payload),
        _ => Some(value),
    }
}

fn parse_optional_u64_deep(value: &serde_json::Value) -> Option<u64> {
    option_payload(value).and_then(parse_u64_deep)
}

fn parse_u64_deep(value: &serde_json::Value) -> Option<u64> {
    parse_json_u64(value).or_else(|| match value {
        serde_json::Value::Array(values) => values.iter().find_map(parse_u64_deep),
        serde_json::Value::Object(map) => map.values().find_map(parse_u64_deep),
        _ => None,
    })
}

fn parse_optional_bytes32_hex(value: &serde_json::Value) -> Option<String> {
    if value.is_null() {
        return None;
    }
    parse_bytes32_hex_deep(value).or_else(|| match value {
        serde_json::Value::Array(values) if values.len() == 1 => {
            parse_optional_bytes32_hex(&values[0])
        }
        _ => None,
    })
}

fn parse_bytes32_hex_deep(value: &serde_json::Value) -> Option<String> {
    parse_bytes32_json(value).map(|bytes| hex_bytes(&bytes))
}

fn parse_optional_variant_code(value: &serde_json::Value) -> Option<String> {
    option_payload(value).and_then(parse_variant_code)
}

fn parse_variant_code(value: &serde_json::Value) -> Option<String> {
    parse_variant_name(value).map(|variant| pascal_to_snake(&variant))
}

fn parse_variant_name(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(value) => Some(value.clone()),
        serde_json::Value::Object(map) => map.keys().next().cloned(),
        serde_json::Value::Array(values) => values.iter().find_map(parse_variant_name),
        _ => None,
    }
}

fn pascal_to_snake(value: &str) -> String {
    let mut rendered = String::with_capacity(value.len());
    for (index, ch) in value.chars().enumerate() {
        if ch.is_ascii_uppercase() {
            if index > 0 {
                rendered.push('_');
            }
            rendered.push(ch.to_ascii_lowercase());
        } else {
            rendered.push(ch);
        }
    }
    rendered
}

fn renewal_status_code(
    status: &AuthRenewalStatusSummary,
    issuer_observation: &AuthIssuerObservation,
) -> &'static str {
    if issuer_observation.drift_detected {
        AUTH_RENEWAL_STATUS_DRIFT_DETECTED
    } else if status.active_attempt.present {
        AUTH_RENEWAL_STATUS_ACTIVE_ATTEMPT
    } else if status.template.enabled == Some(false) {
        AUTH_RENEWAL_STATUS_DISABLED
    } else if status.template.present {
        AUTH_RENEWAL_STATUS_CONFIGURED
    } else {
        AUTH_RENEWAL_STATUS_MISSING
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
        result.status,
        render_issuer_observation(observation),
        root_cert_hash,
        issuer_cert_hash,
        observation.drift_detected
    );
    let next = if observation.drift_detected {
        format!(
            "run canic auth renewal status {} --issuer {}; if drift persists, run canic auth renewal run-once {} or repair the issuer active proof",
            result.deployment, result.issuer_pid, result.deployment
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

fn parse_work_batches_json(values: &[serde_json::Value]) -> Option<Vec<AuthRenewalBatchWork>> {
    values.iter().map(parse_work_batch_json).collect()
}

fn parse_work_batch_json(value: &serde_json::Value) -> Option<AuthRenewalBatchWork> {
    let batch_id = value
        .get("batch_id")
        .or_else(|| find_field(value, "batch_id"))
        .and_then(parse_bytes32_json)?;
    let attempt_count = value
        .get("attempt_count")
        .and_then(parse_json_u64)
        .or_else(|| {
            value
                .get("attempts")
                .and_then(serde_json::Value::as_array)
                .map(|attempts| attempts.len() as u64)
        });
    Some(AuthRenewalBatchWork {
        batch_id,
        attempt_count,
    })
}

fn parse_bytes32_json(value: &serde_json::Value) -> Option<[u8; 32]> {
    match value {
        serde_json::Value::Array(values) => bytes32_from_iter(
            values
                .iter()
                .map(parse_json_byte)
                .collect::<Option<Vec<_>>>()?,
        ),
        serde_json::Value::String(value) => parse_hex_bytes32(value),
        serde_json::Value::Object(map) => map.values().find_map(parse_bytes32_json),
        _ => None,
    }
}

fn parse_json_byte(value: &serde_json::Value) -> Option<u8> {
    let byte = parse_json_u64(value)?;
    u8::try_from(byte).ok()
}

fn parse_work_batches_candid(output: &str) -> Option<Vec<AuthRenewalBatchWork>> {
    if !output.contains("batches") {
        return None;
    }
    let batches = candid_record_blocks(output)
        .into_iter()
        .filter(|block| block.contains("batch_id") && block.contains("attempt_count"))
        .filter_map(parse_work_batch_candid)
        .collect::<Vec<_>>();
    Some(dedupe_work_batches(batches))
}

fn parse_work_batch_candid(block: &str) -> Option<AuthRenewalBatchWork> {
    let batch_id = parse_candid_bytes32_field(block, "batch_id")?;
    let attempt_count = field_value_after_equals(block, "attempt_count").and_then(parse_u64_digits);
    Some(AuthRenewalBatchWork {
        batch_id,
        attempt_count,
    })
}

fn parse_candid_bytes32_field(text: &str, field: &str) -> Option<[u8; 32]> {
    let after_eq = field_value_after_equals(text, field)?;
    parse_candid_bytes32(after_eq)
}

fn parse_candid_bytes32(text: &str) -> Option<[u8; 32]> {
    let trimmed = text.trim_start();
    if trimmed.starts_with("blob") {
        return parse_candid_blob_literal(trimmed).and_then(bytes32_from_iter);
    }
    if trimmed.starts_with("vec") {
        return parse_candid_vec_nat8(trimmed).and_then(bytes32_from_iter);
    }
    None
}

fn parse_candid_blob_literal(text: &str) -> Option<Vec<u8>> {
    let after_blob = text.strip_prefix("blob")?.trim_start();
    let bytes = after_blob.as_bytes();
    if bytes.first().copied() != Some(b'"') {
        return None;
    }

    let mut parsed = Vec::new();
    let mut index = 1;
    while index < bytes.len() {
        match bytes[index] {
            b'"' => return Some(parsed),
            b'\\' => {
                if index + 2 < bytes.len()
                    && let (Some(high), Some(low)) =
                        (hex_value(bytes[index + 1]), hex_value(bytes[index + 2]))
                {
                    parsed.push((high << 4) | low);
                    index += 3;
                    continue;
                }
                let escaped = *bytes.get(index + 1)?;
                parsed.push(match escaped {
                    b'n' => b'\n',
                    b'r' => b'\r',
                    b't' => b'\t',
                    other => other,
                });
                index += 2;
            }
            byte => {
                parsed.push(byte);
                index += 1;
            }
        }
    }
    None
}

fn parse_candid_vec_nat8(text: &str) -> Option<Vec<u8>> {
    let start = text.find('{')?;
    let end = text[start + 1..].find('}')? + start + 1;
    let body = &text[start + 1..end];
    let mut bytes = Vec::new();
    let mut current = String::new();
    for ch in body.chars() {
        if ch.is_ascii_digit() {
            current.push(ch);
        } else if !current.is_empty() {
            bytes.push(current.parse::<u8>().ok()?);
            current.clear();
        }
    }
    if !current.is_empty() {
        bytes.push(current.parse::<u8>().ok()?);
    }
    Some(bytes)
}

fn bytes32_from_iter(bytes: Vec<u8>) -> Option<[u8; 32]> {
    bytes.try_into().ok()
}

fn parse_hex_bytes32(value: &str) -> Option<[u8; 32]> {
    let hex = value.strip_prefix("0x").unwrap_or(value);
    if hex.len() != 64 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let mut bytes = [0_u8; 32];
    for (index, chunk) in hex.as_bytes().chunks(2).enumerate() {
        let high = hex_value(chunk[0])?;
        let low = hex_value(chunk[1])?;
        bytes[index] = (high << 4) | low;
    }
    Some(bytes)
}

const fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn dedupe_work_batches(batches: Vec<AuthRenewalBatchWork>) -> Vec<AuthRenewalBatchWork> {
    let mut seen = BTreeSet::new();
    let mut deduped = Vec::new();
    for batch in batches {
        if seen.insert(batch.batch_id) {
            deduped.push(batch);
        }
    }
    deduped
}

fn root_delegation_renewal_batch_get_arg(batch_id: [u8; 32]) -> String {
    format!("(record {{ batch_id = {} }})", candid_blob32(&batch_id))
}

fn root_issuer_renewal_status_arg(issuer_pid: &str) -> String {
    format!(r#"(record {{ issuer_pid = principal "{issuer_pid}" }})"#)
}

fn renewal_provisioner_upsert_arg(principal: &str, enabled: bool) -> String {
    format!(r#"(record {{ principal = principal "{principal}"; enabled = {enabled} }})"#)
}

fn candid_blob32(bytes: &[u8; 32]) -> String {
    let mut rendered = String::from("blob \"");
    for byte in bytes {
        write!(&mut rendered, "\\{byte:02x}").expect("write to string");
    }
    rendered.push('"');
    rendered
}

fn hex_bytes(bytes: &[u8; 32]) -> String {
    let mut rendered = String::with_capacity(64);
    for byte in bytes {
        write!(&mut rendered, "{byte:02x}").expect("write to string");
    }
    rendered
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
        InstalledDeploymentError::IcpFailed { command, stderr } => {
            AuthCommandError::IcpFailed { command, stderr }
        }
        InstalledDeploymentError::LostLocalDeployment { root, .. } => {
            AuthCommandError::ReplicaQuery(format!("root canister {root} is not present"))
        }
        InstalledDeploymentError::Registry(error) => {
            AuthCommandError::InstallState(error.to_string())
        }
        InstalledDeploymentError::Io(error) => AuthCommandError::InstallState(error.to_string()),
    }
}

fn auth_icp_error(error: canic_host::icp::IcpCommandError) -> AuthCommandError {
    match error {
        canic_host::icp::IcpCommandError::Io(err) => {
            AuthCommandError::InstallState(err.to_string())
        }
        canic_host::icp::IcpCommandError::Failed { command, stderr }
        | canic_host::icp::IcpCommandError::Json {
            command,
            output: stderr,
            ..
        } => AuthCommandError::IcpFailed { command, stderr },
        canic_host::icp::IcpCommandError::SnapshotIdUnavailable { output } => {
            AuthCommandError::IcpFailed {
                command: "icp canister call".to_string(),
                stderr: output,
            }
        }
        error @ (canic_host::icp::IcpCommandError::MissingCli { .. }
        | canic_host::icp::IcpCommandError::IncompatibleCliVersion { .. }) => {
            AuthCommandError::IcpFailed {
                command: "icp --version".to_string(),
                stderr: error.to_string(),
            }
        }
    }
}

fn write_renewal_once_result(
    json: bool,
    result: &AuthRenewalRunOnceResult,
) -> Result<(), AuthCommandError> {
    if json {
        println!("{}", serde_json::to_string_pretty(result)?);
    } else if result.batches.is_empty() {
        println!("No scheduled delegation renewal work.");
    } else {
        for batch in &result.batches {
            match batch.attempt_count {
                Some(attempts) => println!(
                    "Installed renewal batch {} (attempts: {attempts})",
                    batch.batch_id
                ),
                None => println!("Installed renewal batch {}", batch.batch_id),
            }
        }
    }
    Ok(())
}

fn write_renewal_status_result(
    json: bool,
    result: &AuthRenewalStatusResult,
) -> Result<(), AuthCommandError> {
    if json {
        println!("{}", serde_json::to_string_pretty(result)?);
    } else {
        println!("{}", render_renewal_status_result(result));
    }
    Ok(())
}

fn write_renewal_provisioner_list_result(
    json: bool,
    result: &AuthRenewalProvisionerListResult,
) -> Result<(), AuthCommandError> {
    if json {
        println!("{}", serde_json::to_string_pretty(result)?);
    } else if result.provisioners.is_empty() {
        println!("No delegation renewal provisioners configured.");
    } else {
        println!("{}", render_renewal_provisioner_list_result(result));
    }
    Ok(())
}

fn write_renewal_provisioner_upsert_result(
    json: bool,
    result: &AuthRenewalProvisionerUpsertResult,
) -> Result<(), AuthCommandError> {
    if json {
        println!("{}", serde_json::to_string_pretty(result)?);
    } else {
        println!("{}", render_renewal_provisioner_upsert_result(result));
    }
    Ok(())
}

fn render_renewal_provisioner_list_result(result: &AuthRenewalProvisionerListResult) -> String {
    let mut lines = vec![
        format!("Auth renewal provisioners: {}", result.deployment),
        format!("Root: {}", result.target.canister_id),
    ];
    for provisioner in &result.provisioners {
        lines.push(format!(
            "{} {}",
            provisioner.principal,
            render_enabled(provisioner.enabled)
        ));
    }
    lines.join("\n")
}

fn render_renewal_provisioner_upsert_result(result: &AuthRenewalProvisionerUpsertResult) -> String {
    format!(
        "Auth renewal provisioner {} {} for {}.",
        result.provisioner.principal,
        render_enabled(result.provisioner.enabled),
        result.deployment
    )
}

fn render_renewal_status_result(result: &AuthRenewalStatusResult) -> String {
    let mut lines = vec![
        format!("Auth renewal status: {}", result.issuer_pid),
        format!("Deployment: {}", result.deployment),
        format!("Root: {}", result.target.canister_id),
        format!("Status: {}", result.status),
        format!(
            "Template: {}",
            render_template_status(&result.renewal.template)
        ),
    ];
    if result.renewal.state.present {
        lines.push(format!(
            "Last outcome: {}",
            result.renewal.state.last_outcome.as_deref().unwrap_or("-")
        ));
        lines.push(format!(
            "Consecutive failures: {}",
            result
                .renewal
                .state
                .consecutive_failures
                .map_or_else(|| "-".to_string(), |value| value.to_string())
        ));
        lines.push(format!(
            "Last installed expires: {}",
            result
                .renewal
                .state
                .last_installed_expires_at_ns
                .as_deref()
                .unwrap_or("-")
        ));
        lines.push(format!(
            "Refresh after: {}",
            result
                .renewal
                .state
                .last_installed_refresh_after_ns
                .as_deref()
                .unwrap_or("-")
        ));
        lines.push(format!(
            "Next attempt after: {}",
            result
                .renewal
                .state
                .next_attempt_after_ns
                .as_deref()
                .unwrap_or("-")
        ));
    }
    lines.push(format!(
        "Active attempt: {}",
        render_active_attempt_status(&result.renewal.active_attempt)
    ));
    if result.renewal.active_attempt.present {
        lines.push(format!(
            "Batch: {}",
            result
                .renewal
                .active_attempt
                .batch_id
                .as_deref()
                .unwrap_or("-")
        ));
        if let Some(failure) = &result.renewal.active_attempt.failure {
            lines.push(format!("Failure: {failure}"));
        }
    }
    lines.push(format!(
        "Issuer observation: {}",
        render_issuer_observation(&result.issuer_observation)
    ));
    if result.issuer_observation.available {
        lines.push(format!(
            "Issuer cert hash: {}",
            result
                .issuer_observation
                .cert_hash
                .as_deref()
                .unwrap_or("-")
        ));
        lines.push(format!(
            "Issuer expires: {}",
            result
                .issuer_observation
                .expires_at_ns
                .as_deref()
                .unwrap_or("-")
        ));
    } else if let Some(reason) = &result.issuer_observation.reason {
        lines.push(format!("Issuer observation reason: {reason}"));
    }
    lines.join("\n")
}

const fn render_enabled(enabled: bool) -> &'static str {
    if enabled { "enabled" } else { "disabled" }
}

const fn render_template_status(template: &AuthRenewalTemplateStatus) -> &'static str {
    match (template.present, template.enabled) {
        (false, _) => "missing",
        (true, Some(true)) => "enabled",
        (true, Some(false)) => "disabled",
        (true, None) => "configured",
    }
}

fn render_active_attempt_status(attempt: &AuthRenewalActiveAttemptStatus) -> &str {
    if attempt.present {
        attempt.status.as_deref().unwrap_or("present")
    } else {
        "none"
    }
}

fn render_issuer_observation(observation: &AuthIssuerObservation) -> String {
    if observation.drift_detected {
        format!(
            "{} ({})",
            AUTH_RENEWAL_STATUS_DRIFT_DETECTED, observation.status
        )
    } else {
        observation.status.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{cli::globals, run};
    use std::{cell::RefCell, collections::VecDeque};

    #[test]
    fn parses_renewal_run_once_options() {
        let command = AuthOptions::parse([
            OsString::from("renewal"),
            OsString::from("run-once"),
            OsString::from("local"),
            OsString::from("--json"),
            OsString::from(globals::INTERNAL_NETWORK_OPTION),
            OsString::from("local"),
            OsString::from(globals::INTERNAL_ICP_OPTION),
            OsString::from("/bin/icp"),
        ])
        .expect("parse auth renewal run-once options");

        let AuthCommand::RenewalRunOnce(options) = command else {
            panic!("expected renewal run-once command");
        };
        assert_eq!(options.deployment, "local");
        assert_eq!(options.common.network, "local");
        assert_eq!(options.common.icp, "/bin/icp");
        assert!(options.json);
    }

    #[test]
    fn parses_renewal_status_options() {
        let command = AuthOptions::parse([
            OsString::from("renewal"),
            OsString::from("status"),
            OsString::from("local"),
            OsString::from("--issuer"),
            OsString::from("rrkah-fqaaa-aaaaa-aaaaq-cai"),
            OsString::from("--json"),
            OsString::from(globals::INTERNAL_NETWORK_OPTION),
            OsString::from("local"),
            OsString::from(globals::INTERNAL_ICP_OPTION),
            OsString::from("/bin/icp"),
        ])
        .expect("parse auth renewal status options");

        let AuthCommand::RenewalStatus(options) = command else {
            panic!("expected renewal status command");
        };
        assert_eq!(options.deployment, "local");
        assert_eq!(options.issuer, "rrkah-fqaaa-aaaaa-aaaaq-cai");
        assert_eq!(options.common.network, "local");
        assert_eq!(options.common.icp, "/bin/icp");
        assert!(options.json);
    }

    #[test]
    fn parses_renewal_provisioner_options() {
        let list = AuthOptions::parse([
            OsString::from("renewal"),
            OsString::from("provisioner"),
            OsString::from("list"),
            OsString::from("local"),
            OsString::from("--json"),
            OsString::from(globals::INTERNAL_NETWORK_OPTION),
            OsString::from("local"),
            OsString::from(globals::INTERNAL_ICP_OPTION),
            OsString::from("/bin/icp"),
        ])
        .expect("parse auth renewal provisioner list options");
        let AuthCommand::RenewalProvisionerList(options) = list else {
            panic!("expected renewal provisioner list command");
        };
        assert_eq!(options.deployment, "local");
        assert_eq!(options.common.network, "local");
        assert_eq!(options.common.icp, "/bin/icp");
        assert!(options.json);

        let disable = AuthOptions::parse([
            OsString::from("renewal"),
            OsString::from("provisioner"),
            OsString::from("disable"),
            OsString::from("local"),
            OsString::from("rrkah-fqaaa-aaaaa-aaaaq-cai"),
        ])
        .expect("parse auth renewal provisioner disable options");
        let AuthCommand::RenewalProvisionerUpsert(options) = disable else {
            panic!("expected renewal provisioner upsert command");
        };
        assert_eq!(options.deployment, "local");
        assert_eq!(options.principal, "rrkah-fqaaa-aaaaa-aaaaq-cai");
        assert!(!options.enabled);
    }

    #[test]
    fn top_level_forwards_auth_global_icp_and_network() {
        let err = run([
            OsString::from("--icp"),
            OsString::from("/bin/icp"),
            OsString::from("--network"),
            OsString::from("local"),
            OsString::from("auth"),
            OsString::from("renewal"),
            OsString::from("run-once"),
        ])
        .expect_err("missing deployment should be parsed after global options");

        assert!(err.to_string().contains("Usage: canic auth"));
    }

    #[test]
    fn parses_work_batches_from_json_and_candid() {
        let json = serde_json::json!({
            "batches": [{
                "batch_id": vec![7_u8; 32],
                "attempt_count": "2",
                "attempts": []
            }]
        })
        .to_string();
        assert_eq!(
            parse_work_batches(&json),
            Some(vec![AuthRenewalBatchWork {
                batch_id: [7; 32],
                attempt_count: Some(2),
            }])
        );

        let candid = r#"{"response_candid":"(record { batches = vec { record { batch_id = blob \"\\08\\08\\08\\08\\08\\08\\08\\08\\08\\08\\08\\08\\08\\08\\08\\08\\08\\08\\08\\08\\08\\08\\08\\08\\08\\08\\08\\08\\08\\08\\08\\08\"; attempt_count = 1 : nat64; attempts = vec {} } } })"}"#;
        assert_eq!(
            parse_work_batches(candid),
            Some(vec![AuthRenewalBatchWork {
                batch_id: [8; 32],
                attempt_count: Some(1),
            }])
        );
    }

    #[test]
    fn parses_renewal_provisioners_from_json_and_candid() {
        let json = serde_json::json!({
            "provisioners": [{
                "principal": "rrkah-fqaaa-aaaaa-aaaaq-cai",
                "enabled": true
            }]
        })
        .to_string();
        assert_eq!(
            parse_renewal_provisioners(&json),
            Some(vec![AuthRenewalProvisioner {
                principal: "rrkah-fqaaa-aaaaa-aaaaq-cai".to_string(),
                enabled: true,
            }])
        );

        let candid = r#"{"response_candid":"(record { provisioners = vec { record { \"principal\" = principal \"rrkah-fqaaa-aaaaa-aaaaq-cai\"; enabled = false } } })"}"#;
        assert_eq!(
            parse_renewal_provisioners(candid),
            Some(vec![AuthRenewalProvisioner {
                principal: "rrkah-fqaaa-aaaaa-aaaaq-cai".to_string(),
                enabled: false,
            }])
        );
    }

    #[test]
    fn run_once_retrieves_and_installs_scheduled_batches() {
        let runtime = ScriptedAuthRenewalRuntime::new([
            scripted_response(
                CANIC_DELEGATION_RENEWAL_WORK,
                None,
                Some("json"),
                serde_json::json!({
                    "batches": [{
                        "batch_id": vec![9_u8; 32],
                        "attempt_count": 1,
                        "attempts": []
                    }]
                })
                .to_string(),
            ),
            scripted_response(
                CANIC_GET_DELEGATION_RENEWAL_PROOF_BATCH,
                Some(root_delegation_renewal_batch_get_arg([9; 32])),
                None,
                "(record { batch_id = blob \"\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\"; proofs = vec {} })".to_string(),
            ),
            scripted_response(
                CANIC_INSTALL_DELEGATION_PROOF_BATCH,
                Some("(record { batch_id = blob \"\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\\09\"; proofs = vec {} })".to_string()),
                Some("json"),
                "{}".to_string(),
            ),
        ]);
        let result = renewal_once_result_with_runtime(
            &runtime,
            &RenewalRunOnceOptions {
                deployment: "local".to_string(),
                json: true,
                common: CommonOptions {
                    network: "local".to_string(),
                    icp: "icp".to_string(),
                },
            },
        )
        .expect("run-once should retrieve and install scripted batch");

        assert_eq!(result.status, AUTH_RENEWAL_STATUS_INSTALLED);
        assert_eq!(result.schema_version, AUTH_RENEWAL_RUN_ONCE_SCHEMA_VERSION);
        assert_eq!(result.batches.len(), 1);
        assert_eq!(result.batches[0].batch_id, hex_bytes(&[9; 32]));
        assert_eq!(
            runtime.called_methods(),
            vec![
                CANIC_DELEGATION_RENEWAL_WORK,
                CANIC_GET_DELEGATION_RENEWAL_PROOF_BATCH,
                CANIC_INSTALL_DELEGATION_PROOF_BATCH,
            ]
        );
    }

    #[test]
    fn run_once_noops_when_no_work_is_scheduled() {
        let runtime = ScriptedAuthRenewalRuntime::new([scripted_response(
            CANIC_DELEGATION_RENEWAL_WORK,
            None,
            Some("json"),
            serde_json::json!({ "batches": [] }).to_string(),
        )]);
        let result = renewal_once_result_with_runtime(
            &runtime,
            &RenewalRunOnceOptions {
                deployment: "local".to_string(),
                json: false,
                common: CommonOptions {
                    network: "local".to_string(),
                    icp: "icp".to_string(),
                },
            },
        )
        .expect("run-once should tolerate empty work");

        assert_eq!(result.status, AUTH_RENEWAL_STATUS_NO_WORK);
        assert!(result.batches.is_empty());
        assert_eq!(
            runtime.called_methods(),
            vec![CANIC_DELEGATION_RENEWAL_WORK]
        );
    }

    #[test]
    fn renewal_provisioner_list_queries_acl_endpoint() {
        let runtime = ScriptedAuthRenewalRuntime::new([scripted_response(
            CANIC_DELEGATION_RENEWAL_PROVISIONERS,
            None,
            Some("json"),
            serde_json::json!({
                "provisioners": [{
                    "principal": "rrkah-fqaaa-aaaaa-aaaaq-cai",
                    "enabled": true
                }]
            })
            .to_string(),
        )]);

        let result = renewal_provisioner_list_result_with_runtime(
            &runtime,
            &RenewalProvisionerListOptions {
                deployment: "local".to_string(),
                json: true,
                common: CommonOptions {
                    network: "local".to_string(),
                    icp: "icp".to_string(),
                },
            },
        )
        .expect("provisioner list should query scripted endpoint");

        assert_eq!(result.kind, AUTH_RENEWAL_PROVISIONER_LIST_KIND);
        assert_eq!(result.provisioners.len(), 1);
        assert_eq!(
            result.provisioners[0].principal,
            "rrkah-fqaaa-aaaaa-aaaaq-cai"
        );
        assert!(result.provisioners[0].enabled);
        assert_eq!(
            runtime.called_methods(),
            vec![CANIC_DELEGATION_RENEWAL_PROVISIONERS]
        );
    }

    #[test]
    fn renewal_provisioner_upsert_calls_acl_endpoint() {
        let principal = "rrkah-fqaaa-aaaaa-aaaaq-cai";
        let runtime = ScriptedAuthRenewalRuntime::new([scripted_response(
            CANIC_UPSERT_DELEGATION_RENEWAL_PROVISIONER,
            Some(renewal_provisioner_upsert_arg(principal, true)),
            Some("json"),
            serde_json::json!({
                "provisioner": {
                    "principal": principal,
                    "enabled": true
                }
            })
            .to_string(),
        )]);

        let result = renewal_provisioner_upsert_result_with_runtime(
            &runtime,
            &RenewalProvisionerUpsertOptions {
                deployment: "local".to_string(),
                principal: principal.to_string(),
                enabled: true,
                json: true,
                common: CommonOptions {
                    network: "local".to_string(),
                    icp: "icp".to_string(),
                },
            },
        )
        .expect("provisioner upsert should call scripted endpoint");

        assert_eq!(result.kind, AUTH_RENEWAL_PROVISIONER_UPSERT_KIND);
        assert_eq!(result.provisioner.principal, principal);
        assert!(result.provisioner.enabled);
        assert_eq!(
            runtime.called_methods(),
            vec![CANIC_UPSERT_DELEGATION_RENEWAL_PROVISIONER]
        );
    }

    #[test]
    fn renewal_status_queries_root_status_endpoint() {
        let issuer = "rrkah-fqaaa-aaaaa-aaaaq-cai";
        let runtime = ScriptedAuthRenewalRuntime::new([scripted_response(
            CANIC_ROOT_ISSUER_RENEWAL_STATUS,
            Some(root_issuer_renewal_status_arg(issuer)),
            Some("json"),
            serde_json::json!({
                "template": {
                    "enabled": true,
                    "cert_ttl_ns": "300000000000"
                },
                "state": {
                    "last_outcome": "Installed",
                    "consecutive_failures": 0,
                    "last_installed_expires_at_ns": ["1620329000000000000"],
                    "last_installed_refresh_after_ns": ["1620328900000000000"],
                    "next_attempt_after_ns": "1620328900000000000",
                    "active_attempt_id": [vec![1_u8; 32]]
                },
                "active_attempt": {
                    "status": "Prepared",
                    "batch_id": vec![2_u8; 32],
                    "prepared_expires_at_ns": "1620329000000000000",
                    "failure": null
                }
            })
            .to_string(),
        )]);
        let result = renewal_status_result_with_runtime(
            &runtime,
            &RenewalStatusOptions {
                deployment: "local".to_string(),
                issuer: issuer.to_string(),
                json: true,
                common: CommonOptions {
                    network: "local".to_string(),
                    icp: "icp".to_string(),
                },
            },
        )
        .expect("status should query scripted endpoint");

        assert_eq!(result.kind, AUTH_RENEWAL_STATUS_KIND);
        assert_eq!(result.schema_version, AUTH_RENEWAL_STATUS_SCHEMA_VERSION);
        assert_eq!(result.issuer_pid, issuer);
        assert_eq!(result.status, AUTH_RENEWAL_STATUS_ACTIVE_ATTEMPT);
        assert_eq!(result.renewal.template.enabled, Some(true));
        assert_eq!(
            result.issuer_observation.status,
            AUTH_RENEWAL_STATUS_UNAVAILABLE
        );
        assert_eq!(
            result.issuer_observation.reason.as_deref(),
            Some("issuer_not_in_local_registry")
        );
        assert_eq!(
            result.renewal.state.last_outcome.as_deref(),
            Some("installed")
        );
        assert_eq!(
            result.renewal.active_attempt.status.as_deref(),
            Some("prepared")
        );
        assert_eq!(
            runtime.called_methods(),
            vec![CANIC_ROOT_ISSUER_RENEWAL_STATUS]
        );
    }

    #[test]
    fn renewal_status_reports_matching_issuer_observation() {
        let issuer = "rrkah-fqaaa-aaaaa-aaaaq-cai";
        let runtime = ScriptedAuthRenewalRuntime::new([
            scripted_response(
                CANIC_ROOT_ISSUER_RENEWAL_STATUS,
                Some(root_issuer_renewal_status_arg(issuer)),
                Some("json"),
                renewal_status_response_json(issuer, [3; 32], "1620329000000000000"),
            ),
            scripted_response(
                CANIC_ACTIVE_DELEGATION_PROOF_STATUS,
                None,
                Some("json"),
                issuer_status_response_json([3; 32], "1620329000000000000"),
            ),
        ])
        .with_issuer_available();

        let result = renewal_status_result_with_runtime(&runtime, &renewal_status_options(issuer))
            .expect("status should include issuer observation");

        assert_eq!(result.status, AUTH_RENEWAL_STATUS_CONFIGURED);
        assert!(result.issuer_observation.available);
        assert!(!result.issuer_observation.drift_detected);
        assert_eq!(
            result.issuer_observation.cert_hash,
            Some(hex_bytes(&[3; 32]))
        );
        assert_eq!(
            runtime.called_methods(),
            vec![
                CANIC_ROOT_ISSUER_RENEWAL_STATUS,
                CANIC_ACTIVE_DELEGATION_PROOF_STATUS,
            ]
        );
    }

    #[test]
    fn renewal_status_reports_root_issuer_drift() {
        let issuer = "rrkah-fqaaa-aaaaa-aaaaq-cai";
        let runtime = ScriptedAuthRenewalRuntime::new([
            scripted_response(
                CANIC_ROOT_ISSUER_RENEWAL_STATUS,
                Some(root_issuer_renewal_status_arg(issuer)),
                Some("json"),
                renewal_status_response_json(issuer, [3; 32], "1620329000000000000"),
            ),
            scripted_response(
                CANIC_ACTIVE_DELEGATION_PROOF_STATUS,
                None,
                Some("json"),
                issuer_status_response_json([4; 32], "1620329000000000000"),
            ),
        ])
        .with_issuer_available();

        let result = renewal_status_result_with_runtime(&runtime, &renewal_status_options(issuer))
            .expect("status should include drift observation");
        let rendered = render_renewal_status_result(&result);

        assert_eq!(result.status, AUTH_RENEWAL_STATUS_DRIFT_DETECTED);
        assert!(result.issuer_observation.drift_detected);
        assert!(rendered.contains("Issuer observation: drift_detected"));
        assert!(rendered.contains(&hex_bytes(&[4; 32])));
    }

    #[test]
    fn renewal_status_rejects_invalid_issuer_principal() {
        let runtime = ScriptedAuthRenewalRuntime::empty();
        let err = renewal_status_result_with_runtime(
            &runtime,
            &RenewalStatusOptions {
                deployment: "local".to_string(),
                issuer: "not a principal".to_string(),
                json: false,
                common: CommonOptions {
                    network: "local".to_string(),
                    icp: "icp".to_string(),
                },
            },
        )
        .expect_err("invalid issuer principal should fail before transport");

        assert!(matches!(
            err,
            AuthCommandError::InvalidIssuerPrincipal { .. }
        ));
        assert!(runtime.called_methods().is_empty());
    }

    fn renewal_status_options(issuer: &str) -> RenewalStatusOptions {
        RenewalStatusOptions {
            deployment: "local".to_string(),
            issuer: issuer.to_string(),
            json: true,
            common: CommonOptions {
                network: "local".to_string(),
                icp: "icp".to_string(),
            },
        }
    }

    fn renewal_status_response_json(
        _issuer: &str,
        cert_hash: [u8; 32],
        expires_at_ns: &str,
    ) -> String {
        serde_json::json!({
            "template": {
                "enabled": true,
                "cert_ttl_ns": "300000000000"
            },
            "state": {
                "last_installed_cert_hash": [cert_hash.to_vec()],
                "last_outcome": "Installed",
                "consecutive_failures": 0,
                "last_installed_expires_at_ns": [expires_at_ns],
                "last_installed_refresh_after_ns": ["1620328900000000000"],
                "next_attempt_after_ns": "1620328900000000000",
                "active_attempt_id": null
            },
            "active_attempt": null
        })
        .to_string()
    }

    fn issuer_status_response_json(cert_hash: [u8; 32], expires_at_ns: &str) -> String {
        serde_json::json!({
            "status": "Valid",
            "root_pid": ["r7inp-6aaaa-aaaaa-aaabq-cai"],
            "issuer_pid": ["rrkah-fqaaa-aaaaa-aaaaq-cai"],
            "cert_hash": [cert_hash.to_vec()],
            "expires_at_ns": [expires_at_ns],
            "refresh_after_ns": ["1620328900000000000"]
        })
        .to_string()
    }

    struct ScriptedAuthRenewalRuntime {
        responses: RefCell<VecDeque<ScriptedAuthRenewalResponse>>,
        calls: RefCell<Vec<String>>,
        issuer_available: bool,
    }

    impl ScriptedAuthRenewalRuntime {
        fn empty() -> Self {
            Self {
                responses: RefCell::new(VecDeque::new()),
                calls: RefCell::new(Vec::new()),
                issuer_available: false,
            }
        }

        fn new<const N: usize>(responses: [ScriptedAuthRenewalResponse; N]) -> Self {
            Self {
                responses: RefCell::new(VecDeque::from(responses)),
                calls: RefCell::new(Vec::new()),
                issuer_available: false,
            }
        }

        fn with_issuer_available(mut self) -> Self {
            self.issuer_available = true;
            self
        }

        fn called_methods(&self) -> Vec<&'static str> {
            self.calls
                .borrow()
                .iter()
                .map(String::as_str)
                .map(|method| match method {
                    CANIC_DELEGATION_RENEWAL_WORK => CANIC_DELEGATION_RENEWAL_WORK,
                    CANIC_GET_DELEGATION_RENEWAL_PROOF_BATCH => {
                        CANIC_GET_DELEGATION_RENEWAL_PROOF_BATCH
                    }
                    CANIC_INSTALL_DELEGATION_PROOF_BATCH => CANIC_INSTALL_DELEGATION_PROOF_BATCH,
                    CANIC_ROOT_ISSUER_RENEWAL_STATUS => CANIC_ROOT_ISSUER_RENEWAL_STATUS,
                    CANIC_ACTIVE_DELEGATION_PROOF_STATUS => CANIC_ACTIVE_DELEGATION_PROOF_STATUS,
                    CANIC_DELEGATION_RENEWAL_PROVISIONERS => CANIC_DELEGATION_RENEWAL_PROVISIONERS,
                    CANIC_UPSERT_DELEGATION_RENEWAL_PROVISIONER => {
                        CANIC_UPSERT_DELEGATION_RENEWAL_PROVISIONER
                    }
                    _ => panic!("unexpected method {method}"),
                })
                .collect()
        }
    }

    impl AuthRenewalRuntime for ScriptedAuthRenewalRuntime {
        fn resolve_root_target(
            &self,
            _options: &CommonOptions,
            _deployment: &str,
            _method: &str,
            _expected_mode: AuthRenewalMethodMode,
        ) -> Result<AuthRootCallTarget, AuthCommandError> {
            Ok(AuthRootCallTarget {
                target: AuthRootTarget {
                    input: ROOT_ROLE.to_string(),
                    role: ROOT_ROLE.to_string(),
                    canister_id: "rrkah-fqaaa-aaaaa-aaaaq-cai".to_string(),
                    candid_source: AUTH_RENEWAL_CANDID_SOURCE_INSTALLED_DEPLOYMENT.to_string(),
                },
                candid_path: PathBuf::from(".icp/local/canisters/root/root.did"),
                icp_root: PathBuf::from("."),
                registry_entries: if self.issuer_available {
                    vec![RegistryEntry {
                        pid: "rrkah-fqaaa-aaaaa-aaaaq-cai".to_string(),
                        role: Some("issuer".to_string()),
                        kind: None,
                        parent_pid: None,
                        module_hash: None,
                    }]
                } else {
                    Vec::new()
                },
            })
        }

        fn query_output(
            &self,
            _options: &CommonOptions,
            _target: &AuthRootCallTarget,
            method: &str,
            arg: Option<&str>,
            output: Option<&str>,
        ) -> Result<String, AuthCommandError> {
            Ok(self.call(method, arg, output))
        }

        fn call_output(
            &self,
            _options: &CommonOptions,
            _target: &AuthRootCallTarget,
            method: &str,
            arg: &str,
            output: Option<&str>,
        ) -> Result<String, AuthCommandError> {
            Ok(self.call(method, Some(arg), output))
        }

        fn resolve_issuer_target(
            &self,
            _options: &CommonOptions,
            root_target: &AuthRootCallTarget,
            issuer_pid: &str,
            _method: &str,
            _expected_mode: AuthRenewalMethodMode,
        ) -> Result<Option<AuthIssuerCallTarget>, AuthCommandError> {
            if root_target
                .registry_entries
                .iter()
                .any(|entry| entry.pid == issuer_pid)
            {
                Ok(Some(AuthIssuerCallTarget {
                    target: AuthIssuerTarget {
                        input: issuer_pid.to_string(),
                        role: Some("issuer".to_string()),
                        canister_id: issuer_pid.to_string(),
                        candid_source: AUTH_RENEWAL_CANDID_SOURCE_INSTALLED_DEPLOYMENT.to_string(),
                    },
                    candid_path: PathBuf::from(".icp/local/canisters/issuer/issuer.did"),
                    icp_root: PathBuf::from("."),
                }))
            } else {
                Ok(None)
            }
        }

        fn query_issuer_output(
            &self,
            _options: &CommonOptions,
            _target: &AuthIssuerCallTarget,
            method: &str,
            output: Option<&str>,
        ) -> Result<String, AuthCommandError> {
            Ok(self.call(method, None, output))
        }
    }

    impl ScriptedAuthRenewalRuntime {
        fn call(&self, method: &str, arg: Option<&str>, output: Option<&str>) -> String {
            self.calls.borrow_mut().push(method.to_string());
            let response = self
                .responses
                .borrow_mut()
                .pop_front()
                .expect("scripted response");

            assert_eq!(response.method, method);
            assert_eq!(response.arg.as_deref(), arg);
            assert_eq!(response.output, output);
            response.body
        }
    }

    struct ScriptedAuthRenewalResponse {
        method: &'static str,
        arg: Option<String>,
        output: Option<&'static str>,
        body: String,
    }

    fn scripted_response(
        method: &'static str,
        arg: Option<String>,
        output: Option<&'static str>,
        body: String,
    ) -> ScriptedAuthRenewalResponse {
        ScriptedAuthRenewalResponse {
            method,
            arg,
            output,
            body,
        }
    }
}
