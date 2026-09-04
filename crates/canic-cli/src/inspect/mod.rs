//! Module: canic_cli::inspect
//!
//! Responsibility: inspect one current Fleet canister's runtime-observed Canic status.
//! Does not own: ensure planning, runtime endpoint DTOs, or broad topology fanout.
//! Boundary: resolves one terminal ensure target, selects its role-owned Runtime status, and renders a report.

use crate::{
    cli::{
        clap::{flag_arg, parse_matches, render_usage, required_string, string_option_or_else},
        defaults::{default_icp, local_environment},
        globals::{internal_environment_arg, internal_icp_arg},
        help::print_help_or_version,
    },
    support::candid::registry_entry_candid_path,
    version_text,
};
use candid::{CandidType, Deserialize, Principal, types::principal::PrincipalError};
#[cfg(test)]
use canic_core::protocol::CANIC_ROOT_STATUS;
use canic_core::{
    dto::runtime::{
        CanicRuntimeStatus, RUNTIME_INTROSPECTION_SCHEMA_VERSION, RuntimeFeatureStatus,
        RuntimeStatus,
    },
    protocol::status_endpoint_for_role,
};
use canic_host::{
    fleet_ensure::{CurrentFleetInventoryError, resolve_current_fleet},
    icp::{IcpCli, IcpCommandError, IcpJsonResponseError, decode_json_result_response},
    icp_config::{IcpConfigError, resolve_current_canic_icp_root},
    protocol_binding::ResolvedProtocolBinding,
};
use clap::{Arg, Command as ClapCommand};
use serde::Serialize;
use std::{ffi::OsString, path::PathBuf};
use thiserror::Error as ThisError;

const INSPECT_SCHEMA_VERSION: u32 = 1;
const RUNTIME_OBSERVED_SOURCE: &str = "runtime_observed";
const CANDID_RESPONSE_FORMAT: &str = "candid";
const INSPECT_HELP_AFTER: &str = "\
Examples:
  canic inspect canister aaaaa-aa
  canic inspect fleet demo-local --role root

