use super::super::{CanisterControlClassV1, DeploymentCheckV1};
use super::authority::LifecycleVerificationRequirementV1;
use super::proposal::{
    ExternalUpgradeProposalV1, ExternalUpgradeReceiptV1, ExternalUpgradeVerificationResultV1,
};
use serde::{Deserialize, Serialize};

///
/// ExternalUpgradeVerificationReportV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExternalUpgradeVerificationReportV1 {
    pub schema_version: u32,
    pub report_id: String,
    pub report_digest: String,
    pub proposal_id: String,
    pub proposal_digest: String,
    pub receipt_id: String,
    pub receipt_digest: String,
    pub subject: String,
    pub canister_id: Option<String>,
    pub role: Option<String>,
    pub verification_result: ExternalUpgradeVerificationResultV1,
    pub verification_notes: Vec<String>,
    pub live_inventory_required: bool,
    pub status_summary: String,
}

///
/// ExternalUpgradeVerificationReportRequest
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExternalUpgradeVerificationReportRequest {
    pub report_id: String,
    pub proposal: ExternalUpgradeProposalV1,
    pub receipt: ExternalUpgradeReceiptV1,
}

///
/// ExternalUpgradeVerificationPolicyV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExternalUpgradeVerificationPolicyV1 {
    pub schema_version: u32,
    pub policy_id: String,
    pub policy_digest: String,
    pub proposal_id: String,
    pub proposal_digest: String,
    pub deployment_plan_id: String,
    pub deployment_plan_digest: String,
    pub subject: String,
    pub canister_id: Option<String>,
    pub role: Option<String>,
    pub required_verification: Vec<LifecycleVerificationRequirementV1>,
    pub verification_requirements: Vec<ExternalUpgradeVerificationPolicyRequirementV1>,
    pub max_observation_age_seconds: Option<u64>,
    pub status_summary: String,
}

///
/// ExternalUpgradeVerificationPolicyRequirementV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExternalUpgradeVerificationPolicyRequirementV1 {
    pub requirement: LifecycleVerificationRequirementV1,
    pub status: ExternalUpgradeVerificationRequirementStatusV1,
    pub expected_value: Option<String>,
}

///
/// ExternalUpgradeVerificationRequirementStatusV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum ExternalUpgradeVerificationRequirementStatusV1 {
    Required,
    NotRequired,
}

impl ExternalUpgradeVerificationRequirementStatusV1 {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Required => "required",
            Self::NotRequired => "not_required",
        }
    }
}

///
/// ExternalUpgradeVerificationPolicyRequest
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExternalUpgradeVerificationPolicyRequest {
    pub policy_id: String,
    pub proposal: ExternalUpgradeProposalV1,
}

///
/// ExternalUpgradeVerificationObservationV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExternalUpgradeVerificationObservationV1 {
    pub source: ExternalVerificationObservationSourceV1,
    pub deployment_check_id: Option<String>,
    pub deployment_check_digest: Option<String>,
    pub inventory_id: Option<String>,
    pub observed_at: Option<String>,
    pub live_inventory_observed: bool,
    pub controller_observation_present: bool,
    pub observed_control_class: Option<CanisterControlClassV1>,
    pub observed_module_hash: Option<String>,
    pub observed_canonical_embedded_config_sha256: Option<String>,
    pub protected_call_ready: Option<bool>,
}

///
/// ExternalVerificationObservationSourceV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum ExternalVerificationObservationSourceV1 {
    SuppliedObservation,
    DeploymentTruthInventory,
}

impl ExternalVerificationObservationSourceV1 {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::SuppliedObservation => "supplied_observation",
            Self::DeploymentTruthInventory => "deployment_truth_inventory",
        }
    }
}

///
/// ExternalUpgradeVerificationCheckV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExternalUpgradeVerificationCheckV1 {
    pub schema_version: u32,
    pub check_id: String,
    pub check_digest: String,
    pub policy_id: String,
    pub policy_digest: String,
    pub proposal_id: String,
    pub proposal_digest: String,
    pub subject: String,
    pub canister_id: Option<String>,
    pub role: Option<String>,
    pub observation: ExternalUpgradeVerificationObservationV1,
    pub requirement_results: Vec<ExternalUpgradeVerificationCheckRequirementV1>,
    pub verification_result: ExternalUpgradeVerificationResultV1,
    pub status_summary: String,
}

///
/// ExternalUpgradeVerificationCheckRequirementV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExternalUpgradeVerificationCheckRequirementV1 {
    pub requirement: LifecycleVerificationRequirementV1,
    pub status: ExternalUpgradeVerificationRequirementStatusV1,
    pub expected_value: Option<String>,
    pub observed_value: Option<String>,
    pub satisfied: Option<bool>,
}

///
/// ExternalUpgradeVerificationCheckRequest
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExternalUpgradeVerificationCheckRequest {
    pub check_id: String,
    pub policy: ExternalUpgradeVerificationPolicyV1,
    pub observation: Option<ExternalUpgradeVerificationObservationV1>,
    pub deployment_check: Option<DeploymentCheckV1>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn external_upgrade_verification_requirement_status_owns_text_labels() {
        assert_eq!(
            ExternalUpgradeVerificationRequirementStatusV1::Required.label(),
            "required"
        );
        assert_eq!(
            ExternalUpgradeVerificationRequirementStatusV1::NotRequired.label(),
            "not_required"
        );
    }

    #[test]
    fn external_verification_observation_source_owns_text_labels() {
        assert_eq!(
            ExternalVerificationObservationSourceV1::SuppliedObservation.label(),
            "supplied_observation"
        );
        assert_eq!(
            ExternalVerificationObservationSourceV1::DeploymentTruthInventory.label(),
            "deployment_truth_inventory"
        );
    }
}
