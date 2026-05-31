use crate::{
    cli::clap::{parse_matches, parse_subcommand, passthrough_subcommand, path_option, value_arg},
    cli::help::print_help_or_version,
    output, version_text,
};
use canic_host::{
    evidence_envelope::{
        CommandProvenanceV1, EvidenceEnvelopeV1, EvidenceMessageSeverityV1, EvidenceMessageV1,
        EvidenceSummaryV1, EvidenceTargetKindV1, EvidenceTargetV1, ExitClassV1,
        evidence_envelope_schema, json_payload_sha256, policy_gate_report_schema,
        project_evidence_gate_report_schema,
    },
    policy_gate::{
        PolicyEvaluationStatusV1, PolicyFindingSeverityV1, PolicyGateError, PolicyGateReportV1,
        PolicyGateRequest, ProjectEvidenceGateReportV1, ProjectEvidenceManifestGateRequest,
        evaluate_policy_gate, evaluate_project_evidence_manifest_gate,
    },
};
use clap::{ArgGroup, Command as ClapCommand};
use serde::Serialize;
use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error as ThisError;

const COMPARED_FIELDS: &[&str] = &[
    "envelope_schema",
    "command",
    "target",
    "source_config",
    "inputs",
    "payload_schema",
    "payload_sha256",
    "summary",
    "exit_class",
];
const IGNORED_FIELDS: &[&str] = &["canic_version", "generated_at", "payload"];

///
/// EvidenceCommandError
///
#[derive(Debug, ThisError)]
pub enum EvidenceCommandError {
    #[error("{0}")]
    Usage(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error("envelopes differ:\n{0}")]
    EnvelopesDiffer(String),

    #[error(transparent)]
    PolicyGate(#[from] PolicyGateError),

    #[error("policy gate failed ({exit_class:?})\n{findings}")]
    PolicyGateFailed {
        exit_class: ExitClassV1,
        findings: String,
    },
}

///
/// EvidenceCompareOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
struct EvidenceCompareOptions {
    left: PathBuf,
    right: PathBuf,
    format: EvidenceCompareFormat,
}

impl EvidenceCompareOptions {
    fn parse<I>(args: I) -> Result<Self, EvidenceCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(evidence_compare_command(), args)
            .map_err(|_| EvidenceCommandError::Usage(compare_usage()))?;
        let format = match matches
            .get_one::<String>("format")
            .map_or("text", String::as_str)
        {
            "text" => EvidenceCompareFormat::Text,
            "json" => EvidenceCompareFormat::Json,
            _ => return Err(EvidenceCommandError::Usage(compare_usage())),
        };

        Ok(Self {
            left: path_option(&matches, "left").expect("clap requires left"),
            right: path_option(&matches, "right").expect("clap requires right"),
            format,
        })
    }
}

///
/// EvidenceCompareFormat
///
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EvidenceCompareFormat {
    Text,
    Json,
}

///
/// EvidenceCompareStatus
///
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum EvidenceCompareStatus {
    Matched,
    Different,
}

///
/// EvidenceCompareDifference
///
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct EvidenceCompareDifference {
    field: String,
    left: serde_json::Value,
    right: serde_json::Value,
}

///
/// EvidenceCompareReport
///
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct EvidenceCompareReport {
    schema_version: u32,
    status: EvidenceCompareStatus,
    left: String,
    right: String,
    compared_fields: Vec<String>,
    ignored_fields: Vec<String>,
    differences: Vec<EvidenceCompareDifference>,
}

///
/// EvidenceGateOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
struct EvidenceGateOptions {
    policy: PathBuf,
    input: EvidenceGateInput,
    format: EvidenceGateFormat,
    output: Option<PathBuf>,
}

///
/// EvidenceGateInput
///
#[derive(Clone, Debug, Eq, PartialEq)]
enum EvidenceGateInput {
    Envelope(PathBuf),
    Manifest(PathBuf),
}

///
/// EvidenceGateReport
///
#[derive(Clone, Debug, Eq, PartialEq)]
enum EvidenceGateReport {
    Envelope(PolicyGateReportV1),
    Manifest(ProjectEvidenceGateReportV1),
}

impl EvidenceGateReport {
    const fn gate_exit_class(&self) -> ExitClassV1 {
        match self {
            Self::Envelope(report) => report.gate_exit_class,
            Self::Manifest(report) => report.gate_exit_class,
        }
    }
}

impl EvidenceGateOptions {
    fn parse<I>(args: I) -> Result<Self, EvidenceCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(evidence_gate_command(), args)
            .map_err(|_| EvidenceCommandError::Usage(gate_usage()))?;
        let format = match matches
            .get_one::<String>("format")
            .map_or("text", String::as_str)
        {
            "text" => EvidenceGateFormat::Text,
            "json" => EvidenceGateFormat::Json,
            "envelope-json" => EvidenceGateFormat::EnvelopeJson,
            _ => return Err(EvidenceCommandError::Usage(gate_usage())),
        };

