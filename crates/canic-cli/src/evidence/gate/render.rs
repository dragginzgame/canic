//! Module: evidence::gate::render
//!
//! Responsibility: render policy-gate reports and failure findings as text.
//! Does not own: policy evaluation, envelope construction, or file output.
//! Boundary: deterministic display formatting for evidence gate results.

use canic_host::{
    evidence_envelope::{EvidenceTargetV1, ExitClassV1},
    policy_gate::{
        PolicyEvaluationStatusV1, PolicyFindingSeverityV1, PolicyGateReportV1,
        ProjectEvidenceGateReportV1,
    },
};

use super::EvidenceGateReport;

pub(super) fn render_gate_report(report: &EvidenceGateReport) -> String {
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

pub(in crate::evidence) fn render_gate_findings(report: &EvidenceGateReport) -> String {
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
