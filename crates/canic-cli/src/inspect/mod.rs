//! Module: canic_cli::inspect
//!
//! Responsibility: inspect one deployed canister's runtime-observed Canic status.
//! Does not own: deployment planning, runtime endpoint DTOs, or broad topology fanout.
//! Boundary: resolves one explicit target, queries `canic_runtime_status`, and renders a report.

use crate::{
    cli::{
        clap::{flag_arg, parse_matches, render_usage, required_string, string_option_or_else},
        defaults::{default_icp, local_network},
        globals::{internal_icp_arg, internal_network_arg},
        help::print_help_or_version,
    },
    support::candid::registry_entry_candid_path,
    version_text,
};
use candid::{Decode, Principal, types::principal::PrincipalError};
use canic_core::dto::runtime::{CanicRuntimeStatus, RuntimeStatus};
use canic_host::{
    icp::{IcpCli, IcpCommandError},
    icp_config::resolve_current_canic_icp_root,
    installed_deployment::{
        InstalledDeploymentError, InstalledDeploymentRequest,
        resolve_installed_deployment_from_root,
    },
    response_parse::response_candid,
};
use clap::{Arg, Command as ClapCommand};
use serde::Serialize;
use std::{ffi::OsString, path::PathBuf};
use thiserror::Error as ThisError;

const INSPECT_SCHEMA_VERSION: u32 = 1;
const RUNTIME_OBSERVED_SOURCE: &str = "runtime_observed";
const CLI_ARG_SOURCE: &str = "cli_arg";
const DEPLOYMENT_RECORD_SOURCE: &str = "deployment_record";

const INSPECT_HELP_AFTER: &str = "\
Examples:
  canic inspect canister aaaaa-aa
  canic inspect canister aaaaa-aa --json
  canic inspect deployment demo-local --role root
  canic inspect deployment demo-local --role root --json

Inspect is read-only. It queries the guarded canic_runtime_status endpoint for
one explicit target and does not fan out across deployment roles. Use
`canic deploy inspect` for local deployment-truth artifacts and saved reports.";

#[derive(Debug, ThisError)]
pub enum InspectCommandError {
    #[error("{0}")]
    Usage(String),

    #[error("invalid canister principal {value}: {source}")]
    InvalidPrincipal {
        value: String,
        source: PrincipalError,
    },

    #[error("{0}")]
    Target(String),

