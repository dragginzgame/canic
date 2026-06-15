use super::super::super::*;
use super::super::error::{
    ExternalUpgradeCompletionReportError, ExternalUpgradeConsentEvidenceError,
    ExternalUpgradeProposalReportError, ExternalUpgradeReceiptError,
    ExternalUpgradeVerificationCheckError, ExternalUpgradeVerificationPolicyError,
    ExternalUpgradeVerificationReportError,
};

pub(super) fn ensure_external_receipt_field(
    field: &'static str,
    value: &str,
) -> Result<(), ExternalUpgradeReceiptError> {
    if value.trim().is_empty() {
        return Err(ExternalUpgradeReceiptError::MissingRequiredField { field });
    }
    Ok(())
}

pub(super) fn ensure_external_receipt_matches_proposal(
    field: &'static str,
    actual: &str,
    expected: &str,
) -> Result<(), ExternalUpgradeReceiptError> {
    if actual != expected {
        return Err(ExternalUpgradeReceiptError::SourceMismatch { field });
    }
    Ok(())
}

pub(super) fn ensure_external_receipt_option_matches_proposal(
    field: &'static str,
    actual: Option<&str>,
    expected: Option<&str>,
) -> Result<(), ExternalUpgradeReceiptError> {
    if actual != expected {
        return Err(ExternalUpgradeReceiptError::SourceMismatch { field });
    }
    Ok(())
}

pub(super) fn ensure_external_consent_evidence_field(
    field: &'static str,
    value: &str,
) -> Result<(), ExternalUpgradeConsentEvidenceError> {
    if value.trim().is_empty() {
        return Err(ExternalUpgradeConsentEvidenceError::MissingRequiredField { field });
    }
    Ok(())
}

pub(super) fn ensure_external_verification_report_field(
    field: &'static str,
    value: &str,
) -> Result<(), ExternalUpgradeVerificationReportError> {
    if value.trim().is_empty() {
        return Err(ExternalUpgradeVerificationReportError::MissingRequiredField { field });
    }
    Ok(())
}

pub(super) fn ensure_external_verification_policy_field(
    field: &'static str,
    value: &str,
) -> Result<(), ExternalUpgradeVerificationPolicyError> {
    if value.trim().is_empty() {
        return Err(ExternalUpgradeVerificationPolicyError::MissingRequiredField { field });
    }
    Ok(())
}

pub(super) fn ensure_external_verification_check_field(
    field: &'static str,
    value: &str,
) -> Result<(), ExternalUpgradeVerificationCheckError> {
    if value.trim().is_empty() {
        return Err(ExternalUpgradeVerificationCheckError::MissingRequiredField { field });
    }
    Ok(())
}

pub(super) fn ensure_external_verification_check_option_field(
    field: &'static str,
    value: Option<&str>,
) -> Result<(), ExternalUpgradeVerificationCheckError> {
    if value.is_none_or(|value| value.trim().is_empty()) {
        return Err(ExternalUpgradeVerificationCheckError::MissingRequiredField { field });
    }
    Ok(())
}

pub(super) fn ensure_external_completion_report_field(
    field: &'static str,
    value: &str,
) -> Result<(), ExternalUpgradeCompletionReportError> {
    if value.trim().is_empty() {
        return Err(ExternalUpgradeCompletionReportError::MissingRequiredField { field });
    }
    Ok(())
}

pub(super) fn ensure_completion_sources_match_proposal(
    proposal: &ExternalUpgradeProposalV1,
    consent_evidence: &ExternalUpgradeConsentEvidenceV1,
    verification_check: &ExternalUpgradeVerificationCheckV1,
) -> Result<(), ExternalUpgradeCompletionReportError> {
    ensure_completion_source_field(
        "consent_evidence.proposal_id",
        consent_evidence.proposal_id.as_str(),
        proposal.proposal_id.as_str(),
    )?;
    ensure_completion_source_field(
        "consent_evidence.proposal_digest",
        consent_evidence.proposal_digest.as_str(),
        proposal.proposal_digest.as_str(),
    )?;
    ensure_completion_source_field(
        "verification_check.proposal_id",
        verification_check.proposal_id.as_str(),
        proposal.proposal_id.as_str(),
    )?;
    ensure_completion_source_field(
        "verification_check.proposal_digest",
        verification_check.proposal_digest.as_str(),
        proposal.proposal_digest.as_str(),
    )?;
    ensure_completion_source_field(
        "consent_evidence.subject",
        consent_evidence.subject.as_str(),
        proposal.subject.as_str(),
    )?;
    ensure_completion_source_field(
        "verification_check.subject",
        verification_check.subject.as_str(),
        proposal.subject.as_str(),
    )?;
    ensure_completion_option_source_field(
        "consent_evidence.canister_id",
        consent_evidence.canister_id.as_deref(),
        proposal.canister_id.as_deref(),
    )?;
    ensure_completion_option_source_field(
        "verification_check.canister_id",
        verification_check.canister_id.as_deref(),
        proposal.canister_id.as_deref(),
    )?;
    ensure_completion_option_source_field(
        "consent_evidence.role",
        consent_evidence.role.as_deref(),
        proposal.role.as_deref(),
    )?;
    ensure_completion_option_source_field(
        "verification_check.role",
        verification_check.role.as_deref(),
        proposal.role.as_deref(),
    )
}

pub(super) fn ensure_external_proposal_report_field(
    field: &'static str,
    value: &str,
) -> Result<(), ExternalUpgradeProposalReportError> {
    if value.trim().is_empty() {
        return Err(ExternalUpgradeProposalReportError::MissingRequiredField { field });
    }
    Ok(())
}

fn ensure_completion_source_field(
    field: &'static str,
    actual: &str,
    expected: &str,
) -> Result<(), ExternalUpgradeCompletionReportError> {
    if actual != expected {
        return Err(ExternalUpgradeCompletionReportError::SourceMismatch { field });
    }
    Ok(())
}

fn ensure_completion_option_source_field(
    field: &'static str,
    actual: Option<&str>,
    expected: Option<&str>,
) -> Result<(), ExternalUpgradeCompletionReportError> {
    if actual != expected {
        return Err(ExternalUpgradeCompletionReportError::SourceMismatch { field });
    }
    Ok(())
}
