use super::{
    CanisterControlClassV1, DeploymentCommandResultV1, DeploymentExecutionStatusV1,
    DeploymentObservationGapV1, RolePhaseResultV1, SafetyFindingV1, SafetyStatusV1,
};
use serde::{Deserialize, Serialize};

///
/// AuthorityReceiptV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AuthorityReceiptV1 {
    pub schema_version: u32,
    pub operation_id: String,
    pub check_id: Option<String>,
    pub reconciliation_plan_id: String,
    pub authority_report_id: String,
    pub inventory_id: String,
    pub authority_profile_hash: Option<String>,
    pub operation_status: DeploymentExecutionStatusV1,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub attempted_actions: Vec<AuthorityAttemptedActionV1>,
    pub verified_controller_observations: Vec<AuthorityControllerObservationV1>,
    pub hard_failures: Vec<SafetyFindingV1>,
    pub unresolved_observation_gaps: Vec<DeploymentObservationGapV1>,
    pub unresolved_external_actions: Vec<AuthorityExternalActionV1>,
    pub command_result: DeploymentCommandResultV1,
}

///
/// AuthorityDryRunEvidenceV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AuthorityDryRunEvidenceV1 {
    pub schema_version: u32,
    pub evidence_id: String,
    pub check_id: String,
    pub generated_at: String,
    pub reconciliation_plan: AuthorityReconciliationPlanV1,
    pub authority_report: AuthorityReportV1,
    pub authority_receipt: AuthorityReceiptV1,
}

///
/// AuthorityAttemptedActionV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AuthorityAttemptedActionV1 {
    pub subject: String,
    pub canister_id: Option<String>,
    pub role: Option<String>,
    pub action: AuthorityActionV1,
    pub result: RolePhaseResultV1,
    pub error: Option<String>,
}

///
/// AuthorityControllerObservationV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AuthorityControllerObservationV1 {
    pub subject: String,
    pub canister_id: Option<String>,
    pub role: Option<String>,
    pub state: AuthorityReconciliationStateV1,
    pub action: AuthorityActionV1,
    pub observed_controllers: Vec<String>,
    pub desired_controllers: Vec<String>,
    pub controller_delta: AuthorityControllerDeltaV1,
}

///
/// AuthorityReconciliationPlanV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AuthorityReconciliationPlanV1 {
    pub schema_version: u32,
    pub plan_id: String,
    pub inventory_id: String,
    pub authority_profile_hash: Option<String>,
    pub canister_actions: Vec<CanisterAuthorityActionV1>,
    pub automatic_actions: Vec<AuthorityAutomaticActionV1>,
    pub hard_failures: Vec<SafetyFindingV1>,
    pub external_actions_required: Vec<AuthorityExternalActionV1>,
}

///
/// AuthorityAutomaticActionV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AuthorityAutomaticActionV1 {
    pub subject: String,
    pub canister_id: String,
    pub role: Option<String>,
    pub action: AuthorityActionV1,
    pub observed_controllers: Vec<String>,
    pub desired_controllers: Vec<String>,
    pub controller_delta: AuthorityControllerDeltaV1,
    pub reason: String,
}

///
/// AuthorityControllerDeltaV1
///
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct AuthorityControllerDeltaV1 {
    pub add_controllers: Vec<String>,
    pub remove_controllers: Vec<String>,
}

///
/// AuthorityReportV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AuthorityReportV1 {
    pub schema_version: u32,
    pub report_id: String,
    pub check_id: Option<String>,
    pub reconciliation_plan_id: String,
    pub inventory_id: String,
    pub authority_profile_hash: Option<String>,
    pub status: SafetyStatusV1,
    pub summary: String,
    pub counts: AuthorityReportCountsV1,
    pub apply_readiness: AuthorityApplyReadinessV1,
    pub action_counts: Vec<AuthorityActionCountV1>,
    pub control_class_counts: Vec<AuthorityControlClassCountV1>,
    pub observation_gaps: Vec<DeploymentObservationGapV1>,
    pub automatic_actions: Vec<AuthorityAutomaticActionV1>,
    pub hard_failures: Vec<SafetyFindingV1>,
    pub external_actions_required: Vec<AuthorityExternalActionV1>,
    pub next_actions: Vec<String>,
}

///
/// AuthorityApplyReadinessV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AuthorityApplyReadinessV1 {
    pub can_apply_automatically: bool,
    pub automatic_action_count: usize,
    pub blockers: Vec<AuthorityApplyBlockerV1>,
}

///
/// AuthorityApplyBlockerV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AuthorityApplyBlockerV1 {
    UnsafeBlocked,
    HardFailures,
    ObservationGaps,
    ExternalActions,
}

///
/// AuthorityActionCountV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AuthorityActionCountV1 {
    pub action: AuthorityActionV1,
    pub count: usize,
}

///
/// AuthorityControlClassCountV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AuthorityControlClassCountV1 {
    pub control_class: CanisterControlClassV1,
    pub count: usize,
}

///
/// AuthorityReportCountsV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AuthorityReportCountsV1 {
    pub already_correct: usize,
    pub can_apply_automatically: usize,
    pub requires_external_action: usize,
    pub unsafe_blocked: usize,
    pub unknown: usize,
    pub hard_failures: usize,
}

///
/// CanisterAuthorityActionV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CanisterAuthorityActionV1 {
    pub canister_id: Option<String>,
    pub role: Option<String>,
    pub control_classification: CanisterControlClassV1,
    pub observed_controllers: Vec<String>,
    pub desired_controllers: Vec<String>,
    pub controller_delta: AuthorityControllerDeltaV1,
    pub action: AuthorityActionV1,
    pub state: AuthorityReconciliationStateV1,
    pub can_apply: bool,
    pub reason: String,
}

///
/// AuthorityExternalActionV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AuthorityExternalActionV1 {
    pub subject: String,
    pub canister_id: Option<String>,
    pub role: Option<String>,
    pub control_classification: CanisterControlClassV1,
    pub state: AuthorityReconciliationStateV1,
    pub action: AuthorityActionV1,
    pub observed_controllers: Vec<String>,
    pub desired_controllers: Vec<String>,
    pub controller_delta: AuthorityControllerDeltaV1,
    pub reason: String,
}

///
/// AuthorityActionV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AuthorityActionV1 {
    None,
    AddControllers,
    RemoveControllers,
    ReplaceControllerSet,
    RequiresExternalController,
    RequiresDestructiveImportConfirmation,
    ObserveOnly,
    AdoptPlanAvailable,
    BlockedByPolicy,
    UnknownObservation,
}

///
/// AuthorityReconciliationStateV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AuthorityReconciliationStateV1 {
    AlreadyCorrect,
    CanApplyAutomatically,
    RequiresExternalAction,
    UnsafeBlocked,
    Unknown,
}
