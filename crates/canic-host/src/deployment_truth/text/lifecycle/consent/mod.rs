use super::super::super::*;
use super::super::optional_text;
use super::shared::external_upgrade_consent_state_label;

/// Render external-upgrade consent evidence as passive operator text.
#[must_use]
pub fn external_upgrade_consent_evidence_text(
    evidence: &ExternalUpgradeConsentEvidenceV1,
) -> String {
    [
        "External upgrade consent evidence".to_string(),
        "mode: passive".to_string(),
        "execution: none".to_string(),
        format!("evidence_id: {}", evidence.evidence_id),
        format!("evidence_digest: {}", evidence.evidence_digest),
        format!("proposal_id: {}", evidence.proposal_id),
        format!("proposal_digest: {}", evidence.proposal_digest),
        format!("receipt_id: {}", evidence.receipt_id),
        format!("receipt_digest: {}", evidence.receipt_digest),
        format!("subject: {}", evidence.subject),
        format!("role: {}", optional_text(evidence.role.as_deref())),
        format!(
            "canister_id: {}",
            optional_text(evidence.canister_id.as_deref())
        ),
        format!(
            "consent_state: {}",
            external_upgrade_consent_state_label(evidence.consent_state)
        ),
        format!(
            "reported_by: {}",
            optional_text(evidence.reported_by.as_deref())
        ),
        format!("status_summary: {}", evidence.status_summary),
        format!(
            "consent_requirements: {}",
            evidence.consent_requirements.len()
        ),
        format!(
            "allowed_authorization_modes: {}",
            evidence.allowed_authorization_modes.len()
        ),
    ]
    .join("\n")
}
