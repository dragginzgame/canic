use super::super::super::*;
use super::super::digest::*;
use super::super::error::{ExternalUpgradeConsentEvidenceError, ExternalUpgradeReceiptError};
use super::receipt::validate_external_upgrade_receipt_for_proposal;
use super::validation::ensure_external_consent_evidence_field;

/// Build passive consent/action evidence from a proposal/receipt pair.
///
/// This records the reported consent or external action state only. It is not
/// completion proof; verification remains separate and live inventory remains
/// the source of deployment truth.
pub fn external_upgrade_consent_evidence_from_receipt(
    evidence_id: impl Into<String>,
    proposal: &ExternalUpgradeProposalV1,
    receipt: &ExternalUpgradeReceiptV1,
) -> Result<ExternalUpgradeConsentEvidenceV1, ExternalUpgradeReceiptError> {
    validate_external_upgrade_receipt_for_proposal(receipt, proposal)?;
    let consent_state = receipt.consent_state;
    let mut evidence = ExternalUpgradeConsentEvidenceV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        evidence_id: evidence_id.into(),
        evidence_digest: String::new(),
        proposal_id: proposal.proposal_id.clone(),
        proposal_digest: proposal.proposal_digest.clone(),
        receipt_id: receipt.receipt_id.clone(),
        receipt_digest: receipt.receipt_digest.clone(),
        subject: proposal.subject.clone(),
        canister_id: proposal.canister_id.clone(),
        role: proposal.role.clone(),
        consent_state,
        reported_by: receipt.reported_by.clone(),
        consent_requirements: proposal.consent_requirements.clone(),
        allowed_authorization_modes: proposal.allowed_authorization_modes.clone(),
        status_summary: external_upgrade_consent_summary(consent_state).to_string(),
    };
    evidence.evidence_digest = external_upgrade_consent_evidence_digest(&evidence);
    Ok(evidence)
}

/// Validate archived consent evidence consistency and digest.
pub fn validate_external_upgrade_consent_evidence(
    evidence: &ExternalUpgradeConsentEvidenceV1,
) -> Result<(), ExternalUpgradeConsentEvidenceError> {
    if evidence.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(ExternalUpgradeConsentEvidenceError::SchemaVersionMismatch {
            expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            actual: evidence.schema_version,
        });
    }
    ensure_external_consent_evidence_field("evidence_id", evidence.evidence_id.as_str())?;
    ensure_external_consent_evidence_field("evidence_digest", evidence.evidence_digest.as_str())?;
    ensure_external_consent_evidence_field("proposal_id", evidence.proposal_id.as_str())?;
    ensure_external_consent_evidence_field("proposal_digest", evidence.proposal_digest.as_str())?;
    ensure_external_consent_evidence_field("receipt_id", evidence.receipt_id.as_str())?;
    ensure_external_consent_evidence_field("receipt_digest", evidence.receipt_digest.as_str())?;
    ensure_external_consent_evidence_field("subject", evidence.subject.as_str())?;
    ensure_external_consent_evidence_field("status_summary", evidence.status_summary.as_str())?;
    if evidence.status_summary != external_upgrade_consent_summary(evidence.consent_state) {
        return Err(ExternalUpgradeConsentEvidenceError::SourceMismatch {
            field: "status_summary",
        });
    }
    if evidence.evidence_digest != external_upgrade_consent_evidence_digest(evidence) {
        return Err(ExternalUpgradeConsentEvidenceError::DigestMismatch {
            field: "evidence_digest",
        });
    }
    Ok(())
}

/// Validate that archived consent evidence still matches the proposal/receipt
/// pair it claims to summarize.
pub fn validate_external_upgrade_consent_evidence_for_receipt(
    evidence: &ExternalUpgradeConsentEvidenceV1,
    proposal: &ExternalUpgradeProposalV1,
    receipt: &ExternalUpgradeReceiptV1,
) -> Result<(), ExternalUpgradeConsentEvidenceError> {
    validate_external_upgrade_consent_evidence(evidence)?;
    let expected = external_upgrade_consent_evidence_from_receipt(
        evidence.evidence_id.clone(),
        proposal,
        receipt,
    )?;
    if evidence != &expected {
        return Err(ExternalUpgradeConsentEvidenceError::SourceMismatch { field: "receipt" });
    }
    Ok(())
}

pub(super) const fn external_upgrade_consent_summary(
    state: ExternalUpgradeConsentStateV1,
) -> &'static str {
    match state {
        ExternalUpgradeConsentStateV1::Pending => {
            "external consent or action has not been reported"
        }
        ExternalUpgradeConsentStateV1::Refused => "external consent was refused",
        ExternalUpgradeConsentStateV1::Delegated => "delegated install authority was reported",
        ExternalUpgradeConsentStateV1::ExecutedExternally => {
            "external controller execution was reported"
        }
    }
}
