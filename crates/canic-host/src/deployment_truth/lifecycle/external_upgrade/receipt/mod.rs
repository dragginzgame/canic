use super::super::super::*;
use super::super::digest::*;
use super::super::error::ExternalUpgradeReceiptError;
use super::validation::{
    ensure_external_receipt_field, ensure_external_receipt_matches_proposal,
    ensure_external_receipt_option_matches_proposal,
};
use super::verification::{
    external_upgrade_verification_notes, external_upgrade_verification_result,
};

/// Build a passive external-upgrade receipt from post-action observation.
///
/// The receipt records what an external controller claims or completed. It does
/// not verify live state by itself and does not grant deployment authority.
#[must_use]
pub fn external_upgrade_receipt_from_observation(
    receipt_id: impl Into<String>,
    proposal: &ExternalUpgradeProposalV1,
    consent_state: ExternalUpgradeConsentStateV1,
    reported_by: Option<String>,
    observed_after: Option<&ObservedCanisterV1>,
) -> ExternalUpgradeReceiptV1 {
    let observed_after_module_hash =
        observed_after.and_then(|observed| observed.module_hash.clone());
    let observed_after_canonical_embedded_config_sha256 =
        observed_after.and_then(|observed| observed.canonical_embedded_config_digest.clone());
    let verification_result = external_upgrade_verification_result(
        consent_state,
        proposal,
        observed_after_module_hash.as_deref(),
        observed_after_canonical_embedded_config_sha256.as_deref(),
    );
    let verification_notes = external_upgrade_verification_notes(
        verification_result,
        proposal,
        observed_after_module_hash.as_deref(),
        observed_after_canonical_embedded_config_sha256.as_deref(),
    );

    let mut receipt = ExternalUpgradeReceiptV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        receipt_id: receipt_id.into(),
        proposal_id: proposal.proposal_id.clone(),
        proposal_digest: proposal.proposal_digest.clone(),
        subject: proposal.subject.clone(),
        canister_id: proposal.canister_id.clone(),
        role: proposal.role.clone(),
        consent_state,
        reported_by,
        observed_before_module_hash: proposal.current_module_hash.clone(),
        observed_after_module_hash,
        observed_after_canonical_embedded_config_sha256,
        verification_result,
        verification_notes,
        receipt_digest: String::new(),
    };
    receipt.receipt_digest = external_upgrade_receipt_digest(&receipt);
    receipt
}

/// Validate the internal consistency of an external-upgrade receipt.
///
/// This is structural validation only. Live inventory remains the source of
/// truth for whether the external upgrade actually completed.
pub fn validate_external_upgrade_receipt(
    receipt: &ExternalUpgradeReceiptV1,
) -> Result<(), ExternalUpgradeReceiptError> {
    if receipt.schema_version != DEPLOYMENT_TRUTH_SCHEMA_VERSION {
        return Err(ExternalUpgradeReceiptError::SchemaVersionMismatch {
            expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            actual: receipt.schema_version,
        });
    }
    ensure_external_receipt_field("receipt_id", receipt.receipt_id.as_str())?;
    ensure_external_receipt_field("proposal_id", receipt.proposal_id.as_str())?;
    ensure_external_receipt_field("proposal_digest", receipt.proposal_digest.as_str())?;
    ensure_external_receipt_field("subject", receipt.subject.as_str())?;
    ensure_external_receipt_field("receipt_digest", receipt.receipt_digest.as_str())?;

    if receipt.consent_state == ExternalUpgradeConsentStateV1::Refused
        && receipt.verification_result == ExternalUpgradeVerificationResultV1::Verified
    {
        return Err(ExternalUpgradeReceiptError::RefusedConsentVerified);
    }
    let has_observation = receipt.observed_after_module_hash.is_some()
        || receipt
            .observed_after_canonical_embedded_config_sha256
            .is_some();
    if matches!(
        receipt.verification_result,
        ExternalUpgradeVerificationResultV1::Verified
            | ExternalUpgradeVerificationResultV1::Mismatch
    ) && !has_observation
    {
        return Err(ExternalUpgradeReceiptError::VerificationMismatch);
    }
    if receipt.receipt_digest != external_upgrade_receipt_digest(receipt) {
        return Err(ExternalUpgradeReceiptError::DigestMismatch {
            field: "receipt_digest",
        });
    }
    Ok(())
}

/// Validate an external-upgrade receipt against the proposal it claims to
/// satisfy.
///
/// This remains structural verification. It proves the receipt is linked to the
/// supplied proposal and that its verification result matches the proposal's
/// target facts, but live inventory remains the source of deployment truth.
pub fn validate_external_upgrade_receipt_for_proposal(
    receipt: &ExternalUpgradeReceiptV1,
    proposal: &ExternalUpgradeProposalV1,
) -> Result<(), ExternalUpgradeReceiptError> {
    validate_external_upgrade_receipt(receipt)?;
    ensure_external_receipt_matches_proposal(
        "proposal_id",
        receipt.proposal_id.as_str(),
        proposal.proposal_id.as_str(),
    )?;
    ensure_external_receipt_matches_proposal(
        "proposal_digest",
        receipt.proposal_digest.as_str(),
        proposal.proposal_digest.as_str(),
    )?;
    ensure_external_receipt_matches_proposal(
        "subject",
        receipt.subject.as_str(),
        proposal.subject.as_str(),
    )?;
    ensure_external_receipt_option_matches_proposal(
        "canister_id",
        receipt.canister_id.as_deref(),
        proposal.canister_id.as_deref(),
    )?;
    ensure_external_receipt_option_matches_proposal(
        "role",
        receipt.role.as_deref(),
        proposal.role.as_deref(),
    )?;
    ensure_external_receipt_option_matches_proposal(
        "observed_before_module_hash",
        receipt.observed_before_module_hash.as_deref(),
        proposal.current_module_hash.as_deref(),
    )?;

    let expected_result = external_upgrade_verification_result(
        receipt.consent_state,
        proposal,
        receipt.observed_after_module_hash.as_deref(),
        receipt
            .observed_after_canonical_embedded_config_sha256
            .as_deref(),
    );
    if receipt.verification_result != expected_result {
        return Err(ExternalUpgradeReceiptError::VerificationMismatch);
    }
    let expected_notes = external_upgrade_verification_notes(
        expected_result,
        proposal,
        receipt.observed_after_module_hash.as_deref(),
        receipt
            .observed_after_canonical_embedded_config_sha256
            .as_deref(),
    );
    if receipt.verification_notes != expected_notes {
        return Err(ExternalUpgradeReceiptError::SourceMismatch {
            field: "verification_notes",
        });
    }

    Ok(())
}