Inspect is read-only. It queries the guarded role-owned Runtime selector for
one explicit target and does not fan out across Fleet roles. Use
the Fleet form only after one current `fleet ensure` operation converges.";

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

    #[error(transparent)]
    CurrentFleet(#[from] CurrentFleetInventoryError),

    #[error("icp command failed: {0}")]
    Icp(#[from] IcpCommandError),

    #[error("invalid role-owned Runtime status response: {0}")]
    InvalidResponse(#[source] IcpJsonResponseError),

    #[error(
        "unsupported {subject} schema version {actual}; expected {RUNTIME_INTROSPECTION_SCHEMA_VERSION}"
    )]
    UnsupportedRuntimeSchema { subject: &'static str, actual: u32 },

    #[error("runtime status reported {0}")]
    ReportStatus(String),

    #[error("failed to resolve ICP project root: {0}")]
    IcpRoot(#[source] IcpConfigError),

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
            | Self::CurrentFleet(_)
            | Self::Icp(_)
            | Self::InvalidResponse(_)
            | Self::UnsupportedRuntimeSchema { .. }
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
        environment: String,
        icp: String,
        json: bool,
    },
    Fleet {
        fleet: String,
        role: String,
        environment: String,
        icp: String,
        json: bool,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ResolvedInspectTarget {
    command: InspectCommandKind,
    fleet: Option<String>,
    role: Option<String>,
    canister_id: String,
    environment: String,
    icp: String,
    source: InspectSource,
    protocol_binding: ResolvedProtocolBinding,
    icp_root: Option<PathBuf>,
    json: bool,
}

#[derive(CandidType, Deserialize)]
enum RoleStatusResponse {
    Runtime(CanicRuntimeStatus),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum InspectCommandKind {
    #[serde(rename = "canic inspect canister")]
    Canister,
    #[serde(rename = "canic inspect fleet")]
    Fleet,
}

impl InspectCommandKind {
    const fn label(self) -> &'static str {
        match self {
            Self::Canister => "canic inspect canister",
            Self::Fleet => "canic inspect fleet",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
enum InspectSource {
    #[serde(rename = "cli_arg")]
    CliArg,
    #[serde(rename = "current_ensure_inventory")]
    CurrentEnsureInventory,
}

impl InspectSource {
    const fn label(self) -> &'static str {
        match self {
            Self::CliArg => "cli_arg",
            Self::CurrentEnsureInventory => "current_ensure_inventory",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct InspectReport {
    schema_version: u32,
    command: InspectCommandKind,
    target_resolution: TargetResolution,
    endpoint: &'static str,
    status: RuntimeStatus,
    runtime_status: RuntimeStatusPayload,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct TargetResolution {
    fleet: Option<String>,
    role: Option<String>,
    canister_id: String,
    environment: String,
    source: InspectSource,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct RuntimeStatusPayload {
    source: &'static str,
    status: CanicRuntimeStatus,
    response_format: &'static str,
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
                    environment: string_option_or_else(matches, "environment", local_environment),
                    icp: string_option_or_else(matches, "icp", default_icp),
                    json: matches.get_flag("json"),
                })
            }
            Some(("fleet", matches)) => Ok(Self::Fleet {
                fleet: required_string(matches, "fleet"),
                role: required_string(matches, "role"),
                environment: string_option_or_else(matches, "environment", local_environment),
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
            environment,
            icp,
            json,
        } => {
            let command = InspectCommandKind::Canister;
            let source = InspectSource::CliArg;
            Err(InspectCommandError::Target(format!(
                "{} cannot select an exact protected protocol binding from source {}; use `canic inspect fleet <fleet> --role <role>` before querying Canister {canister} (environment {environment}, ICP executable {icp}, json={json})",
                command.label(),
                source.label(),
            )))
        }
        InspectOptions::Fleet {
            fleet,
            role,
            environment,
            icp,
            json,
        } => resolve_fleet_target(fleet, role, environment, icp, *json),
    }
}

fn resolve_fleet_target(
    fleet: &str,
    role: &str,
    environment: &str,
    icp: &str,
    json: bool,
) -> Result<ResolvedInspectTarget, InspectCommandError> {
    let root = resolve_current_canic_icp_root().map_err(InspectCommandError::IcpRoot)?;
    let current = resolve_current_fleet(&root, environment, fleet)?;
    let matches = current
        .registry
        .entries
        .iter()
        .filter(|entry| entry.role.as_deref() == Some(role))
        .collect::<Vec<_>>();

    let entry = match matches.as_slice() {
        [] => {
            return Err(InspectCommandError::Target(format!(
                "role {role} was not found in Fleet {fleet}"
            )));
        }
        [entry] => *entry,
        _ => {
            return Err(InspectCommandError::Target(format!(
                "role {role} resolves to multiple canisters in Fleet {fleet}; explicit disambiguation is not supported by canic inspect fleet"
            )));
        }
    };
    validate_principal(&entry.pid)?;

    let protocol_binding = registry_entry_candid_path(Some(root.as_path()), environment, entry)
        .map_err(|error| InspectCommandError::Target(error.to_string()))?;
    Ok(ResolvedInspectTarget {
        command: InspectCommandKind::Fleet,
        fleet: Some(fleet.to_string()),
        role: Some(role.to_string()),
        canister_id: entry.pid.clone(),
        environment: environment.to_string(),
        icp: icp.to_string(),
        source: InspectSource::CurrentEnsureInventory,
        protocol_binding,
        icp_root: Some(root),
        json,
    })
}

fn inspect_report(target: &ResolvedInspectTarget) -> Result<InspectReport, InspectCommandError> {
    let mut icp = IcpCli::new(&target.icp, Some(target.environment.clone()));
    if let Some(root) = &target.icp_root {
        icp = icp.with_cwd(root);
    }
    let endpoint = status_endpoint_for_role(&target.protocol_binding.binding().role);
    let output = icp.canister_query_arg_output_with_candid(
        &target.canister_id,
        endpoint,
        "(variant { Runtime })",
        Some("json"),
        Some(target.protocol_binding.candid_path()),
    )?;
    let runtime_status = runtime_response_payload(&output)?;
    let status = runtime_status.status.status;

    Ok(InspectReport {
        schema_version: INSPECT_SCHEMA_VERSION,
        command: target.command,
        target_resolution: TargetResolution {
            fleet: target.fleet.clone(),
            role: target.role.clone(),
            canister_id: target.canister_id.clone(),
            environment: target.environment.clone(),
            source: target.source,
        },
        endpoint,
        status,
        runtime_status,
    })
}

fn runtime_response_payload(output: &str) -> Result<RuntimeStatusPayload, InspectCommandError> {
    let response = decode_json_result_response::<RoleStatusResponse>(output)
        .map_err(InspectCommandError::InvalidResponse)?;
    let RoleStatusResponse::Runtime(status) = response;
    require_current_runtime_schema(&status)?;

    Ok(RuntimeStatusPayload {
        source: RUNTIME_OBSERVED_SOURCE,
        status,
        response_format: CANDID_RESPONSE_FORMAT,
    })
}

fn require_current_runtime_schema(status: &CanicRuntimeStatus) -> Result<(), InspectCommandError> {
    for (subject, actual) in [
        ("runtime introspection", status.schema_version),
        ("runtime readiness", status.readiness.schema_version),
    ] {
        if actual != RUNTIME_INTROSPECTION_SCHEMA_VERSION {
            return Err(InspectCommandError::UnsupportedRuntimeSchema { subject, actual });
        }
    }
    Ok(())
}

fn command_exit_result(report: &InspectReport) -> Result<(), InspectCommandError> {
    match report.status {
        RuntimeStatus::Failing => Err(InspectCommandError::ReportStatus(
            RuntimeStatus::Failing.label().to_string(),
        )),
        RuntimeStatus::Ok | RuntimeStatus::Degraded | RuntimeStatus::Unknown => Ok(()),
    }
}

fn render_text_report(report: &InspectReport) -> String {
    let mut lines = vec![
        report.command.label().to_string(),
        format!("status: {}", report.status.label()),
        format!("endpoint: {}", report.endpoint),
        format!("canister: {}", report.target_resolution.canister_id),
        format!("environment: {}", report.target_resolution.environment),
        format!("source: {}", report.target_resolution.source.label()),
    ];
    if let Some(fleet) = &report.target_resolution.fleet {
        lines.push(format!("fleet: {fleet}"));
    }
    if let Some(role) = &report.target_resolution.role {
        lines.push(format!("role: {role}"));
    }
    let runtime_status = &report.runtime_status;
    lines.extend([
        String::new(),
        "runtime_status".to_string(),
        format!("source: {}", runtime_status.source),
        format!("response_format: {}", runtime_status.response_format),
    ]);
    let status = &runtime_status.status;
    lines.extend([
        format!("runtime_status: {}", status.status.label()),
        format!("schema_version: {}", status.schema_version),
        format!("observed_at_ns: {}", status.observed_at_ns),
        format!("role: {}", status.role.as_deref().unwrap_or("unknown")),
        format!("features: {}", status.features.len()),
        format!("timers: {}", status.timers.len()),
        format!("recent_failures: {}", status.recent_failures.len()),
    ]);
    if let Some(state) = &status.state {
        lines.push(format!("state_domains: {}", state.domains.len()));
    }
    append_runtime_metadata_lines(&mut lines, status);
    lines.join("\n")
}

fn append_runtime_metadata_lines(lines: &mut Vec<String>, status: &CanicRuntimeStatus) {
    lines.push(format!(
        "enabled_features: {}",
        enabled_runtime_features(status)
    ));
    if let Some(auth) = &status.auth {
        lines.push(format!(
            "auth: enabled_features={}",
            enabled_runtime_feature_rows(&auth.auth_features)
        ));
    }
    if let Some(blob_storage) = &status.blob_storage {
        lines.push(format!(
            "blob_storage: enabled_features={}",
            enabled_runtime_feature_rows(&blob_storage.blob_storage_features)
        ));
    }
    if let Some(capacity) = &status.receipt_capacity {
        lines.push(format!(
            "receipt_capacity: status={} receipts={}/{} receipt_headroom={} resource_totals={}/{} resource_headroom={} warning_headroom_threshold={}",
            capacity.status.label(),
            capacity.receipt_records,
            capacity.receipt_record_limit,
            capacity.remaining_receipt_record_headroom,
            capacity.resource_total_records,
            capacity.resource_total_record_limit,
            capacity.remaining_resource_total_headroom,
            capacity.warning_headroom_threshold,
        ));
    }

    for timer in &status.timers {
        lines.push(format!(
            "timer: {}/{}/{} registration={} condition={} mode={} enabled={}",
            timer.owner,
            timer.subsystem,
            timer.name,
            timer.registration.label(),
            timer.condition.label(),
            timer.scheduling_mode.label(),
            timer.enabled
        ));
    }

    if let Some(state) = &status.state {
        for domain in &state.domains {
            let memory_id = domain
                .memory_id
                .map_or_else(|| "none".to_string(), |memory_id| memory_id.to_string());
            lines.push(format!(
                "state_domain: {} version={} storage={} memory_id={} status={}",
                domain.domain,
                domain.version,
                domain.storage,
                memory_id,
                domain.status.label()
            ));
        }
    }

    for failure in &status.recent_failures {
        lines.push(format!(
            "recent_failure: {}/{} severity={} redacted={}",
            failure.subsystem,
            failure.code,
            failure.severity.label(),
            failure.redacted
        ));
    }
}

fn enabled_runtime_features(status: &CanicRuntimeStatus) -> String {
    enabled_runtime_feature_rows(&status.features)
}

fn enabled_runtime_feature_rows(features: &[RuntimeFeatureStatus]) -> String {
    let names = features
        .iter()
        .filter(|feature| feature.enabled)
        .map(|feature| feature.name.as_str())
        .collect::<Vec<_>>();
    if names.is_empty() {
        "none".to_string()
    } else {
        names.join(", ")
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

fn command() -> ClapCommand {
    ClapCommand::new("inspect")
        .bin_name("canic inspect")
        .about("Inspect runtime-observed status for one current Canic canister")
        .disable_help_flag(true)
        .subcommand_required(true)
        .subcommand(canister_command())
        .subcommand(fleet_command())
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
        .arg(internal_environment_arg())
        .arg(internal_icp_arg())
        .arg(flag_arg("json").long("json").help("Print JSON output"))
}

fn fleet_command() -> ClapCommand {
    ClapCommand::new("fleet")
        .about("Inspect one role in a terminal current Fleet")
        .disable_help_flag(true)
        .arg(
            Arg::new("fleet")
                .value_name("fleet")
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
        .arg(internal_environment_arg())
        .arg(internal_icp_arg())
        .arg(flag_arg("json").long("json").help("Print JSON output"))
}

fn usage() -> String {
    render_usage(command)
}

fn canister_usage() -> String {
    render_usage(canister_command)
}

fn fleet_usage() -> String {
    render_usage(fleet_command)
}

fn print_leaf_help_or_version(args: &[OsString]) -> bool {
    let Some(usage) = args
        .first()
        .and_then(|arg| arg.to_str())
        .and_then(|leaf| match leaf {
            "canister" => Some(canister_usage as fn() -> String),
            "fleet" => Some(fleet_usage as fn() -> String),
            _ => None,
        })
    else {
        return false;
    };
    let Some(arg) = args.get(1).and_then(|arg| arg.to_str()) else {
        return false;
    };
    match arg {
        "--help" | "-h" => {
            println!("{}", usage());
            true
        }
        "--version" | "-V" => {
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
    use canic_core::cdk::utils::hash::hex_bytes;

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
                environment: local_environment(),
                icp: default_icp(),
                json: true,
            }
        );
    }

    #[test]
    fn parses_fleet_role_target() {
        let options = InspectOptions::parse([
            OsString::from("fleet"),
            OsString::from("demo-local"),
            OsString::from("--role"),
            OsString::from("root"),
        ])
        .expect("parse Fleet inspect");

        assert_eq!(
            options,
            InspectOptions::Fleet {
                fleet: "demo-local".to_string(),
                role: "root".to_string(),
                environment: local_environment(),
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
    fn rejects_fleet_without_role() {
        assert!(
            InspectOptions::parse([OsString::from("fleet"), OsString::from("demo-local")]).is_err()
        );
    }

    #[test]
    fn rejects_broad_fleet_fanout() {
        assert!(
            InspectOptions::parse([
                OsString::from("fleet"),
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
    fn usage_binds_fleet_inspection_to_terminal_current_ensure() {
        let text = usage();

        assert!(text.contains("guarded role-owned Runtime selector"));
        assert!(text.contains("current `fleet ensure` operation converges"));
        assert!(!text.contains("canic deploy inspect"));
    }

    #[test]
    fn response_without_response_bytes_is_rejected() {
        let err = runtime_response_payload("{}").expect_err("typed response bytes are required");

        assert!(matches!(
            err,
            InspectCommandError::InvalidResponse(IcpJsonResponseError::MissingResponseBytes)
        ));
    }

    #[test]
    fn decodes_runtime_status_from_response_bytes() {
        let status = sample_runtime_status(RuntimeStatus::Ok);
        let response =
            Ok::<_, canic_core::dto::error::Error>(RoleStatusResponse::Runtime(status.clone()));
        let output = format!(
            r#"{{"response_bytes":"{}"}}"#,
            hex_bytes(Encode!(&response).expect("encode runtime status response"))
        );
        let payload = runtime_response_payload(&output).expect("decode runtime status");

        assert_eq!(payload.source, RUNTIME_OBSERVED_SOURCE);
        assert_eq!(payload.status, status);
        assert_eq!(payload.response_format, CANDID_RESPONSE_FORMAT);
        assert_eq!(payload.status.status, RuntimeStatus::Ok);
    }

    #[test]
    fn rejects_unsupported_runtime_schema() {
        let mut status = sample_runtime_status(RuntimeStatus::Ok);
        status.schema_version = RUNTIME_INTROSPECTION_SCHEMA_VERSION + 1;
        let response = Ok::<_, canic_core::dto::error::Error>(RoleStatusResponse::Runtime(status));
        let output = format!(
            r#"{{"response_bytes":"{}"}}"#,
            hex_bytes(Encode!(&response).expect("encode unsupported runtime status response"))
        );

        assert!(matches!(
            runtime_response_payload(&output),
            Err(InspectCommandError::UnsupportedRuntimeSchema {
                subject: "runtime introspection",
                actual,
            }) if actual == RUNTIME_INTROSPECTION_SCHEMA_VERSION + 1
        ));
    }

    #[test]
    fn rejects_unsupported_readiness_schema() {
        let mut status = sample_runtime_status(RuntimeStatus::Ok);
        status.readiness.schema_version = RUNTIME_INTROSPECTION_SCHEMA_VERSION + 1;
        let response = Ok::<_, canic_core::dto::error::Error>(RoleStatusResponse::Runtime(status));
        let output = format!(
            r#"{{"response_bytes":"{}"}}"#,
            hex_bytes(Encode!(&response).expect("encode unsupported readiness response"))
        );

        assert!(matches!(
            runtime_response_payload(&output),
            Err(InspectCommandError::UnsupportedRuntimeSchema {
                subject: "runtime readiness",
                actual,
            }) if actual == RUNTIME_INTROSPECTION_SCHEMA_VERSION + 1
        ));
    }

    #[test]
    fn invalid_response_bytes_hex_is_rejected() {
        let err = runtime_response_payload(r#"{"response_bytes":"not-hex"}"#)
            .expect_err("invalid hex rejected");

        assert!(matches!(
            err,
            InspectCommandError::InvalidResponse(IcpJsonResponseError::Hex(_))
        ));
    }

    #[test]
    fn text_report_labels_runtime_observed_payload() {
        let report = sample_inspect_report();

        let rendered = render_text_report(&report);

        assert!(rendered.contains("source: cli_arg"));
        assert!(rendered.contains("source: runtime_observed"));
        assert!(rendered.contains("endpoint: canic_root_status"));
        assert!(rendered.contains("response_format: candid"));
        assert!(rendered.contains("status: ok"));
        assert!(rendered.contains("runtime_status: ok"));
        assert!(rendered.contains("schema_version: 1"));
        assert!(rendered.contains("role: root"));
        assert!(rendered.contains("features: 2"));
        assert!(rendered.contains("timers: 1"));
        assert!(rendered.contains("recent_failures: 1"));
        assert!(rendered.contains("state_domains: 1"));
        assert!(rendered.contains("enabled_features: sharding"));
        assert!(rendered.contains(
            "receipt_capacity: status=pass receipts=12/1000 receipt_headroom=988 resource_totals=7/1000 resource_headroom=993 warning_headroom_threshold=100"
        ));
        assert!(
            rendered
                .contains("timer: canic/runtime/heartbeat registration=scheduled condition=active mode=after_completion enabled=true")
        );
        assert!(rendered.contains(
            "state_domain: runtime_bindings version=1 storage=stable_memory memory_id=1 status=ok"
        ));
        assert!(rendered.contains(
            "recent_failure: runtime/runtime_status_sample severity=warning redacted=true"
        ));
        assert!(!rendered.contains("(record {})"));
        assert!(!rendered.contains("safe"));
    }

    #[test]
    fn json_report_labels_runtime_observed_payload() {
        let value = serde_json::to_value(sample_inspect_report()).expect("serialize report");

        assert_eq!(value["schema_version"], INSPECT_SCHEMA_VERSION);
        assert_eq!(value["command"], "canic inspect canister");
        assert_eq!(value["target_resolution"]["source"], "cli_arg");
        assert_eq!(value["endpoint"], CANIC_ROOT_STATUS);
        assert_eq!(value["status"], "ok");
        assert_eq!(value["runtime_status"]["source"], "runtime_observed");
        assert_eq!(value["runtime_status"]["status"]["status"], "ok");
        assert_eq!(value["runtime_status"]["status"]["build_network"], "local");
        assert_eq!(
            value["runtime_status"]["status"]["features"][0]["name"],
            "sharding"
        );
        assert_eq!(
            value["runtime_status"]["status"]["features"][0]["source"],
            "compile_feature"
        );
        assert_eq!(
            value["runtime_status"]["status"]["timers"][0]["subsystem"],
            "runtime"
        );
        assert_eq!(
            value["runtime_status"]["status"]["state"]["domains"][0]["domain"],
            "runtime_bindings"
        );
        assert_eq!(
            value["runtime_status"]["status"]["receipt_capacity"]["receipt_record_limit"],
            1_000
        );
        assert_eq!(
            value["runtime_status"]["status"]["receipt_capacity"]["remaining_resource_total_headroom"],
            993
        );
        assert_eq!(
            value["runtime_status"]["status"]["recent_failures"][0]["redacted"],
            true
        );
        assert_eq!(value["runtime_status"]["response_format"], "candid");
        assert!(value.get("health_status").is_none());
        assert!(value.get("readiness_status").is_none());
        assert!(value.get("warnings").is_none());
        assert!(value.get("next_actions").is_none());
    }

    #[test]
    fn fleet_json_report_uses_fleet_identity() {
        let mut report = sample_inspect_report();
        report.command = InspectCommandKind::Fleet;
        report.target_resolution.fleet = Some("demo".to_string());
        report.target_resolution.role = Some("root".to_string());
        report.target_resolution.source = InspectSource::CurrentEnsureInventory;

        let value = serde_json::to_value(report).expect("serialize Fleet report");

        assert_eq!(value["command"], "canic inspect fleet");
        assert_eq!(value["target_resolution"]["fleet"], "demo");
        assert_eq!(value["target_resolution"]["role"], "root");
        assert_eq!(
            value["target_resolution"]["source"],
            "current_ensure_inventory"
        );
    }

    #[test]
    fn failing_runtime_status_maps_to_status_exit() {
        let mut report = sample_inspect_report();
        report.runtime_status.status = sample_runtime_status(RuntimeStatus::Failing);
        report.status = report.runtime_status.status.status;

        let err = command_exit_result(&report).expect_err("failing status exits nonzero");

        assert_eq!(report.status, RuntimeStatus::Failing);
        assert_eq!(err.exit_code(), 1);
        assert!(err.suppress_stderr());
    }

    fn sample_inspect_report() -> InspectReport {
        InspectReport {
            schema_version: INSPECT_SCHEMA_VERSION,
            command: InspectCommandKind::Canister,
            target_resolution: TargetResolution {
                fleet: None,
                role: None,
                canister_id: "aaaaa-aa".to_string(),
                environment: "local".to_string(),
                source: InspectSource::CliArg,
            },
            endpoint: CANIC_ROOT_STATUS,
            status: RuntimeStatus::Ok,
            runtime_status: RuntimeStatusPayload {
                source: RUNTIME_OBSERVED_SOURCE,
                status: sample_runtime_status(RuntimeStatus::Ok),
                response_format: CANDID_RESPONSE_FORMAT,
            },
        }
    }

    fn sample_runtime_status(status: RuntimeStatus) -> CanicRuntimeStatus {
        use canic_core::dto::runtime::{
            CanicReadinessStatus, FailureSeverity, ReadinessStatus, RecentFailure,
            RuntimeAuthStatusSummary, RuntimeBlobStorageStatusSummary, RuntimeBuildInfo,
            RuntimeCheck, RuntimeCheckStatus, RuntimeFeatureStatus, RuntimeFieldVisibility,
            RuntimeStateDomainStatus, RuntimeStateDomainSummary, RuntimeStateSummary,
        };

        CanicRuntimeStatus {
            schema_version: canic_core::dto::runtime::RUNTIME_INTROSPECTION_SCHEMA_VERSION,
            observed_at_ns: 42,
            canister_id: Principal::anonymous(),
            role: Some("root".to_string()),
            root: None,
            build_network: Some(canic_core::ids::BuildNetwork::Local),
            build: RuntimeBuildInfo {
                package_name: "root".to_string(),
                package_version: "0.81.0".to_string(),
                canic_version: "0.81.0".to_string(),
                canister_version: 7,
            },
            features: vec![
                RuntimeFeatureStatus {
                    name: "sharding".to_string(),
                    enabled: true,
                    visibility: RuntimeFieldVisibility::OperatorOnly,
                    source: "compile_feature".to_string(),
                },
                RuntimeFeatureStatus {
                    name: "blob-storage".to_string(),
                    enabled: false,
                    visibility: RuntimeFieldVisibility::OperatorOnly,
                    source: "compile_feature".to_string(),
                },
            ],
            topology: None,
            timers: vec![sample_timer_status()],
            timer_inventory: RuntimeCheck {
                category: "runtime".to_string(),
                code: "timer_inventory_available".to_string(),
                status: RuntimeCheckStatus::Pass,
                subject: "shared_timer_registry".to_string(),
                detail: "available".to_string(),
                next: None,
                source: "ic_timers".to_string(),
            },
            state: Some(RuntimeStateSummary {
                manifest_schema_version: 1,
                domains: vec![RuntimeStateDomainSummary {
                    domain: "runtime_bindings".to_string(),
                    version: 1,
                    storage: "stable_memory".to_string(),
                    memory_id: Some(1),
                    status: RuntimeStateDomainStatus::Ok,
                }],
                total_stable_memory_pages: None,
            }),
            auth: Some(RuntimeAuthStatusSummary {
                auth_features: vec![RuntimeFeatureStatus {
                    name: "auth-delegated-token-verify".to_string(),
                    enabled: true,
                    visibility: RuntimeFieldVisibility::OperatorOnly,
                    source: "compile_feature".to_string(),
                }],
            }),
            blob_storage: Some(RuntimeBlobStorageStatusSummary {
                blob_storage_features: vec![RuntimeFeatureStatus {
                    name: "blob-storage".to_string(),
                    enabled: false,
                    visibility: RuntimeFieldVisibility::OperatorOnly,
                    source: "compile_feature".to_string(),
                }],
            }),
            receipt_capacity: Some(sample_receipt_capacity()),
            recent_failures: vec![RecentFailure {
                occurred_at_ns: 41,
                subsystem: "runtime".to_string(),
                code: "runtime_status_sample".to_string(),
                severity: FailureSeverity::Warning,
                summary: "redacted sample failure".to_string(),
                correlation_id: None,
                redacted: true,
            }],
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

    fn sample_receipt_capacity() -> canic_core::dto::runtime::RuntimeReceiptCapacityStatus {
        canic_core::dto::runtime::RuntimeReceiptCapacityStatus {
            status: canic_core::dto::runtime::RuntimeCheckStatus::Pass,
            receipt_records: 12,
            application_receipt_records: 10,
            canic_owned_receipt_records: 2,
            pending_application_receipt_records: 3,
            terminal_application_receipt_records: 7,
            receipt_record_limit: 1_000,
            remaining_receipt_record_headroom: 988,
            resource_total_records: 7,
            resource_total_record_limit: 1_000,
            remaining_resource_total_headroom: 993,
            warning_headroom_threshold: 100,
            reserved_terminal_slots: 10,
            reserved_terminal_pages: 8,
            next_terminal_eligibility_at_ns: Some(123),
            source: "intent_storage".to_string(),
        }
    }

    fn sample_timer_status() -> canic_core::dto::runtime::CanisterTimerStatus {
        use canic_core::dto::runtime::{
            CanisterTimerStatus, TimerCallbackPerformanceStatus, TimerExecutionOutcome,
            TimerMemoryPageExtentStatus, TimerMemoryPageSampleStatus, TimerProcessCondition,
            TimerRegistrationStatus, TimerSchedulingMode,
        };

        CanisterTimerStatus {
            name: "heartbeat".to_string(),
            owner: "canic".to_string(),
            subsystem: "runtime".to_string(),
            scheduling_mode: TimerSchedulingMode::AfterCompletion,
            registration: TimerRegistrationStatus::Scheduled,
            condition: TimerProcessCondition::Active,
            enabled: true,
            generation: Some(2),
            next_due_at_ns: Some(100),
            last_outcome: Some(TimerExecutionOutcome::Success),
            last_work_count: 1,
            last_success_at_ns: None,
            last_failure_at_ns: None,
            consecutive_expected_failures: 0,
            schedules_since_runtime_start: 2,
            executions_since_runtime_start: 1,
            successes_since_runtime_start: 1,
            expected_failures_since_runtime_start: 0,
            invariant_failures_since_runtime_start: 0,
            stale_callbacks_since_runtime_start: 0,
            scheduler_performance: TimerCallbackPerformanceStatus {
                instruction_samples_since_runtime_start: 0,
                instructions_latest: None,
                instructions_maximum: None,
                instructions_total_since_runtime_start: 0,
                memory_page_samples_since_runtime_start: 0,
                memory_pages_latest: None,
                maximum_wasm_memory_growth_pages: None,
                maximum_stable_memory_growth_pages: None,
            },
            work_performance: TimerCallbackPerformanceStatus {
                instruction_samples_since_runtime_start: 1,
                instructions_latest: Some(10),
                instructions_maximum: Some(10),
                instructions_total_since_runtime_start: 10,
                memory_page_samples_since_runtime_start: 1,
                memory_pages_latest: Some(TimerMemoryPageSampleStatus {
                    start: TimerMemoryPageExtentStatus {
                        wasm_pages: 20,
                        stable_pages: 2,
                    },
                    end: TimerMemoryPageExtentStatus {
                        wasm_pages: 21,
                        stable_pages: 2,
                    },
                }),
                maximum_wasm_memory_growth_pages: Some(1),
                maximum_stable_memory_growth_pages: Some(0),
            },
        }
    }
}
