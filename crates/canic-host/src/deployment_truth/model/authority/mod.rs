use super::{CanisterControlClassV1, SafetyFindingV1};
use serde::{Deserialize, Serialize};

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
