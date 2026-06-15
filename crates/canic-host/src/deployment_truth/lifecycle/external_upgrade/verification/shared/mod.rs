use super::super::super::super::*;

pub(in crate::deployment_truth::lifecycle::external_upgrade) fn external_upgrade_verification_result(
    consent_state: ExternalUpgradeConsentStateV1,
    proposal: &ExternalUpgradeProposalV1,
    observed_after_module_hash: Option<&str>,
    observed_after_config: Option<&str>,
) -> ExternalUpgradeVerificationResultV1 {
    match consent_state {
        ExternalUpgradeConsentStateV1::Pending => ExternalUpgradeVerificationResultV1::Pending,
        ExternalUpgradeConsentStateV1::Refused => ExternalUpgradeVerificationResultV1::Refused,
        ExternalUpgradeConsentStateV1::Delegated
        | ExternalUpgradeConsentStateV1::ExecutedExternally => {
            if external_upgrade_observation_matches(
                proposal.target_installed_module_hash.as_deref(),
                observed_after_module_hash,
            ) && external_upgrade_observation_matches(
                proposal.target_canonical_embedded_config_sha256.as_deref(),
                observed_after_config,
            ) {
                ExternalUpgradeVerificationResultV1::Verified
            } else {
                ExternalUpgradeVerificationResultV1::Mismatch
            }
        }
    }
}

pub(in crate::deployment_truth::lifecycle::external_upgrade) fn external_upgrade_verification_notes(
    verification_result: ExternalUpgradeVerificationResultV1,
    proposal: &ExternalUpgradeProposalV1,
    observed_after_module_hash: Option<&str>,
    observed_after_config: Option<&str>,
) -> Vec<String> {
    let mut notes = Vec::new();
    if verification_result == ExternalUpgradeVerificationResultV1::Mismatch {
        if !external_upgrade_observation_matches(
            proposal.target_installed_module_hash.as_deref(),
            observed_after_module_hash,
        ) {
            notes.push("observed module hash does not match proposal target".to_string());
        }
        if !external_upgrade_observation_matches(
            proposal.target_canonical_embedded_config_sha256.as_deref(),
            observed_after_config,
        ) {
            notes.push("observed embedded config does not match proposal target".to_string());
        }
    }
    notes
}

pub(super) const fn external_upgrade_verification_summary(
    result: ExternalUpgradeVerificationResultV1,
) -> &'static str {
    match result {
        ExternalUpgradeVerificationResultV1::Pending => {
            "external action has not been reported as complete"
        }
        ExternalUpgradeVerificationResultV1::Refused => "external consent was refused",
        ExternalUpgradeVerificationResultV1::Verified => {
            "reported external completion matches proposal target facts"
        }
        ExternalUpgradeVerificationResultV1::Mismatch => {
            "reported external completion does not match proposal target facts"
        }
    }
}

pub(super) fn control_class_value(control_class: CanisterControlClassV1) -> String {
    format!("{control_class:?}")
}

fn external_upgrade_observation_matches(expected: Option<&str>, observed: Option<&str>) -> bool {
    expected.is_none_or(|expected| observed == Some(expected))
}