    #[error("icp command failed: {0}")]
    Icp(#[from] IcpCommandError),

    #[error("invalid canic_runtime_status response: {0}")]
    InvalidResponse(String),

    #[error("runtime status reported {0}")]
    ReportStatus(String),

    #[error("failed to resolve ICP project root: {0}")]
    IcpRoot(String),

    #[error("failed to render inspect JSON: {0}")]
    Json(#[from] serde_json::Error),
}

impl InspectCommandError {
    #[must_use]
    pub const fn exit_code(&self) -> u8 {
        match self {
            Self::Usage(_)
            | Self::InvalidPrincipal { .. }
            | Self::Target(_)
            | Self::Icp(_)
            | Self::InvalidResponse(_)
            | Self::IcpRoot(_)
            | Self::Json(_) => 2,
            Self::ReportStatus(_) => 1,
        }
    }

    #[must_use]
    pub const fn suppress_stderr(&self) -> bool {
        matches!(self, Self::ReportStatus(_))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum InspectOptions {
    Canister {
        canister: String,
        network: String,
        icp: String,
        json: bool,
    },
    Deployment {
        deployment: String,
        role: String,
        network: String,
        icp: String,
        json: bool,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ResolvedInspectTarget {
    command: String,
    deployment: Option<String>,
    role: Option<String>,
    canister_id: String,
    network: String,
    icp: String,
    source: &'static str,
    candid_path: Option<PathBuf>,
    icp_root: Option<PathBuf>,
    json: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct InspectReport {
    schema_version: u32,
    command: String,
    target_resolution: TargetResolution,
    endpoint: String,
    status: String,
    health_status: Option<serde_json::Value>,
    readiness_status: Option<serde_json::Value>,
    runtime_status: Option<RuntimeStatusPayload>,
    warnings: Vec<String>,
    next_actions: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct TargetResolution {
    deployment: Option<String>,
    role: Option<String>,
    canister_id: String,
    network: String,
    source: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct RuntimeStatusPayload {
    source: String,
    status: Option<CanicRuntimeStatus>,
    response_format: String,
    response_bytes_present: bool,
    response_candid_present: bool,
}

pub fn run<I>(args: I) -> Result<(), InspectCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }
    if print_leaf_help_or_version(&args) {
        return Ok(());
    }

    let options = InspectOptions::parse(args)?;
    let target = resolve_target(&options)?;
    let report = inspect_report(&target)?;
    if target.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("{}", render_text_report(&report));
    }
    command_exit_result(&report)?;
    Ok(())
}

impl InspectOptions {
    fn parse<I>(args: I) -> Result<Self, InspectCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches =
            parse_matches(command(), args).map_err(|_| InspectCommandError::Usage(usage()))?;
        match matches.subcommand() {
            Some(("canister", matches)) => {
                let canister = required_string(matches, "canister");
                validate_principal(&canister)?;
                Ok(Self::Canister {
                    canister,
                    network: string_option_or_else(matches, "network", local_network),
                    icp: string_option_or_else(matches, "icp", default_icp),
                    json: matches.get_flag("json"),
                })
            }
            Some(("deployment", matches)) => Ok(Self::Deployment {
                deployment: required_string(matches, "deployment"),
                role: required_string(matches, "role"),
                network: string_option_or_else(matches, "network", local_network),
                icp: string_option_or_else(matches, "icp", default_icp),
                json: matches.get_flag("json"),
            }),
            _ => Err(InspectCommandError::Usage(usage())),
        }
    }
}

fn resolve_target(options: &InspectOptions) -> Result<ResolvedInspectTarget, InspectCommandError> {
    match options {
        InspectOptions::Canister {
            canister,
            network,
            icp,
            json,
        } => Ok(ResolvedInspectTarget {
            command: "canic inspect canister".to_string(),
            deployment: None,
            role: None,
            canister_id: canister.clone(),
            network: network.clone(),
            icp: icp.clone(),
            source: CLI_ARG_SOURCE,
            candid_path: None,
            icp_root: resolve_current_canic_icp_root().ok(),
            json: *json,
        }),
        InspectOptions::Deployment {
            deployment,
            role,
            network,
            icp,
            json,
        } => resolve_deployment_target(deployment, role, network, icp, *json),
    }
}

fn resolve_deployment_target(
    deployment: &str,
    role: &str,
    network: &str,
    icp: &str,
    json: bool,
) -> Result<ResolvedInspectTarget, InspectCommandError> {
    let root = resolve_current_canic_icp_root()
        .map_err(|err| InspectCommandError::IcpRoot(err.to_string()))?;
    let installed = resolve_installed_deployment_from_root(
        &InstalledDeploymentRequest {
            deployment: deployment.to_string(),
            network: network.to_string(),
            icp: icp.to_string(),
            detect_lost_local_root: false,
        },
        &root,
    )
    .map_err(installed_deployment_error)?;
    let matches = installed
        .registry
        .entries
        .iter()
        .filter(|entry| entry.role.as_deref() == Some(role))
        .collect::<Vec<_>>();

    let entry = match matches.as_slice() {
        [] => {
            return Err(InspectCommandError::Target(format!(
                "role {role} was not found in deployment target {deployment}"
            )));
        }
        [entry] => *entry,
        _ => {
            return Err(InspectCommandError::Target(format!(
                "role {role} resolves to multiple canisters in deployment target {deployment}; explicit disambiguation is not supported in 0.81"
            )));
        }
    };
    validate_principal(&entry.pid)?;

    Ok(ResolvedInspectTarget {
        command: "canic inspect deployment".to_string(),
        deployment: Some(deployment.to_string()),
        role: Some(role.to_string()),
        canister_id: entry.pid.clone(),
        network: network.to_string(),
        icp: icp.to_string(),
        source: DEPLOYMENT_RECORD_SOURCE,
        candid_path: registry_entry_candid_path(Some(root.as_path()), network, entry),
        icp_root: Some(root),
        json,
    })
}

fn inspect_report(target: &ResolvedInspectTarget) -> Result<InspectReport, InspectCommandError> {
    let mut icp = IcpCli::new(&target.icp, None, Some(target.network.clone()));
    if let Some(root) = &target.icp_root {
        icp = icp.with_cwd(root);
    }
    let output = icp.canister_query_output_with_candid(
        &target.canister_id,
        canic_core::protocol::CANIC_RUNTIME_STATUS,
        Some("json"),
        target.candid_path.as_deref(),
    )?;
    let runtime_status = runtime_response_payload(&output)?;
    let status = inspect_status_label(&runtime_status).to_string();

    Ok(InspectReport {
        schema_version: INSPECT_SCHEMA_VERSION,
        command: target.command.clone(),
        target_resolution: TargetResolution {
            deployment: target.deployment.clone(),
            role: target.role.clone(),
            canister_id: target.canister_id.clone(),
            network: target.network.clone(),
            source: target.source.to_string(),
        },
        endpoint: canic_core::protocol::CANIC_RUNTIME_STATUS.to_string(),
        status,
        health_status: None,
        readiness_status: None,
        runtime_status: Some(runtime_status),
        warnings: Vec::new(),
        next_actions: Vec::new(),
    })
}

fn runtime_response_payload(output: &str) -> Result<RuntimeStatusPayload, InspectCommandError> {
    let value = serde_json::from_str::<serde_json::Value>(output)
        .map_err(|err| InspectCommandError::InvalidResponse(err.to_string()))?;
    let response_candid_present = response_candid(&value).is_some();
    let response_bytes = value
        .get("response_bytes")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let response_bytes_present = response_bytes.is_some();

    if !response_bytes_present && !response_candid_present {
        return Err(InspectCommandError::InvalidResponse(
            "missing response_bytes and response_candid".to_string(),
        ));
    }
    let status = response_bytes
        .as_deref()
        .map(decode_runtime_status_response_hex)
        .transpose()?;

    Ok(RuntimeStatusPayload {
        source: RUNTIME_OBSERVED_SOURCE.to_string(),
        status,
        response_format: "candid".to_string(),
        response_bytes_present,
        response_candid_present,
    })
}

fn decode_runtime_status_response_hex(
    response_bytes: &str,
) -> Result<CanicRuntimeStatus, InspectCommandError> {
    let bytes = hex_to_bytes(response_bytes).ok_or_else(|| {
        InspectCommandError::InvalidResponse("response_bytes was not valid hex".to_string())
    })?;
    let response = Decode!(
        &bytes,
        Result<CanicRuntimeStatus, canic_core::dto::error::Error>
    )
    .map_err(|err| InspectCommandError::InvalidResponse(err.to_string()))?;
    response.map_err(|err| InspectCommandError::InvalidResponse(err.message))
}

fn command_exit_result(report: &InspectReport) -> Result<(), InspectCommandError> {
    let status = report
        .runtime_status
        .as_ref()
        .and_then(|payload| payload.status.as_ref())
        .map(|status| status.status);
    match status {
        Some(RuntimeStatus::Failing) => Err(InspectCommandError::ReportStatus(
            runtime_status_label(RuntimeStatus::Failing).to_string(),
        )),
        Some(RuntimeStatus::Ok | RuntimeStatus::Degraded | RuntimeStatus::Unknown) | None => Ok(()),
    }
}

fn render_text_report(report: &InspectReport) -> String {
    let mut lines = vec![
        report.command.clone(),
        format!("status: {}", report.status),
        format!("endpoint: {}", report.endpoint),
        format!("canister: {}", report.target_resolution.canister_id),
        format!("network: {}", report.target_resolution.network),
        format!("source: {}", report.target_resolution.source),
    ];
    if let Some(deployment) = &report.target_resolution.deployment {
        lines.push(format!("deployment: {deployment}"));
    }
    if let Some(role) = &report.target_resolution.role {
        lines.push(format!("role: {role}"));
    }
    if let Some(runtime_status) = &report.runtime_status {
        lines.extend([
            String::new(),
            "runtime_status".to_string(),
            format!("source: {}", runtime_status.source),
            format!("response_format: {}", runtime_status.response_format),
            format!(
                "response_bytes_present: {}",
                runtime_status.response_bytes_present
            ),
            format!(
                "response_candid_present: {}",
                runtime_status.response_candid_present
            ),
        ]);
        if let Some(status) = &runtime_status.status {
            lines.extend([
                format!("runtime_status: {}", runtime_status_label(status.status)),
                format!("schema_version: {}", status.schema_version),
                format!("observed_at_ns: {}", status.observed_at_ns),
                format!("role: {}", status.role.as_deref().unwrap_or("unknown")),
                format!("timers: {}", status.timers.len()),
                format!("recent_failures: {}", status.recent_failures.len()),
            ]);
            if let Some(state) = &status.state {
                lines.push(format!("state_domains: {}", state.domains.len()));
            }
        }
    }
    lines.join("\n")
}

fn inspect_status_label(payload: &RuntimeStatusPayload) -> &'static str {
    payload
        .status
        .as_ref()
        .map_or("unknown", |status| runtime_status_label(status.status))
}

const fn runtime_status_label(status: RuntimeStatus) -> &'static str {
    match status {
        RuntimeStatus::Ok => "ok",
        RuntimeStatus::Degraded => "degraded",
        RuntimeStatus::Failing => "failing",
        RuntimeStatus::Unknown => "unknown",
    }
}

fn hex_to_bytes(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    text.as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = hex_value(pair[0])?;
            let low = hex_value(pair[1])?;
            Some((high << 4) | low)
        })
        .collect()
}

const fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn validate_principal(value: &str) -> Result<(), InspectCommandError> {
    Principal::from_text(value).map(|_| ()).map_err(|source| {
        InspectCommandError::InvalidPrincipal {
            value: value.to_string(),
            source,
        }
    })
}

fn installed_deployment_error(error: InstalledDeploymentError) -> InspectCommandError {
    match error {
        InstalledDeploymentError::NoInstalledDeployment {
            network,
            deployment,
        } => InspectCommandError::Target(format!(
            "deployment target {deployment} is not installed on network {network}"
        )),
        InstalledDeploymentError::LostLocalDeployment { root, .. } => {
            InspectCommandError::Target(format!("root canister {root} is not present"))
        }
        InstalledDeploymentError::InstallState(error)
        | InstalledDeploymentError::ReplicaQuery(error) => InspectCommandError::Target(error),
        InstalledDeploymentError::Registry(error) => InspectCommandError::Target(error.to_string()),
        InstalledDeploymentError::IcpFailed { command, stderr } => InspectCommandError::Target(
            format!("failed to resolve deployment target via `{command}`: {stderr}"),
        ),
        InstalledDeploymentError::Io(error) => InspectCommandError::Target(error.to_string()),
    }
}

fn command() -> ClapCommand {
    ClapCommand::new("inspect")
        .bin_name("canic inspect")
        .about("Inspect runtime-observed status for one deployed Canic canister")
        .disable_help_flag(true)
        .subcommand_required(true)
        .subcommand(canister_command())
        .subcommand(deployment_command())
        .after_help(INSPECT_HELP_AFTER)
}

fn canister_command() -> ClapCommand {
    ClapCommand::new("canister")
        .about("Inspect one explicit canister principal")
        .disable_help_flag(true)
        .arg(
            Arg::new("canister")
                .value_name("principal")
                .num_args(1)
                .required(true),
        )
        .arg(internal_network_arg())
        .arg(internal_icp_arg())
        .arg(flag_arg("json").long("json").help("Print JSON output"))
}

fn deployment_command() -> ClapCommand {
    ClapCommand::new("deployment")
        .about("Inspect one role in an installed deployment target")
        .disable_help_flag(true)
        .arg(
            Arg::new("deployment")
                .value_name("deployment")
                .num_args(1)
                .required(true),
        )
        .arg(
            Arg::new("role")
                .long("role")
                .value_name("role")
                .num_args(1)
                .required(true),
        )
        .arg(internal_network_arg())
        .arg(internal_icp_arg())
        .arg(flag_arg("json").long("json").help("Print JSON output"))
}

fn usage() -> String {
    render_usage(command)
}

fn canister_usage() -> String {
    render_usage(canister_command)
}

fn deployment_usage() -> String {
    render_usage(deployment_command)
}

fn print_leaf_help_or_version(args: &[OsString]) -> bool {
    let Some(usage) = args
        .first()
        .and_then(|arg| arg.to_str())
        .and_then(|leaf| match leaf {
            "canister" => Some(canister_usage as fn() -> String),
            "deployment" => Some(deployment_usage as fn() -> String),
            _ => None,
        })
    else {
        return false;
    };
    let Some(arg) = args.get(1).and_then(|arg| arg.to_str()) else {
        return false;
    };
    match arg {
        "help" | "--help" | "-h" => {
            println!("{}", usage());
            true
        }
        "version" | "--version" | "-V" => {
            println!("{}", version_text());
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use candid::Encode;

    #[test]
    fn parses_direct_canister_target() {
        let options = InspectOptions::parse([
            OsString::from("canister"),
            OsString::from("aaaaa-aa"),
            OsString::from("--json"),
        ])
        .expect("parse canister inspect");

        assert_eq!(
            options,
            InspectOptions::Canister {
                canister: "aaaaa-aa".to_string(),
                network: local_network(),
                icp: default_icp(),
                json: true,
            }
        );
    }

    #[test]
    fn parses_deployment_role_target() {
        let options = InspectOptions::parse([
            OsString::from("deployment"),
            OsString::from("demo-local"),
            OsString::from("--role"),
            OsString::from("root"),
        ])
        .expect("parse deployment inspect");

        assert_eq!(
            options,
            InspectOptions::Deployment {
                deployment: "demo-local".to_string(),
                role: "root".to_string(),
                network: local_network(),
                icp: default_icp(),
                json: false,
            }
        );
    }

    #[test]
    fn rejects_ambiguous_target_form() {
        assert!(InspectOptions::parse([OsString::from("demo-local")]).is_err());
    }

    #[test]
    fn rejects_deployment_without_role() {
        assert!(
            InspectOptions::parse([OsString::from("deployment"), OsString::from("demo-local")])
                .is_err()
        );
    }

    #[test]
    fn rejects_broad_deployment_fanout() {
        assert!(
            InspectOptions::parse([
                OsString::from("deployment"),
                OsString::from("demo-local"),
                OsString::from("--all"),
            ])
            .is_err()
        );
    }

    #[test]
    fn rejects_endpoint_mode_flags_in_first_slice() {
        assert!(
            InspectOptions::parse([
                OsString::from("canister"),
                OsString::from("aaaaa-aa"),
                OsString::from("--health"),
            ])
            .is_err()
        );
        assert!(
            InspectOptions::parse([
                OsString::from("canister"),
                OsString::from("aaaaa-aa"),
                OsString::from("--readiness"),
            ])
            .is_err()
        );
    }

    #[test]
    fn rejects_ambiguous_runtime_status_alias() {
        assert!(
            InspectOptions::parse([OsString::from("runtime"), OsString::from("aaaaa-aa")]).is_err()
        );
    }

    #[test]
    fn usage_distinguishes_runtime_inspect_from_deploy_artifacts() {
        let text = usage();

        assert!(text.contains("guarded canic_runtime_status endpoint"));
        assert!(text.contains("canic deploy inspect"));
        assert!(text.contains("local deployment-truth artifacts"));
    }

    #[test]
    fn extracts_response_candid_fallback_from_icp_json() {
        let payload = runtime_response_payload(
            r#"{"response_candid":"(record { status = variant { Ok } })"}"#,
        )
        .expect("extract runtime status Candid");

        assert_eq!(payload.source, RUNTIME_OBSERVED_SOURCE);
        assert_eq!(payload.status, None);
        assert_eq!(payload.response_format, "candid");
        assert!(!payload.response_bytes_present);
        assert!(payload.response_candid_present);
    }

    #[test]
    fn decodes_runtime_status_from_response_bytes() {
        let status = sample_runtime_status(RuntimeStatus::Ok);
        let response = Ok::<_, canic_core::dto::error::Error>(status.clone());
        let output = format!(
            r#"{{"response_bytes":"{}"}}"#,
            hex_bytes(&Encode!(&response).expect("encode runtime status response"))
        );
        let payload = runtime_response_payload(&output).expect("decode runtime status");

        assert_eq!(payload.source, RUNTIME_OBSERVED_SOURCE);
        assert_eq!(payload.status, Some(status));
        assert_eq!(payload.response_format, "candid");
        assert!(payload.response_bytes_present);
        assert!(!payload.response_candid_present);
        assert_eq!(inspect_status_label(&payload), "ok");
    }

    #[test]
    fn invalid_response_bytes_hex_is_rejected() {
        let err = runtime_response_payload(r#"{"response_bytes":"not-hex"}"#)
            .expect_err("invalid hex rejected");

        assert!(matches!(err, InspectCommandError::InvalidResponse(_)));
    }

    #[test]
    fn text_report_labels_runtime_observed_payload() {
        let report = sample_inspect_report();

        let rendered = render_text_report(&report);

        assert!(rendered.contains("source: cli_arg"));
        assert!(rendered.contains("source: runtime_observed"));
        assert!(rendered.contains("endpoint: canic_runtime_status"));
        assert!(rendered.contains("response_format: candid"));
        assert!(rendered.contains("response_bytes_present: true"));
        assert!(rendered.contains("response_candid_present: true"));
        assert!(rendered.contains("status: ok"));
        assert!(rendered.contains("runtime_status: ok"));
        assert!(rendered.contains("schema_version: 1"));
        assert!(rendered.contains("role: root"));
        assert!(!rendered.contains("(record {})"));
        assert!(!rendered.contains("safe"));
    }

    #[test]
    fn json_report_labels_runtime_observed_payload() {
        let value = serde_json::to_value(sample_inspect_report()).expect("serialize report");

        assert_eq!(value["schema_version"], INSPECT_SCHEMA_VERSION);
        assert_eq!(value["command"], "canic inspect canister");
        assert_eq!(value["target_resolution"]["source"], CLI_ARG_SOURCE);
        assert_eq!(
            value["endpoint"],
            canic_core::protocol::CANIC_RUNTIME_STATUS
        );
        assert_eq!(value["status"], "ok");
        assert_eq!(value["runtime_status"]["source"], RUNTIME_OBSERVED_SOURCE);
        assert_eq!(value["runtime_status"]["status"]["status"], "ok");
        assert_eq!(value["runtime_status"]["response_format"], "candid");
        assert_eq!(value["runtime_status"]["response_bytes_present"], true);
        assert_eq!(value["runtime_status"]["response_candid_present"], true);
        assert!(value["runtime_status"].get("response_candid").is_none());
    }

    #[test]
    fn failing_runtime_status_maps_to_status_exit() {
        let mut report = sample_inspect_report();
        let payload = report.runtime_status.as_mut().expect("runtime status");
        payload.status = Some(sample_runtime_status(RuntimeStatus::Failing));
        report.status = inspect_status_label(payload).to_string();

        let err = command_exit_result(&report).expect_err("failing status exits nonzero");

        assert_eq!(report.status, "failing");
        assert_eq!(err.exit_code(), 1);
        assert!(err.suppress_stderr());
    }

    fn sample_inspect_report() -> InspectReport {
        InspectReport {
            schema_version: INSPECT_SCHEMA_VERSION,
            command: "canic inspect canister".to_string(),
            target_resolution: TargetResolution {
                deployment: None,
                role: None,
                canister_id: "aaaaa-aa".to_string(),
                network: "local".to_string(),
                source: CLI_ARG_SOURCE.to_string(),
            },
            endpoint: canic_core::protocol::CANIC_RUNTIME_STATUS.to_string(),
            status: "ok".to_string(),
            health_status: None,
            readiness_status: None,
            runtime_status: Some(RuntimeStatusPayload {
                source: RUNTIME_OBSERVED_SOURCE.to_string(),
                status: Some(sample_runtime_status(RuntimeStatus::Ok)),
                response_format: "candid".to_string(),
                response_bytes_present: true,
                response_candid_present: true,
            }),
            warnings: Vec::new(),
            next_actions: Vec::new(),
        }
    }

    fn sample_runtime_status(status: RuntimeStatus) -> CanicRuntimeStatus {
        use canic_core::dto::runtime::{CanicReadinessStatus, ReadinessStatus, RuntimeBuildInfo};

        CanicRuntimeStatus {
            schema_version: canic_core::dto::runtime::RUNTIME_INTROSPECTION_SCHEMA_VERSION,
            observed_at_ns: 42,
            canister_id: Principal::anonymous(),
            role: Some("root".to_string()),
            root: None,
            network: Some("local".to_string()),
            build: RuntimeBuildInfo {
                package_name: "root".to_string(),
                package_version: "0.81.0".to_string(),
                canic_version: "0.81.0".to_string(),
                canister_version: 7,
            },
            features: Vec::new(),
            topology: None,
            timers: Vec::new(),
            state: None,
            recent_failures: Vec::new(),
            visibility: Vec::new(),
            readiness: CanicReadinessStatus {
                schema_version: canic_core::dto::runtime::RUNTIME_INTROSPECTION_SCHEMA_VERSION,
                role: Some("root".to_string()),
                status: ReadinessStatus::Ready,
                observed_at_ns: 42,
                checks: Vec::new(),
                blockers: Vec::new(),
                warnings: Vec::new(),
            },
            status,
        }
    }

    fn hex_bytes(bytes: &[u8]) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut out = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            out.push(char::from(HEX[usize::from(byte >> 4)]));
            out.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
        out
    }
}
