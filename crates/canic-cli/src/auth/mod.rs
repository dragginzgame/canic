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
    hex_bytes, parse_issuer_observed_status, parse_issuer_principal, parse_principal_text,
    parse_renewal_provisioner_response, parse_renewal_provisioners, parse_renewal_status_summary,
    parse_work_batches, renewal_provisioner_upsert_arg, root_delegation_renewal_batch_get_arg,
    root_issuer_renewal_status_arg,
};
use render::{
    render_issuer_observation, write_renewal_once_result, write_renewal_provisioner_list_result,
    write_renewal_provisioner_upsert_result, write_renewal_status_result,
};

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
