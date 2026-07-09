use super::super::CanisterControlClassV1;
use super::authority::{LifecycleAuthorityV1, LifecycleModeV1};
use super::handoff::ExternalLifecyclePendingActionV1;
use super::proposal::ExternalUpgradeProposalV1;
use serde::{Deserialize, Serialize};

///
/// ExternalLifecyclePlanV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExternalLifecyclePlanV1 {
    pub schema_version: u32,
    pub lifecycle_plan_id: String,
    pub lifecycle_plan_digest: String,
    pub lifecycle_authority_report_id: String,
    pub deployment_plan_id: String,
    pub deployment_plan_digest: String,
    pub inventory_id: String,
    pub lifecycle_authority_rows: Vec<LifecycleAuthorityV1>,
    pub directly_executable_role_upgrades: Vec<ExternalLifecycleRoleUpgradeV1>,
    pub proposed_external_role_upgrades: Vec<ExternalLifecycleRoleUpgradeV1>,
    pub blocked_role_upgrades: Vec<ExternalLifecycleRoleUpgradeV1>,
    pub dependency_blockers: Vec<String>,
    pub protected_call_implications: Vec<String>,
    pub residual_exposure: Vec<String>,
    pub status: ExternalLifecyclePlanStatusV1,
}

///
/// ExternalLifecycleRoleUpgradeV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExternalLifecycleRoleUpgradeV1 {
    pub subject: String,
    pub canister_id: Option<String>,
    pub role: Option<String>,
    pub control_class: CanisterControlClassV1,
    pub lifecycle_mode: LifecycleModeV1,
    pub required_external_action: Option<String>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
}

///
/// ExternalLifecyclePlanStatusV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum ExternalLifecyclePlanStatusV1 {
    Ready,
    PendingExternalAction,
    Blocked,
}

impl ExternalLifecyclePlanStatusV1 {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::PendingExternalAction => "pending_external_action",
            Self::Blocked => "blocked",
        }
    }
}

///
/// ExternalUpgradeProposalReportV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExternalUpgradeProposalReportV1 {
    pub schema_version: u32,
    pub report_id: String,
    pub report_digest: String,
    pub lifecycle_plan_id: String,
    pub lifecycle_plan_digest: String,
    pub deployment_plan_id: String,
    pub deployment_plan_digest: String,
    pub inventory_id: String,
    pub proposals: Vec<ExternalUpgradeProposalV1>,
    pub blocked_subjects: Vec<String>,
}

///
/// ExternalLifecyclePendingReportV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExternalLifecyclePendingReportV1 {
    pub schema_version: u32,
    pub report_id: String,
    pub report_digest: String,
    pub lifecycle_plan_id: String,
    pub lifecycle_plan_digest: String,
    pub proposal_report_id: String,
    pub proposal_report_digest: String,
    pub deployment_plan_id: String,
    pub deployment_plan_digest: String,
    pub inventory_id: String,
    pub direct_upgrade_count: usize,
    pub pending_external_count: usize,
    pub blocked_count: usize,
    pub pending_external_actions: Vec<ExternalLifecyclePendingActionV1>,
    pub blocked_subjects: Vec<String>,
    pub residual_exposure: Vec<String>,
    pub status: ExternalLifecyclePlanStatusV1,
}

///
/// ExternalLifecycleCheckV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExternalLifecycleCheckV1 {
    pub schema_version: u32,
    pub check_id: String,
    pub check_digest: String,
    pub lifecycle_plan_id: String,
    pub lifecycle_plan_digest: String,
    pub proposal_report_id: String,
    pub proposal_report_digest: String,
    pub pending_report_id: String,
    pub pending_report_digest: String,
    pub deployment_plan_id: String,
    pub deployment_plan_digest: String,
    pub inventory_id: String,
    pub status: ExternalLifecyclePlanStatusV1,
    pub direct_upgrade_count: usize,
    pub pending_external_count: usize,
    pub blocked_count: usize,
    pub residual_exposure_count: usize,
    pub summary: String,
    pub next_actions: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn external_lifecycle_plan_status_owns_text_labels() {
        assert_eq!(ExternalLifecyclePlanStatusV1::Ready.label(), "ready");
        assert_eq!(
            ExternalLifecyclePlanStatusV1::PendingExternalAction.label(),
            "pending_external_action"
        );
        assert_eq!(ExternalLifecyclePlanStatusV1::Blocked.label(), "blocked");
    }
}
