//! Module: evidence::gate
//!
//! Responsibility: evaluate policy gates and render/write gate reports.
//! Does not own: command dispatch, option parsing, or envelope comparison.
//! Boundary: policy-gate file IO, gate report envelopes, summaries, and text output.

use crate::output;
use canic_host::{
    evidence_envelope::{
        CommandProvenanceV1, EvidenceEnvelopeV1, EvidenceMessageSeverityV1, EvidenceMessageV1,
        EvidenceSummaryV1, EvidenceTargetKindV1, EvidenceTargetV1, ExitClassV1, InputFingerprintV1,
        PayloadSchemaRefV1, evidence_envelope_schema, json_payload_sha256,
        policy_gate_report_schema, project_evidence_gate_report_schema,
    },
    policy_gate::{
        PolicyEvaluationStatusV1, PolicyFindingSeverityV1, PolicyFindingV1, PolicyGateReportV1,
        PolicyGateRequest, ProjectEvidenceGateReportV1, ProjectEvidenceManifestGateRequest,
        evaluate_policy_gate, evaluate_project_evidence_manifest_gate,
    },
};
use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use super::{
    EvidenceCommandError,
    options::{EvidenceGateFormat, EvidenceGateInput, EvidenceGateOptions},
};

///
/// EvidenceGateReport
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum EvidenceGateReport {
    Envelope(PolicyGateReportV1),
    Manifest(ProjectEvidenceGateReportV1),
}

impl EvidenceGateReport {
    pub(super) const fn gate_exit_class(&self) -> ExitClassV1 {
        match self {
            Self::Envelope(report) => report.gate_exit_class,
            Self::Manifest(report) => report.gate_exit_class,
        }
    }
}

pub(super) fn evaluate_gate_files(
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

pub(super) fn write_gate_report(
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

pub(super) fn policy_gate_envelope(
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

pub(super) fn render_gate_findings(report: &EvidenceGateReport) -> String {
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

fn push_gate_summary_finding(summary: &mut EvidenceSummaryV1, finding: &PolicyFindingV1) {
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

fn policy_gate_payload_schema(report: &EvidenceGateReport) -> PayloadSchemaRefV1 {
    match report {
        EvidenceGateReport::Envelope(_) => policy_gate_report_schema(),
        EvidenceGateReport::Manifest(_) => project_evidence_gate_report_schema(),
    }
}

fn policy_gate_inputs(report: &EvidenceGateReport) -> Vec<InputFingerprintV1> {
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

pub(super) const fn is_success_exit_class(exit_class: ExitClassV1) -> bool {
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
