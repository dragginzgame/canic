//! Module: evidence::gate::envelope
//!
//! Responsibility: wrap policy-gate reports into stable EvidenceEnvelopeV1 values.
//! Does not own: policy evaluation, option parsing, or text rendering.
//! Boundary: evidence envelope construction for `canic evidence gate`.

use canic_host::{
    evidence_envelope::{
        CommandProvenanceV1, EvidenceEnvelopeV1, EvidenceMessageSeverityV1, EvidenceMessageV1,
        EvidenceSummaryV1, EvidenceTargetKindV1, EvidenceTargetV1, InputFingerprintV1,
        PayloadSchemaRefV1, evidence_envelope_schema, json_payload_sha256,
        policy_gate_report_schema, project_evidence_gate_report_schema,
    },
    policy_gate::{PolicyFindingSeverityV1, PolicyFindingV1},
};
use std::time::{SystemTime, UNIX_EPOCH};

use super::EvidenceGateReport;
use crate::evidence::{
    EvidenceCommandError,
    options::{EvidenceGateFormat, EvidenceGateInput, EvidenceGateOptions},
};

pub(in crate::evidence) fn policy_gate_envelope(
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
    match options.format {
        EvidenceGateFormat::Text => {}
        EvidenceGateFormat::Json => args.push("--json".to_string()),
        EvidenceGateFormat::EnvelopeJson => args.push("--evidence-envelope".to_string()),
    }
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
