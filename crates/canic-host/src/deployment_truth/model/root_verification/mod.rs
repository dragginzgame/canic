use super::{DeploymentCheckV1, DeploymentRootObservationSourceV1, SafetyFindingV1};
use serde::{Deserialize, Serialize};

///
/// DeploymentRootVerificationRequestV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeploymentRootVerificationRequestV1 {
    pub report_id: String,
    pub requested_at: String,
    pub deployment_name: String,
    pub network: String,
    pub expected_fleet_template: String,
    pub expected_root_principal: String,
    pub current_root_verification: DeploymentRootVerificationStateV1,
    pub source: DeploymentRootVerificationSourceV1,
    pub deployment_check: DeploymentCheckV1,
}

///
/// DeploymentRootVerificationReportV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeploymentRootVerificationReportV1 {
    pub schema_version: u32,
    pub report_id: String,
    pub report_digest: String,
    pub requested_at: String,
    pub evidence_status: DeploymentRootVerificationEvidenceStatusV1,
    pub state_transition: DeploymentRootVerificationStateTransitionV1,
    pub deployment_name: String,
    pub network: String,
    pub expected_fleet_template: String,
    pub expected_root_principal: String,
    pub observed_deployment_name: Option<String>,
    pub observed_network: Option<String>,
    pub observed_fleet_template: Option<String>,
    pub observed_root_principal: Option<String>,
    pub observed_root_canister_id: Option<String>,
    pub observed_root_observation_source: Option<DeploymentRootObservationSourceV1>,
    pub source: DeploymentRootVerificationSourceV1,
    pub source_check_id: String,
    pub source_check_digest: String,
    pub source_deployment_plan_id: String,
    pub source_deployment_plan_digest: String,
    pub source_inventory_id: String,
    pub source_inventory_digest: String,
    pub current_root_verification: DeploymentRootVerificationStateV1,
    pub identity_checks: Vec<DeploymentRootVerificationCheckV1>,
    pub evidence_checks: Vec<DeploymentRootVerificationCheckV1>,
    pub blockers: Vec<SafetyFindingV1>,
    pub warnings: Vec<SafetyFindingV1>,
    pub recommended_next_actions: Vec<String>,
}

///
/// DeploymentRootVerificationReceiptV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeploymentRootVerificationReceiptV1 {
    pub schema_version: u32,
    pub receipt_id: String,
    pub receipt_digest: String,
    pub deployment_name: String,
    pub network: String,
    pub fleet_template: String,
    pub root_principal: String,
    pub previous_root_verification: DeploymentRootVerificationStateV1,
    pub new_root_verification: DeploymentRootVerificationStateV1,
    pub state_transition: DeploymentRootVerificationStateTransitionV1,
    pub source_report_id: String,
    pub source_report_digest: String,
    pub source_report_requested_at: String,
    pub source_report_source: DeploymentRootVerificationSourceV1,
    pub source_report_evidence_status: DeploymentRootVerificationEvidenceStatusV1,
    pub source_report_current_root_verification: DeploymentRootVerificationStateV1,
    pub source_report_state_transition: DeploymentRootVerificationStateTransitionV1,
    pub source_root_observation_source: DeploymentRootObservationSourceV1,
    pub source_observed_root_canister_id: String,
    pub source_check_id: String,
    pub source_check_digest: String,
    pub source_deployment_plan_id: String,
    pub source_deployment_plan_digest: String,
    pub source_inventory_id: String,
    pub source_inventory_digest: String,
    pub verified_at_unix_secs: u64,
    pub local_state_path: String,
    pub local_state_digest_before: String,
    pub local_state_digest_after: String,
    pub warnings: Vec<SafetyFindingV1>,
}

///
/// DeploymentRootVerificationCheckV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeploymentRootVerificationCheckV1 {
    pub name: String,
    pub expected: Option<String>,
    pub observed: Option<String>,
    pub satisfied: bool,
}

///
/// DeploymentRootVerificationSourceV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum DeploymentRootVerificationSourceV1 {
    DeploymentTruthCheck,
}

impl DeploymentRootVerificationSourceV1 {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::DeploymentTruthCheck => "DeploymentTruthCheck",
        }
    }
}

///
/// DeploymentRootVerificationEvidenceStatusV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum DeploymentRootVerificationEvidenceStatusV1 {
    EvidenceSatisfied,
    VerificationFailed,
    NotApplicable,
}

impl DeploymentRootVerificationEvidenceStatusV1 {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::EvidenceSatisfied => "EvidenceSatisfied",
            Self::VerificationFailed => "VerificationFailed",
            Self::NotApplicable => "NotApplicable",
        }
    }
}

///
/// DeploymentRootVerificationStateTransitionV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum DeploymentRootVerificationStateTransitionV1 {
    NotAttempted,
    WouldPromoteNotVerifiedToVerified,
    PromotedNotVerifiedToVerified,
    NoStateChange,
    Blocked,
}

impl DeploymentRootVerificationStateTransitionV1 {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::NotAttempted => "NotAttempted",
            Self::WouldPromoteNotVerifiedToVerified => "WouldPromoteNotVerifiedToVerified",
            Self::PromotedNotVerifiedToVerified => "PromotedNotVerifiedToVerified",
            Self::NoStateChange => "NoStateChange",
            Self::Blocked => "Blocked",
        }
    }
}

///
/// DeploymentRootVerificationStateV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum DeploymentRootVerificationStateV1 {
    NotVerified,
    Verified,
}

impl DeploymentRootVerificationStateV1 {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::NotVerified => "NotVerified",
            Self::Verified => "Verified",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deployment_root_verification_source_owns_text_labels() {
        assert_eq!(
            DeploymentRootVerificationSourceV1::DeploymentTruthCheck.label(),
            "DeploymentTruthCheck"
        );
    }

    #[test]
    fn deployment_root_verification_evidence_status_owns_text_labels() {
        assert_eq!(
            DeploymentRootVerificationEvidenceStatusV1::EvidenceSatisfied.label(),
            "EvidenceSatisfied"
        );
        assert_eq!(
            DeploymentRootVerificationEvidenceStatusV1::VerificationFailed.label(),
            "VerificationFailed"
        );
        assert_eq!(
            DeploymentRootVerificationEvidenceStatusV1::NotApplicable.label(),
            "NotApplicable"
        );
    }

    #[test]
    fn deployment_root_verification_state_transition_owns_text_labels() {
        assert_eq!(
            DeploymentRootVerificationStateTransitionV1::NotAttempted.label(),
            "NotAttempted"
        );
        assert_eq!(
            DeploymentRootVerificationStateTransitionV1::WouldPromoteNotVerifiedToVerified.label(),
            "WouldPromoteNotVerifiedToVerified"
        );
        assert_eq!(
            DeploymentRootVerificationStateTransitionV1::PromotedNotVerifiedToVerified.label(),
            "PromotedNotVerifiedToVerified"
        );
        assert_eq!(
            DeploymentRootVerificationStateTransitionV1::NoStateChange.label(),
            "NoStateChange"
        );
        assert_eq!(
            DeploymentRootVerificationStateTransitionV1::Blocked.label(),
            "Blocked"
        );
    }

    #[test]
    fn deployment_root_verification_state_owns_text_labels() {
        assert_eq!(
            DeploymentRootVerificationStateV1::NotVerified.label(),
            "NotVerified"
        );
        assert_eq!(
            DeploymentRootVerificationStateV1::Verified.label(),
            "Verified"
        );
    }
}
