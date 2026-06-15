use super::super::super::*;
use super::super::optional_text;
use super::shared::{
    external_upgrade_consent_state_label, external_upgrade_verification_result_label,
};

/// Render an external-upgrade receipt as passive operator text.
#[must_use]
pub fn external_upgrade_receipt_text(receipt: &ExternalUpgradeReceiptV1) -> String {
    [
        "External upgrade receipt".to_string(),
        "mode: passive".to_string(),
        "execution: none".to_string(),
        format!("receipt_id: {}", receipt.receipt_id),
        format!("receipt_digest: {}", receipt.receipt_digest),
        format!("proposal_id: {}", receipt.proposal_id),
        format!("proposal_digest: {}", receipt.proposal_digest),
        format!("subject: {}", receipt.subject),
        format!("role: {}", optional_text(receipt.role.as_deref())),
        format!(
            "canister_id: {}",
            optional_text(receipt.canister_id.as_deref())
        ),
        format!(
            "consent_state: {}",
            external_upgrade_consent_state_label(receipt.consent_state)
        ),
        format!(
            "verification_result: {}",
            external_upgrade_verification_result_label(receipt.verification_result)
        ),
        format!(
            "reported_by: {}",
            optional_text(receipt.reported_by.as_deref())
        ),
        format!(
            "observed_before_module_hash: {}",
            optional_text(receipt.observed_before_module_hash.as_deref())
        ),
        format!(
            "observed_after_module_hash: {}",
            optional_text(receipt.observed_after_module_hash.as_deref())
        ),
        format!("verification_notes: {}", receipt.verification_notes.len()),
    ]
    .join("\n")
}
