use super::super::CanisterControlClassV1;
use super::authority::{
    ConsentRequirementV1, ExternalUpgradeAuthorizationModeV1, LifecycleModeV1,
    LifecycleVerificationRequirementV1,
};
use serde::{Deserialize, Serialize};

///
/// ExternalUpgradeProposalV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExternalUpgradeProposalV1 {
    pub proposal_id: String,
    pub proposal_digest: String,
    pub deployment_plan_id: String,
    pub deployment_plan_digest: String,
    pub lifecycle_plan_id: String,
    pub lifecycle_plan_digest: String,
    pub promotion_plan_id: Option<String>,
    pub promotion_plan_digest: Option<String>,
    pub promotion_provenance_id: Option<String>,
    pub promotion_provenance_digest: Option<String>,
    pub subject: String,
    pub canister_id: Option<String>,
    pub role: Option<String>,
    pub control_class: CanisterControlClassV1,
    pub lifecycle_mode: LifecycleModeV1,
    pub observed_before_digest: String,
    pub current_module_hash: Option<String>,
    pub current_canonical_embedded_config_sha256: Option<String>,
    pub target_wasm_sha256: Option<String>,
    pub target_wasm_gz_sha256: Option<String>,
    pub target_installed_module_hash: Option<String>,
    pub target_role_artifact_identity: Option<String>,
    pub target_canonical_embedded_config_sha256: Option<String>,
    pub root_trust_anchor: Option<String>,
    pub authority_profile_hash: Option<String>,
    pub required_external_action: String,
    pub consent_requirements: Vec<ConsentRequirementV1>,
    pub allowed_authorization_modes: Vec<ExternalUpgradeAuthorizationModeV1>,
    pub verification_requirements: Vec<LifecycleVerificationRequirementV1>,
    pub expires_at: Option<String>,
    pub supersedes_proposal_id: Option<String>,
}

///
/// ExternalUpgradeReceiptV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExternalUpgradeReceiptV1 {
    pub schema_version: u32,
    pub receipt_id: String,
    pub proposal_id: String,
    pub proposal_digest: String,
    pub subject: String,
    pub canister_id: Option<String>,
    pub role: Option<String>,
    pub consent_state: ExternalUpgradeConsentStateV1,
    pub reported_by: Option<String>,
    pub observed_before_module_hash: Option<String>,
    pub observed_after_module_hash: Option<String>,
    pub observed_after_canonical_embedded_config_sha256: Option<String>,
    pub verification_result: ExternalUpgradeVerificationResultV1,
    pub verification_notes: Vec<String>,
    pub receipt_digest: String,
}

///
/// ExternalUpgradeConsentEvidenceV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExternalUpgradeConsentEvidenceV1 {
    pub schema_version: u32,
    pub evidence_id: String,
    pub evidence_digest: String,
    pub proposal_id: String,
    pub proposal_digest: String,
    pub receipt_id: String,
    pub receipt_digest: String,
    pub subject: String,
    pub canister_id: Option<String>,
    pub role: Option<String>,
    pub consent_state: ExternalUpgradeConsentStateV1,
    pub reported_by: Option<String>,
    pub consent_requirements: Vec<ConsentRequirementV1>,
    pub allowed_authorization_modes: Vec<ExternalUpgradeAuthorizationModeV1>,
    pub status_summary: String,
}

///
/// ExternalUpgradeConsentEvidenceRequest
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExternalUpgradeConsentEvidenceRequest {
    pub evidence_id: String,
    pub proposal: ExternalUpgradeProposalV1,
    pub receipt: ExternalUpgradeReceiptV1,
}

///
/// ExternalUpgradeConsentStateV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum ExternalUpgradeConsentStateV1 {
    Pending,
    Refused,
    Delegated,
    ExecutedExternally,
}

impl ExternalUpgradeConsentStateV1 {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Refused => "refused",
            Self::Delegated => "delegated",
            Self::ExecutedExternally => "executed_externally",
        }
    }
}

///
/// ExternalUpgradeVerificationResultV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum ExternalUpgradeVerificationResultV1 {
    Pending,
    Refused,
    Verified,
    Mismatch,
}

impl ExternalUpgradeVerificationResultV1 {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Refused => "refused",
            Self::Verified => "verified",
            Self::Mismatch => "mismatch",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn external_upgrade_consent_state_owns_text_labels() {
        assert_eq!(ExternalUpgradeConsentStateV1::Pending.label(), "pending");
        assert_eq!(ExternalUpgradeConsentStateV1::Refused.label(), "refused");
        assert_eq!(
            ExternalUpgradeConsentStateV1::Delegated.label(),
            "delegated"
        );
        assert_eq!(
            ExternalUpgradeConsentStateV1::ExecutedExternally.label(),
            "executed_externally"
        );
    }

    #[test]
    fn external_upgrade_verification_result_owns_text_labels() {
        assert_eq!(
            ExternalUpgradeVerificationResultV1::Pending.label(),
            "pending"
        );
        assert_eq!(
            ExternalUpgradeVerificationResultV1::Refused.label(),
            "refused"
        );
        assert_eq!(
            ExternalUpgradeVerificationResultV1::Verified.label(),
            "verified"
        );
        assert_eq!(
            ExternalUpgradeVerificationResultV1::Mismatch.label(),
            "mismatch"
        );
    }
}