        Ok(Self {
            policy: path_option(&matches, "policy").expect("clap requires policy"),
            input: if let Some(envelope) = path_option(&matches, "envelope") {
                EvidenceGateInput::Envelope(envelope)
            } else {
                EvidenceGateInput::Manifest(
                    path_option(&matches, "manifest").expect(
                        "clap requires one of envelope or manifest through gate-input group",
                    ),
                )
            },
            format,
            output: path_option(&matches, "output"),
        })
    }
}

///
/// EvidenceGateFormat
///
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EvidenceGateFormat {
    Text,
    Json,
    EnvelopeJson,
}

/// Run an evidence subcommand.
pub fn run<I>(args: I) -> Result<(), EvidenceCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    let Some((command, args)) = parse_subcommand(evidence_command(), args)
        .map_err(|_| EvidenceCommandError::Usage(usage()))?
    else {
        return Err(EvidenceCommandError::Usage(usage()));
    };

    match command.as_str() {
        "compare" => {
            if print_help_or_version(&args, compare_usage, version_text()) {
                return Ok(());
            }
            let options = EvidenceCompareOptions::parse(args)?;
            let report = compare_envelope_files(&options)?;
            write_compare_report(&options, &report)?;
            if report.status == EvidenceCompareStatus::Different {
                return Err(EvidenceCommandError::EnvelopesDiffer(
                    render_compare_differences(&report),
                ));
            }
            Ok(())
        }
        "gate" => {
            if print_help_or_version(&args, gate_usage, version_text()) {
                return Ok(());
            }
            let options = EvidenceGateOptions::parse(args)?;
            let report = evaluate_gate_files(&options)?;
            write_gate_report(&options, &report)?;
            if !is_success_exit_class(report.gate_exit_class()) {
                return Err(EvidenceCommandError::PolicyGateFailed {
                    exit_class: report.gate_exit_class(),
                    findings: render_gate_findings(&report),
                });
            }
            Ok(())
        }
        _ => unreachable!("evidence dispatch command only defines known commands"),
    }
}

fn evaluate_gate_files(
    options: &EvidenceGateOptions,
) -> Result<EvidenceGateReport, EvidenceCommandError> {
    let policy_source = fs::read_to_string(&options.policy)?;
    let root = std::env::current_dir()?;
    match &options.input {
        EvidenceGateInput::Envelope(envelope_path) => {
            let envelope =
                output::read_json_file::<EvidenceEnvelopeV1, EvidenceCommandError>(envelope_path)?;
            evaluate_policy_gate(PolicyGateRequest {
                policy_source: &policy_source,
                policy_path: &options.policy,
                envelope_path,
                fingerprint_root: &root,
                envelope,
            })
            .map(EvidenceGateReport::Envelope)
            .map_err(EvidenceCommandError::from)
        }
        EvidenceGateInput::Manifest(manifest_path) => {
            let manifest_source = fs::read_to_string(manifest_path)?;
            evaluate_project_evidence_manifest_gate(ProjectEvidenceManifestGateRequest {
                policy_source: &policy_source,
                policy_path: &options.policy,
                manifest_source: &manifest_source,
                manifest_path,
                fingerprint_root: &root,
            })
            .map(EvidenceGateReport::Manifest)
            .map_err(EvidenceCommandError::from)
        }
    }
}

fn compare_envelope_files(
    options: &EvidenceCompareOptions,
) -> Result<EvidenceCompareReport, EvidenceCommandError> {
    let left = output::read_json_file::<EvidenceEnvelopeV1, EvidenceCommandError>(&options.left)?;
    let right = output::read_json_file::<EvidenceEnvelopeV1, EvidenceCommandError>(&options.right)?;
    Ok(compare_envelopes(
        &left,
        &right,
        &options.left,
        &options.right,
    ))
}

fn compare_envelopes(
    left: &EvidenceEnvelopeV1,
    right: &EvidenceEnvelopeV1,
    left_path: &Path,
    right_path: &Path,
) -> EvidenceCompareReport {
    let left_value = serde_json::to_value(left).expect("envelope should serialize");
    let right_value = serde_json::to_value(right).expect("envelope should serialize");
    let mut differences = Vec::new();
    for field in COMPARED_FIELDS {
        let left_field = left_value.get(*field).cloned().unwrap_or_default();
        let right_field = right_value.get(*field).cloned().unwrap_or_default();
        if left_field != right_field {
            differences.push(EvidenceCompareDifference {
                field: (*field).to_string(),
                left: left_field,
                right: right_field,
            });
        }
    }

    EvidenceCompareReport {
        schema_version: 1,
        status: if differences.is_empty() {
            EvidenceCompareStatus::Matched
        } else {
            EvidenceCompareStatus::Different
        },
        left: left_path.display().to_string(),
        right: right_path.display().to_string(),
        compared_fields: COMPARED_FIELDS
            .iter()
            .map(|field| (*field).to_string())
            .collect(),
        ignored_fields: IGNORED_FIELDS
            .iter()
            .map(|field| (*field).to_string())
            .collect(),
        differences,
    }
}

