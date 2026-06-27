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
use canic_core::protocol::{
    CANIC_DELEGATION_RENEWAL_WORK, CANIC_GET_DELEGATION_RENEWAL_PROOF_BATCH,
    CANIC_INSTALL_DELEGATION_PROOF_BATCH,
};
use canic_host::{
    candid_endpoints::{CandidEndpointError, EndpointMode, parse_candid_service_endpoints},
    icp::IcpCli,
    icp_config::resolve_current_canic_icp_root,
    installed_deployment::{
        InstalledDeploymentError, InstalledDeploymentRequest,
        resolve_installed_deployment_from_root,
    },
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
const DEPLOYMENT_ARG: &str = "deployment";
const JSON_ARG: &str = "json";
const ROOT_ROLE: &str = "root";
const AUTH_RENEWAL_SCHEMA_VERSION: u16 = 1;
const AUTH_RENEWAL_RUN_ONCE_KIND: &str = "auth_renewal_run_once_result";
const AUTH_RENEWAL_STATUS_NO_WORK: &str = "no_work";
const AUTH_RENEWAL_STATUS_INSTALLED: &str = "installed";
const AUTH_RENEWAL_CANDID_SOURCE_INSTALLED_DEPLOYMENT: &str = "installed_deployment";

const HELP_AFTER: &str = "\
Examples:
  canic auth renewal run-once local
  canic auth renewal run-once local --json";

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

fn run_command(command: AuthCommand) -> Result<(), AuthCommandError> {
    match command {
        AuthCommand::RenewalRunOnce(options) => run_renewal_once(&options),
    }
}

fn run_renewal_once(options: &RenewalRunOnceOptions) -> Result<(), AuthCommandError> {
    let runtime = LiveAuthRenewalRuntime;
    let result = renewal_once_result_with_runtime(&runtime, options)?;
    write_renewal_once_result(options.json, &result)
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

    fn call_output(
        &self,
        options: &CommonOptions,
        target: &AuthRootCallTarget,
        method: &str,
        arg: &str,
        output: Option<&str>,
    ) -> Result<String, AuthCommandError>;
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
        schema_version: AUTH_RENEWAL_SCHEMA_VERSION,
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
    })
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

        let AuthCommand::RenewalRunOnce(options) = command;
        assert_eq!(options.deployment, "local");
        assert_eq!(options.common.network, "local");
        assert_eq!(options.common.icp, "/bin/icp");
        assert!(options.json);
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

    struct ScriptedAuthRenewalRuntime {
        responses: RefCell<VecDeque<ScriptedAuthRenewalResponse>>,
        calls: RefCell<Vec<String>>,
    }

    impl ScriptedAuthRenewalRuntime {
        fn new<const N: usize>(responses: [ScriptedAuthRenewalResponse; N]) -> Self {
            Self {
                responses: RefCell::new(VecDeque::from(responses)),
                calls: RefCell::new(Vec::new()),
            }
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
            self.call(method, arg, output)
        }

        fn call_output(
            &self,
            _options: &CommonOptions,
            _target: &AuthRootCallTarget,
            method: &str,
            arg: &str,
            output: Option<&str>,
        ) -> Result<String, AuthCommandError> {
            self.call(method, Some(arg), output)
        }
    }

    impl ScriptedAuthRenewalRuntime {
        fn call(
            &self,
            method: &str,
            arg: Option<&str>,
            output: Option<&str>,
        ) -> Result<String, AuthCommandError> {
            self.calls.borrow_mut().push(method.to_string());
            let response = self
                .responses
                .borrow_mut()
                .pop_front()
                .expect("scripted response");

            assert_eq!(response.method, method);
            assert_eq!(response.arg.as_deref(), arg);
            assert_eq!(response.output, output);
            Ok(response.body)
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
