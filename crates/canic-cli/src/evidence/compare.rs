//! Module: evidence::compare
//!
//! Responsibility: compare stable EvidenceEnvelopeV1 fields and render compare reports.
//! Does not own: command dispatch, option parsing, policy gate evaluation, or gate rendering.
//! Boundary: deterministic evidence-envelope comparison and compare output.

use crate::output;
use canic_host::evidence_envelope::EvidenceEnvelopeV1;
use serde::Serialize;
use std::path::Path;

use super::{
    EvidenceCommandError,
    options::{EvidenceCompareFormat, EvidenceCompareOptions},
};

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
/// EvidenceCompareStatus
///
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum EvidenceCompareStatus {
    Matched,
    Different,
}

///
/// EvidenceCompareDifference
///
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct EvidenceCompareDifference {
    pub(super) field: String,
    pub(super) left: serde_json::Value,
    pub(super) right: serde_json::Value,
}

///
/// EvidenceCompareReport
///
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct EvidenceCompareReport {
    pub(super) schema_version: u32,
    pub(super) status: EvidenceCompareStatus,
    pub(super) left: String,
    pub(super) right: String,
    pub(super) compared_fields: Vec<String>,
    pub(super) ignored_fields: Vec<String>,
    pub(super) differences: Vec<EvidenceCompareDifference>,
}

pub(super) fn compare_envelope_files(
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

pub(super) fn compare_envelopes(
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

pub(super) fn write_compare_report(
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

pub(super) fn render_compare_differences(report: &EvidenceCompareReport) -> String {
    report
        .differences
        .iter()
        .map(|difference| format!("- {}", difference.field))
        .collect::<Vec<_>>()
        .join("\n")
}