fn write_compare_report(
    options: &EvidenceCompareOptions,
    report: &EvidenceCompareReport,
) -> Result<(), EvidenceCommandError> {
    match options.format {
        EvidenceCompareFormat::Text => {
            output::write_text::<EvidenceCommandError>(None, &render_compare_report(report))
        }
        EvidenceCompareFormat::Json => output::write_pretty_json(None, report),
    }
}

fn write_gate_report(
    options: &EvidenceGateOptions,
    report: &EvidenceGateReport,
) -> Result<(), EvidenceCommandError> {
    match options.format {
        EvidenceGateFormat::Text => output::write_text::<EvidenceCommandError>(
            options.output.as_ref(),
            &render_gate_report(report),
        ),
        EvidenceGateFormat::Json => match report {
            EvidenceGateReport::Envelope(report) => {
                output::write_pretty_json(options.output.as_ref(), report)
            }
            EvidenceGateReport::Manifest(report) => {
                output::write_pretty_json(options.output.as_ref(), report)
            }
        },
        EvidenceGateFormat::EnvelopeJson => {
            let envelope = policy_gate_envelope(options, report)?;
            output::write_pretty_json(options.output.as_ref(), &envelope)
        }
    }
}

fn policy_gate_envelope(
    options: &EvidenceGateOptions,
    report: &EvidenceGateReport,
) -> Result<EvidenceEnvelopeV1, EvidenceCommandError> {
    let (payload_sha256, payload) = match report {
        EvidenceGateReport::Envelope(report) => {
            (json_payload_sha256(report)?, serde_json::to_value(report)?)
        }
        EvidenceGateReport::Manifest(report) => {
            (json_payload_sha256(report)?, serde_json::to_value(report)?)
        }
    };
    Ok(EvidenceEnvelopeV1 {
        envelope_schema: evidence_envelope_schema(),
        canic_version: env!("CARGO_PKG_VERSION").to_string(),
        command: CommandProvenanceV1 {
            name: "canic evidence gate".to_string(),
            argv_normalized: normalized_gate_args(options),
            argv_redactions: Vec::new(),
            format: "envelope-json".to_string(),
        },
        target: policy_gate_target(report),
        generated_at: current_evidence_generated_at(),
        source_config: None,
        inputs: policy_gate_inputs(report),
        payload_schema: policy_gate_payload_schema(report),
        payload_sha256: Some(payload_sha256),
        payload,
        summary: policy_gate_summary(report),
        exit_class: report.gate_exit_class(),
    })
}

fn render_compare_report(report: &EvidenceCompareReport) -> String {
    let mut lines = vec![
        "Evidence envelope compare:".to_string(),
        format!("  left: {}", report.left),
        format!("  right: {}", report.right),
        format!(
            "  status: {}",
            match report.status {
                EvidenceCompareStatus::Matched => "matched",
                EvidenceCompareStatus::Different => "different",
            }
        ),
        format!("  compared_fields: {}", report.compared_fields.join(", ")),
        format!("  ignored_fields: {}", report.ignored_fields.join(", ")),
    ];

    if report.differences.is_empty() {
        lines.push("Differences: none".to_string());
    } else {
        lines.push("Differences:".to_string());
        for difference in &report.differences {
            lines.push(format!("  - {}", difference.field));
        }
    }

    lines.join("\n")
}

