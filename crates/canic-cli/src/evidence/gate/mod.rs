//! Module: evidence::gate
//!
//! Responsibility: evaluate policy gates and write selected gate report formats.
//! Does not own: command dispatch, option parsing, envelope construction, or text rendering.
//! Boundary: policy-gate file IO and output-format orchestration for evidence commands.

mod envelope;
mod render;

use crate::output;
use canic_host::{
    evidence_envelope::{EvidenceEnvelopeV1, ExitClassV1},
    policy_gate::{
        PolicyGateReportV1, PolicyGateRequest, ProjectEvidenceGateReportV1,
        ProjectEvidenceManifestGateRequest, evaluate_policy_gate,
        evaluate_project_evidence_manifest_gate,
    },
};
use std::fs;

use super::{
    EvidenceCommandError,
    options::{EvidenceGateFormat, EvidenceGateInput, EvidenceGateOptions},
};
pub(super) use envelope::policy_gate_envelope;
pub(super) use render::render_gate_findings;
use render::render_gate_report;

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

pub(super) const fn is_success_exit_class(exit_class: ExitClassV1) -> bool {
    matches!(
        exit_class,
        ExitClassV1::Success | ExitClassV1::SuccessWithWarnings
    )
}
