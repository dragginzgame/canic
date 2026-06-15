use super::super::CanisterControlClassV1;
use super::authority::{
    ConsentChannelKindV1, ConsentRequirementV1, ConsentSubjectKindV1, LifecycleModeV1,
    LifecycleVerificationRequirementV1,
};
use super::plan::ExternalLifecyclePlanStatusV1;
use serde::{Deserialize, Serialize};

///
/// ExternalLifecycleHandoffV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExternalLifecycleHandoffV1 {
    pub schema_version: u32,
    pub handoff_id: String,
    pub handoff_digest: String,
    pub lifecycle_check_id: String,
    pub lifecycle_check_digest: String,
    pub pending_report_id: String,
    pub pending_report_digest: String,
    pub proposal_report_id: String,
    pub proposal_report_digest: String,
    pub deployment_plan_id: String,
    pub deployment_plan_digest: String,
    pub inventory_id: String,
    pub status: ExternalLifecyclePlanStatusV1,
    pub handoff_actions: Vec<ExternalLifecycleHandoffActionV1>,
    pub blocked_subjects: Vec<String>,
    pub residual_exposure: Vec<String>,
    pub operator_summary: String,
}

///
/// ExternalLifecycleHandoffActionV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExternalLifecycleHandoffActionV1 {
    pub subject: String,
    pub proposal_id: String,
    pub proposal_digest: String,
    pub canister_id: Option<String>,
    pub role: Option<String>,
    pub control_class: CanisterControlClassV1,
    pub lifecycle_mode: LifecycleModeV1,
    pub required_external_action: String,
    pub consent_channel_kind: ConsentChannelKindV1,
    pub consent_subject_kind: ConsentSubjectKindV1,
    pub required_principals: Vec<String>,
    pub current_module_hash: Option<String>,
    pub target_installed_module_hash: Option<String>,
    pub target_canonical_embedded_config_sha256: Option<String>,
    pub verification_requirements: Vec<LifecycleVerificationRequirementV1>,
    pub operator_instructions: Vec<String>,
}

///
/// ExternalLifecyclePendingActionV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExternalLifecyclePendingActionV1 {
    pub subject: String,
    pub proposal_id: String,
    pub proposal_digest: String,
    pub canister_id: Option<String>,
    pub role: Option<String>,
    pub control_class: CanisterControlClassV1,
    pub lifecycle_mode: LifecycleModeV1,
    pub required_external_action: String,
    pub consent_requirements: Vec<ConsentRequirementV1>,
    pub verification_requirements: Vec<LifecycleVerificationRequirementV1>,
}

///
/// CriticalExternalFixReportV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CriticalExternalFixReportV1 {
    pub schema_version: u32,
    pub report_id: String,
    pub report_digest: String,
    pub fix_id: String,
    pub severity: String,
    pub lifecycle_plan_id: String,
    pub lifecycle_plan_digest: String,
    pub pending_report_id: String,
    pub pending_report_digest: String,
    pub deployment_plan_id: String,
    pub deployment_plan_digest: String,
    pub inventory_id: String,
    pub affected_roles: Vec<String>,
    pub affected_canisters: Vec<String>,
    pub directly_patchable_roles: Vec<String>,
    pub externally_blocked_roles: Vec<String>,
    pub dependency_blocked_roles: Vec<String>,
    pub required_external_actions: Vec<String>,
    pub protected_call_implications: Vec<String>,
    pub residual_exposure: Vec<String>,
    pub operator_next_steps: Vec<String>,
}