fn render_compare_differences(report: &EvidenceCompareReport) -> String {
    report
        .differences
        .iter()
        .map(|difference| format!("- {}", difference.field))
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_gate_report(report: &EvidenceGateReport) -> String {
    match report {
        EvidenceGateReport::Envelope(report) => render_single_gate_report(report),
        EvidenceGateReport::Manifest(report) => render_manifest_gate_report(report),
    }
}

fn render_single_gate_report(report: &PolicyGateReportV1) -> String {
    let mut lines = vec![
        "Evidence policy gate:".to_string(),
        format!(
            "  policy_status: {}",
            policy_status_label(report.policy_status)
        ),
        format!(
            "  evaluated_envelope_exit_class: {}",
            exit_class_label(report.evaluated_envelope_exit_class)
        ),
        format!(
            "  gate_exit_class: {}",
            exit_class_label(report.gate_exit_class)
        ),
        format!("  payload_schema: {}", report.evaluated_payload_schema.id),
        format!("  target: {}", render_target(&report.evaluated_target)),
    ];

    if report.findings.is_empty() {
        lines.push("Findings: none".to_string());
    } else {
        lines.push("Findings:".to_string());
        for finding in &report.findings {
            lines.push(format!(
                "  - {} [{}]: {}",
                finding.code,
                policy_finding_severity_label(finding.severity),
                finding.message
            ));
        }
    }

    lines.join("\n")
}

fn render_manifest_gate_report(report: &ProjectEvidenceGateReportV1) -> String {
    let mut lines = vec![
        "Project evidence policy gate:".to_string(),
        format!("  project: {}", report.project_name),
        format!(
            "  policy_status: {}",
            policy_status_label(report.policy_status)
        ),
        format!(
            "  gate_exit_class: {}",
            exit_class_label(report.gate_exit_class)
        ),
        format!("  evidence_count: {}", report.evidence.len()),
    ];

    lines.push("Evidence:".to_string());
    for entry in &report.evidence {
        lines.push(format!(
            "  - {} {} [{}]: {}",
            entry.kind,
            entry.path,
            if entry.required {
                "required"
            } else {
                "optional"
            },
            exit_class_label(entry.gate_exit_class)
        ));
        for finding in &entry.findings {
            lines.push(format!(
                "      - {} [{}]: {}",
                finding.code,
                policy_finding_severity_label(finding.severity),
                finding.message
            ));
        }
        if let Some(policy_report) = &entry.policy_report {
            for finding in &policy_report.findings {
                lines.push(format!(
                    "      - {} [{}]: {}",
                    finding.code,
                    policy_finding_severity_label(finding.severity),
                    finding.message
                ));
            }
        }
    }

    lines.join("\n")
}

fn render_gate_findings(report: &EvidenceGateReport) -> String {
    let findings = match report {
        EvidenceGateReport::Envelope(report) => report
            .findings
            .iter()
            .map(|finding| format!("- {}: {}", finding.code, finding.message))
            .collect::<Vec<_>>(),
        EvidenceGateReport::Manifest(report) => report
            .evidence
            .iter()
            .flat_map(|entry| {
                entry
                    .findings
                    .iter()
                    .chain(
                        entry
                            .policy_report
                            .iter()
                            .flat_map(|policy_report| policy_report.findings.iter()),
                    )
                    .map(|finding| {
                        format!("- {} {}: {}", entry.path, finding.code, finding.message)
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>(),
    };

    if findings.is_empty() {
        return "no findings were emitted".to_string();
    }

    findings.join("\n")
}

fn policy_gate_summary(report: &EvidenceGateReport) -> EvidenceSummaryV1 {
    let mut summary = EvidenceSummaryV1 {
        warnings: Vec::new(),
        blocked_actions: Vec::new(),
        missing_or_stale_evidence: Vec::new(),
        evidence_conflicts: Vec::new(),
    };

    match report {
        EvidenceGateReport::Envelope(report) => {
            for finding in &report.findings {
                push_gate_summary_finding(&mut summary, finding);
            }
        }
        EvidenceGateReport::Manifest(report) => {
            for entry in &report.evidence {
                for finding in &entry.findings {
                    push_gate_summary_finding(&mut summary, finding);
                }
                if let Some(policy_report) = &entry.policy_report {
                    for finding in &policy_report.findings {
                        push_gate_summary_finding(&mut summary, finding);
                    }
                }
            }
        }
    }

    summary
}

fn push_gate_summary_finding(
    summary: &mut EvidenceSummaryV1,
    finding: &canic_host::policy_gate::PolicyFindingV1,
) {
    let message = EvidenceMessageV1::new(
        &finding.code,
        finding.message.clone(),
        match finding.severity {
            PolicyFindingSeverityV1::Info => EvidenceMessageSeverityV1::Info,
            PolicyFindingSeverityV1::Warning => EvidenceMessageSeverityV1::Warning,
            PolicyFindingSeverityV1::Error => EvidenceMessageSeverityV1::Error,
        },
    );
    match finding.subject.as_deref() {
        Some("evidence_conflict") => summary.evidence_conflicts.push(message),
        Some("missing_required_evidence") => summary.missing_or_stale_evidence.push(message),
        Some("success_with_warnings") => summary.warnings.push(message),
        _ => summary.blocked_actions.push(message),
    }
}

fn policy_gate_payload_schema(
    report: &EvidenceGateReport,
) -> canic_host::evidence_envelope::PayloadSchemaRefV1 {
    match report {
        EvidenceGateReport::Envelope(_) => policy_gate_report_schema(),
        EvidenceGateReport::Manifest(_) => project_evidence_gate_report_schema(),
    }
}

fn policy_gate_inputs(
    report: &EvidenceGateReport,
) -> Vec<canic_host::evidence_envelope::InputFingerprintV1> {
    match report {
        EvidenceGateReport::Envelope(report) => vec![
            report.policy_file_fingerprint.clone(),
            report.evaluated_envelope_fingerprint.clone(),
        ],
        EvidenceGateReport::Manifest(report) => {
            let mut inputs = vec![
                report.policy_file_fingerprint.clone(),
                report.manifest_file_fingerprint.clone(),
            ];
            inputs.extend(
                report
                    .evidence
                    .iter()
                    .filter_map(|entry| entry.evaluated_envelope_fingerprint.clone()),
            );
            inputs
        }
    }
}

fn policy_gate_target(report: &EvidenceGateReport) -> EvidenceTargetV1 {
    match report {
        EvidenceGateReport::Envelope(report) => EvidenceTargetV1 {
            kind: EvidenceTargetKindV1::PolicyGate,
            deployment: report.evaluated_target.deployment.clone(),
            fleet: report.evaluated_target.fleet.clone(),
            role: report.evaluated_target.role.clone(),
            profile: report.evaluated_target.profile.clone(),
            network: report.evaluated_target.network.clone(),
        },
        EvidenceGateReport::Manifest(report) => EvidenceTargetV1 {
            kind: EvidenceTargetKindV1::PolicyGate,
            deployment: None,
            fleet: None,
            role: None,
            profile: Some(report.project_name.clone()),
            network: None,
        },
    }
}

fn normalized_gate_args(options: &EvidenceGateOptions) -> Vec<String> {
    let mut args = vec![
        "canic".to_string(),
        "evidence".to_string(),
        "gate".to_string(),
        "--policy".to_string(),
        options.policy.display().to_string(),
    ];
    match &options.input {
        EvidenceGateInput::Envelope(path) => {
            args.push("--envelope".to_string());
            args.push(path.display().to_string());
        }
        EvidenceGateInput::Manifest(path) => {
            args.push("--manifest".to_string());
            args.push(path.display().to_string());
        }
    }
    args.extend([
        "--format".to_string(),
        match options.format {
            EvidenceGateFormat::Text => "text",
            EvidenceGateFormat::Json => "json",
            EvidenceGateFormat::EnvelopeJson => "envelope-json",
        }
        .to_string(),
    ]);
    if let Some(output) = &options.output {
        args.push("--output".to_string());
        args.push(output.display().to_string());
    }
    args
}

fn current_evidence_generated_at() -> String {
    format!(
        "unix:{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_secs())
    )
}

const fn is_success_exit_class(exit_class: ExitClassV1) -> bool {
    matches!(
        exit_class,
        ExitClassV1::Success | ExitClassV1::SuccessWithWarnings
    )
}

const fn policy_status_label(status: PolicyEvaluationStatusV1) -> &'static str {
    match status {
        PolicyEvaluationStatusV1::Passed => "passed",
        PolicyEvaluationStatusV1::Failed => "failed",
    }
}

const fn policy_finding_severity_label(severity: PolicyFindingSeverityV1) -> &'static str {
    match severity {
        PolicyFindingSeverityV1::Info => "info",
        PolicyFindingSeverityV1::Warning => "warning",
        PolicyFindingSeverityV1::Error => "error",
    }
}

const fn exit_class_label(exit_class: ExitClassV1) -> &'static str {
    match exit_class {
        ExitClassV1::Success => "success",
        ExitClassV1::SuccessWithWarnings => "success_with_warnings",
        ExitClassV1::BlockedByPolicy => "blocked_by_policy",
        ExitClassV1::EvidenceConflict => "evidence_conflict",
        ExitClassV1::MissingRequiredEvidence => "missing_required_evidence",
        ExitClassV1::InvalidInput => "invalid_input",
        ExitClassV1::ExecutionFailed => "execution_failed",
        ExitClassV1::InternalError => "internal_error",
    }
}

fn render_target(target: &EvidenceTargetV1) -> String {
    [
        target
            .deployment
            .as_ref()
            .map(|value| format!("deployment={value}")),
        target.fleet.as_ref().map(|value| format!("fleet={value}")),
        target.role.as_ref().map(|value| format!("role={value}")),
        target
            .profile
            .as_ref()
            .map(|value| format!("profile={value}")),
        target
            .network
            .as_ref()
            .map(|value| format!("network={value}")),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" ")
}

fn usage() -> String {
    let mut command = evidence_command();
    command.render_help().to_string()
}

fn compare_usage() -> String {
    let mut command = evidence_compare_command();
    command.render_help().to_string()
}

fn gate_usage() -> String {
    let mut command = evidence_gate_command();
    command.render_help().to_string()
}

fn evidence_command() -> ClapCommand {
    ClapCommand::new("evidence")
        .bin_name("canic evidence")
        .about("Evaluate and compare stable Canic evidence envelopes")
        .disable_help_flag(true)
        .subcommand(passthrough_subcommand(
            ClapCommand::new("gate")
                .about("Evaluate one EvidenceEnvelopeV1 against a CI policy")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("compare")
                .about("Compare two EvidenceEnvelopeV1 JSON files")
                .disable_help_flag(true),
        ))
}

fn evidence_gate_command() -> ClapCommand {
    ClapCommand::new("gate")
        .bin_name("canic evidence gate")
        .about("Evaluate one EvidenceEnvelopeV1 against a CI policy")
        .disable_help_flag(true)
        .arg(
            value_arg("policy")
                .long("policy")
                .value_name("path")
                .required(true),
        )
        .arg(
            value_arg("envelope")
                .long("envelope")
                .value_name("path")
                .required(false),
        )
        .arg(
            value_arg("manifest")
                .long("manifest")
                .value_name("path")
                .required(false),
        )
        .arg(
            value_arg("format")
                .long("format")
                .value_name("text|json|envelope-json")
                .default_value("text"),
        )
        .arg(value_arg("output").long("output").value_name("path"))
        .group(
            ArgGroup::new("gate-input")
                .args(["envelope", "manifest"])
                .required(true)
                .multiple(false),
        )
        .after_help(
            "Reads exactly one policy file and either one existing EvidenceEnvelopeV1 or one project evidence manifest. The gate is passive: it does not run builds, deploy, discover live state, mutate inputs, or turn policy success into deployment truth.",
        )
}

fn evidence_compare_command() -> ClapCommand {
    ClapCommand::new("compare")
        .bin_name("canic evidence compare")
        .about("Compare two EvidenceEnvelopeV1 JSON files")
        .disable_help_flag(true)
        .arg(
            value_arg("left")
                .long("left")
                .value_name("file")
                .required(true),
        )
        .arg(
            value_arg("right")
                .long("right")
                .value_name("file")
                .required(true),
        )
        .arg(
            value_arg("format")
                .long("format")
                .value_name("text|json")
                .default_value("text"),
        )
        .after_help(
            "Compares stable envelope fields and ignores generated_at, canic_version, and the nested payload body. The payload_sha256 field is compared.",
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_dir;
    use canic_host::evidence_envelope::{
        CommandProvenanceV1, EvidenceMessageSeverityV1, EvidenceMessageV1, EvidenceSummaryV1,
        EvidenceTargetKindV1, EvidenceTargetV1, ExitClassV1, adoption_report_schema,
        evidence_envelope_schema,
    };
    use serde_json::json;
    use std::{fs, time::UNIX_EPOCH};

    #[test]
    fn parses_compare_options() {
        let options = EvidenceCompareOptions::parse([
            OsString::from("--left"),
            OsString::from("left.json"),
            OsString::from("--right"),
            OsString::from("right.json"),
            OsString::from("--format"),
            OsString::from("json"),
        ])
        .expect("parse options");

        assert_eq!(options.left, PathBuf::from("left.json"));
        assert_eq!(options.right, PathBuf::from("right.json"));
        assert_eq!(options.format, EvidenceCompareFormat::Json);
    }

    #[test]
    fn parses_gate_options() {
        let options = EvidenceGateOptions::parse([
            OsString::from("--policy"),
            OsString::from("policy.toml"),
            OsString::from("--envelope"),
            OsString::from("evidence.json"),
            OsString::from("--format"),
            OsString::from("envelope-json"),
            OsString::from("--output"),
            OsString::from("gate.json"),
        ])
        .expect("parse options");

        assert_eq!(options.policy, PathBuf::from("policy.toml"));
        assert_eq!(
            options.input,
            EvidenceGateInput::Envelope(PathBuf::from("evidence.json"))
        );
        assert_eq!(options.format, EvidenceGateFormat::EnvelopeJson);
        assert_eq!(options.output, Some(PathBuf::from("gate.json")));
    }

    #[test]
    fn parses_gate_manifest_options() {
        let options = EvidenceGateOptions::parse([
            OsString::from("--policy"),
            OsString::from("policy.toml"),
            OsString::from("--manifest"),
            OsString::from("evidence.toml"),
        ])
        .expect("parse options");

        assert_eq!(
            options.input,
            EvidenceGateInput::Manifest(PathBuf::from("evidence.toml"))
        );
        assert_eq!(options.format, EvidenceGateFormat::Text);
    }

    #[test]
    fn gate_rejects_envelope_and_manifest_together() {
        let err = EvidenceGateOptions::parse([
            OsString::from("--policy"),
            OsString::from("policy.toml"),
            OsString::from("--envelope"),
            OsString::from("evidence.json"),
            OsString::from("--manifest"),
            OsString::from("evidence.toml"),
        ])
        .expect_err("conflicting gate inputs fail");

        assert!(matches!(err, EvidenceCommandError::Usage(_)));
    }

    #[test]
    fn compare_ignores_timestamp_version_and_payload_body() {
        let left_path = PathBuf::from("left.json");
        let right_path = PathBuf::from("right.json");
        let mut left = sample_envelope();
        let mut right = sample_envelope();
        right.canic_version = "different".to_string();
        right.generated_at = "2030-01-01T00:00:00Z".to_string();
        right.payload = json!({ "changed": true });

        let report = compare_envelopes(&left, &right, &left_path, &right_path);

        assert_eq!(report.status, EvidenceCompareStatus::Matched);
        assert!(report.differences.is_empty());
        left.payload_sha256 = Some("left-hash".to_string());
        right.payload_sha256 = Some("right-hash".to_string());

        let report = compare_envelopes(&left, &right, &left_path, &right_path);

        assert_eq!(report.status, EvidenceCompareStatus::Different);
        assert_eq!(report.differences[0].field, "payload_sha256");
    }

    #[test]
    fn compare_detects_exit_class_and_summary_differences() {
        let left_path = PathBuf::from("left.json");
        let right_path = PathBuf::from("right.json");
        let left = sample_envelope();
        let mut right = sample_envelope();
        right.exit_class = ExitClassV1::EvidenceConflict;
        right
            .summary
            .evidence_conflicts
            .push(EvidenceMessageV1::new(
                "test.conflict",
                "conflict",
                EvidenceMessageSeverityV1::Error,
            ));

        let report = compare_envelopes(&left, &right, &left_path, &right_path);

        assert_eq!(report.status, EvidenceCompareStatus::Different);
        assert!(
            report
                .differences
                .iter()
                .any(|difference| difference.field == "exit_class")
        );
        assert!(
            report
                .differences
                .iter()
                .any(|difference| difference.field == "summary")
        );
    }

    #[test]
    fn compare_files_returns_error_for_differences_after_writing_report() {
        let left_path = temp_json_path("canic-evidence-left");
        let right_path = temp_json_path("canic-evidence-right");
        let left = sample_envelope();
        let mut right = sample_envelope();
        right.exit_class = ExitClassV1::BlockedByPolicy;
        fs::write(&left_path, serde_json::to_vec(&left).expect("encode left")).expect("write left");
        fs::write(
            &right_path,
            serde_json::to_vec(&right).expect("encode right"),
        )
        .expect("write right");
        let options = EvidenceCompareOptions {
            left: left_path.clone(),
            right: right_path.clone(),
            format: EvidenceCompareFormat::Text,
        };

        let report = compare_envelope_files(&options).expect("compare files");

        fs::remove_file(left_path).expect("clean left");
        fs::remove_file(right_path).expect("clean right");
        assert_eq!(report.status, EvidenceCompareStatus::Different);
        assert_eq!(report.differences[0].field, "exit_class");
    }

    #[test]
    fn gate_files_evaluate_policy_and_json_output() {
        let root = temp_dir("canic-evidence-gate-json");
        fs::create_dir_all(&root).expect("create root");
        let policy = root.join("policy.toml");
        let envelope = root.join("envelope.json");
        let output = root.join("gate.json");
        fs::write(&policy, MINIMAL_POLICY).expect("write policy");
        fs::write(
            &envelope,
            serde_json::to_vec(&sample_envelope()).expect("encode envelope"),
        )
        .expect("write envelope");
        let options = EvidenceGateOptions {
            policy,
            input: EvidenceGateInput::Envelope(envelope),
            format: EvidenceGateFormat::Json,
            output: Some(output.clone()),
        };

        let report = evaluate_gate_files(&options).expect("evaluate gate");
        write_gate_report(&options, &report).expect("write gate");
        let written = fs::read_to_string(&output).expect("read output");

        fs::remove_dir_all(root).expect("clean");
        let EvidenceGateReport::Envelope(report) = report else {
            panic!("expected single envelope gate report");
        };
        assert_eq!(report.policy_status, PolicyEvaluationStatusV1::Passed);
        assert_eq!(report.gate_exit_class, ExitClassV1::Success);
        assert!(written.contains("\"policy_status\": \"passed\""));
    }

    #[test]
    fn gate_envelope_wraps_policy_report() {
        let root = temp_dir("canic-evidence-gate-envelope");
        fs::create_dir_all(&root).expect("create root");
        let policy = root.join("policy.toml");
        let envelope = root.join("envelope.json");
        fs::write(&policy, MINIMAL_POLICY).expect("write policy");
        fs::write(
            &envelope,
            serde_json::to_vec(&sample_envelope()).expect("encode envelope"),
        )
        .expect("write envelope");
        let options = EvidenceGateOptions {
            policy,
            input: EvidenceGateInput::Envelope(envelope),
            format: EvidenceGateFormat::EnvelopeJson,
            output: None,
        };
        let report = evaluate_gate_files(&options).expect("evaluate gate");

        let gate = policy_gate_envelope(&options, &report).expect("wrap gate");

        fs::remove_dir_all(root).expect("clean");
        assert_eq!(gate.target.kind, EvidenceTargetKindV1::PolicyGate);
        assert_eq!(gate.target.fleet.as_deref(), Some("demo"));
        assert_eq!(gate.payload_schema.id, "canic.policy_gate_report.v1");
        assert_eq!(gate.exit_class, ExitClassV1::Success);
        assert_eq!(gate.inputs.len(), 2);
    }

    #[test]
    fn gate_manifest_evaluates_project_evidence_and_wraps_report() {
        let root = temp_dir("canic-evidence-gate-manifest");
        fs::create_dir_all(&root).expect("create root");
        let policy = root.join("policy.toml");
        let manifest = root.join("evidence.toml");
        let envelope = root.join("adoption.json");
        fs::write(&policy, MINIMAL_POLICY).expect("write policy");
        fs::write(
            &envelope,
            serde_json::to_vec(&sample_envelope()).expect("encode envelope"),
        )
        .expect("write envelope");
        fs::write(&manifest, sample_manifest()).expect("write manifest");
        let options = EvidenceGateOptions {
            policy,
            input: EvidenceGateInput::Manifest(manifest),
            format: EvidenceGateFormat::EnvelopeJson,
            output: None,
        };

        let report = evaluate_gate_files(&options).expect("evaluate manifest gate");
        let envelope = policy_gate_envelope(&options, &report).expect("wrap manifest gate");

        fs::remove_dir_all(root).expect("clean");
        let EvidenceGateReport::Manifest(report) = report else {
            panic!("expected manifest gate report");
        };
        assert_eq!(report.policy_status, PolicyEvaluationStatusV1::Passed);
        assert_eq!(report.gate_exit_class, ExitClassV1::Success);
        assert_eq!(report.evidence.len(), 1);
        assert_eq!(
            envelope.payload_schema.id,
            "canic.project_evidence_gate_report.v1"
        );
        assert_eq!(envelope.target.profile.as_deref(), Some("demo"));
        assert_eq!(envelope.inputs.len(), 3);
    }

    #[test]
    fn gate_writes_report_before_failure_can_be_returned() {
        let root = temp_dir("canic-evidence-gate-failure");
        fs::create_dir_all(&root).expect("create root");
        let policy = root.join("policy.toml");
        let envelope = root.join("envelope.json");
        let output = root.join("gate.json");
        let mut failing = sample_envelope();
        failing.exit_class = ExitClassV1::SuccessWithWarnings;
        fs::write(&policy, MINIMAL_POLICY).expect("write policy");
        fs::write(
            &envelope,
            serde_json::to_vec(&failing).expect("encode envelope"),
        )
        .expect("write envelope");
        let options = EvidenceGateOptions {
            policy,
            input: EvidenceGateInput::Envelope(envelope),
            format: EvidenceGateFormat::Json,
            output: Some(output.clone()),
        };

        let report = evaluate_gate_files(&options).expect("evaluate gate");
        write_gate_report(&options, &report).expect("write failing report");
        let written = fs::read_to_string(&output).expect("read output");

        fs::remove_dir_all(root).expect("clean");
        let EvidenceGateReport::Envelope(report) = report else {
            panic!("expected single envelope gate report");
        };
        assert_eq!(report.policy_status, PolicyEvaluationStatusV1::Failed);
        assert_eq!(report.gate_exit_class, ExitClassV1::BlockedByPolicy);
        assert!(written.contains("\"gate_exit_class\": \"blocked_by_policy\""));
    }

    fn sample_envelope() -> EvidenceEnvelopeV1 {
        EvidenceEnvelopeV1 {
            envelope_schema: evidence_envelope_schema(),
            canic_version: env!("CARGO_PKG_VERSION").to_string(),
            command: CommandProvenanceV1 {
                name: "canic fleet adoption report".to_string(),
                argv_normalized: vec![
                    "canic".to_string(),
                    "fleet".to_string(),
                    "adoption".to_string(),
                    "report".to_string(),
                    "demo".to_string(),
                ],
                argv_redactions: Vec::new(),
                format: "envelope-json".to_string(),
            },
            target: EvidenceTargetV1 {
                kind: EvidenceTargetKindV1::FleetAdoption,
                deployment: None,
                fleet: Some("demo".to_string()),
                role: None,
                profile: Some("minimal".to_string()),
                network: None,
            },
            generated_at: "2026-05-31T00:00:00Z".to_string(),
            source_config: None,
            inputs: Vec::new(),
            payload_schema: adoption_report_schema(),
            payload_sha256: Some(sample_sha256("payload")),
            payload: json!({ "report_id": "report-1" }),
            summary: EvidenceSummaryV1 {
                warnings: Vec::new(),
                blocked_actions: Vec::new(),
                missing_or_stale_evidence: Vec::new(),
                evidence_conflicts: Vec::new(),
            },
            exit_class: ExitClassV1::Success,
        }
    }

    fn sample_sha256(label: &str) -> String {
        let mut hash = String::new();
        while hash.len() < 64 {
            hash.push_str(label);
        }
        hash.truncate(64);
        hash
    }

    fn sample_manifest() -> String {
        r#"
schema_version = 1

[project]
name = "demo"
root = "."

[[evidence]]
kind = "adoption_report"
path = "adoption.json"
required = true
payload_schema = "canic.adoption_report.v1"

[evidence.target]
fleet = "demo"
profile = "minimal"
"#
        .to_string()
    }

    fn temp_json_path(name: &str) -> PathBuf {
        let suffix = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("{name}-{suffix}.json"))
    }

    const MINIMAL_POLICY: &str = r#"
schema_version = 1

[envelope]
required_schema = "canic.evidence_envelope.v1"

[exit_class]
allowed = ["success"]
"#;
}
