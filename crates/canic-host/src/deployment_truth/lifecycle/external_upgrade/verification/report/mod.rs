use super::super::super::super::*;
use super::super::super::digest::*;
use super::super::super::error::{
    ExternalUpgradeReceiptError, ExternalUpgradeVerificationReportError,
};
use super::super::receipt::validate_external_upgrade_receipt_for_proposal;
use super::super::validation::ensure_external_verification_report_field;
use super::shared::external_upgrade_verification_summary;

/// Build a passive verification report for a proposal/receipt pair.
///
/// This packages structural verification evidence only. Live inventory remains
/// the source of truth for deployment state.
pub fn external_upgrade_verification_report_from_receipt(
    report_id: impl Into<String>,
    proposal: &ExternalUpgradeProposalV1,
    receipt: &ExternalUpgradeReceiptV1,
) -> Result<ExternalUpgradeVerificationReportV1, ExternalUpgradeReceiptError> {
    validate_external_upgrade_receipt_for_proposal(receipt, proposal)?;
    let verification_result = receipt.verification_result;
    let mut report = ExternalUpgradeVerificationReportV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        report_id: report_id.into(),
        report_digest: String::new(),
        proposal_id: proposal.proposal_id.clone(),
        proposal_digest: proposal.proposal_digest.clone(),
        receipt_id: receipt.receipt_id.clone(),
        receipt_digest: receipt.receipt_digest.clone(),
        subject: proposal.subject.clone(),
        canister_id: proposal.canister_id.clone(),
        role: proposal.role.clone(),
        verification_result,
        verification_notes: receipt.verification_notes.clone(),
        live_inventory_required: verification_result
            != ExternalUpgradeVerificationResultV1::Pending
            && verification_result != ExternalUpgradeVerificationResultV1::Refused,
        status_summary: external_upgrade_verification_summary(verification_result).to_string(),
    };
    report.report_digest = external_upgrade_verification_report_digest(&report);
    Ok(report)
}

/// Validate archived external-upgrade verification report consistency and
/// digest.
pub fn validate_external_upgrade_verification_report(
    report: &ExternalUpgradeVerificationReportV1,
) -> Result<(), ExternalUpgradeVerificationReportError> {
    if report.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(
            ExternalUpgradeVerificationReportError::SchemaVersionMismatch {
                expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
                actual: report.schema_version,
            },
        );
    }
    ensure_external_verification_report_field("report_id", report.report_id.as_str())?;
    ensure_external_verification_report_field("report_digest", report.report_digest.as_str())?;
    ensure_external_verification_report_field("proposal_id", report.proposal_id.as_str())?;
    ensure_external_verification_report_field("proposal_digest", report.proposal_digest.as_str())?;
    ensure_external_verification_report_field("receipt_id", report.receipt_id.as_str())?;
    ensure_external_verification_report_field("receipt_digest", report.receipt_digest.as_str())?;
    ensure_external_verification_report_field("subject", report.subject.as_str())?;
    ensure_external_verification_report_field("status_summary", report.status_summary.as_str())?;
    if report.status_summary != external_upgrade_verification_summary(report.verification_result) {
        return Err(ExternalUpgradeVerificationReportError::SourceMismatch {
            field: "status_summary",
        });
    }
    if report.report_digest != external_upgrade_verification_report_digest(report) {
        return Err(ExternalUpgradeVerificationReportError::DigestMismatch {
            field: "report_digest",
        });
    }
    Ok(())
}

/// Validate that an archived verification report still matches the
/// proposal/receipt pair it claims to summarize.
pub fn validate_external_upgrade_verification_report_for_receipt(
    report: &ExternalUpgradeVerificationReportV1,
    proposal: &ExternalUpgradeProposalV1,
    receipt: &ExternalUpgradeReceiptV1,
) -> Result<(), ExternalUpgradeVerificationReportError> {
    validate_external_upgrade_verification_report(report)?;
    let expected = external_upgrade_verification_report_from_receipt(
        report.report_id.clone(),
        proposal,
        receipt,
    )?;
    if report != &expected {
        return Err(ExternalUpgradeVerificationReportError::SourceMismatch { field: "receipt" });
    }
    Ok(())
}
