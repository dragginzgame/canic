use super::super::CanisterControlClassV1;
use serde::{Deserialize, Serialize};

///
/// LifecycleAuthorityReportV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LifecycleAuthorityReportV1 {
    pub schema_version: u32,
    pub report_id: String,
    pub report_digest: String,
    pub check_id: String,
    pub plan_id: String,
    pub inventory_id: String,
    pub authorities: Vec<LifecycleAuthorityV1>,
    pub external_action_required_count: usize,
    pub blocked_count: usize,
}

///
/// LifecycleAuthorityV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LifecycleAuthorityV1 {
    pub subject: String,
    pub canister_id: Option<String>,
    pub role: Option<String>,
    pub control_class: CanisterControlClassV1,
    pub lifecycle_mode: LifecycleModeV1,
    pub observed_controllers: Vec<String>,
    pub expected_deployment_controllers: Vec<String>,
    pub external_controllers: Vec<String>,
    pub required_controllers: Vec<String>,
    pub consent_requirements: Vec<ConsentRequirementV1>,
    pub allowed_upgrade_modes: Vec<LifecycleUpgradeModeV1>,
    pub verification_requirements: Vec<LifecycleVerificationRequirementV1>,
    pub external_action_required: bool,
    pub blocked: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub reason: String,
}

///
/// LifecycleModeV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum LifecycleModeV1 {
    DirectDeploymentAuthority,
    ProposalRequired,
    DelegatedInstallRequired,
    ExternalCompletionOnly,
    VerifyOnly,
    MustNotTouch,
    UnknownUnsafeBlocked,
}

///
/// LifecycleUpgradeModeV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum LifecycleUpgradeModeV1 {
    DirectByDeploymentAuthority,
    ExternalProposal,
    ExternalExecution,
    VerifyExternalCompletion,
    ObserveOnly,
    Blocked,
}

///
/// LifecycleVerificationRequirementV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum LifecycleVerificationRequirementV1 {
    LiveInventory,
    ControllerObservation,
    ModuleHash,
    CanonicalEmbeddedConfig,
    ProtectedCallReadiness,
}

///
/// ConsentRequirementV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConsentRequirementV1 {
    pub consent_subject_kind: ConsentSubjectKindV1,
    pub required_principals: Vec<String>,
    pub required_controller_set_digest: Option<String>,
    pub consent_channel_kind: ConsentChannelKindV1,
    pub required_action: ExternalUpgradeAuthorizationModeV1,
}

///
/// ConsentSubjectKindV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum ConsentSubjectKindV1 {
    UserPrincipal,
    ProjectHub,
    GovernanceCanister,
    CustomerController,
    DelegatedInstallCanister,
    MultisigAuthority,
    UnknownExternalController,
}

///
/// ConsentChannelKindV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum ConsentChannelKindV1 {
    OutOfBand,
    GeneratedCommand,
    DelegatedInstall,
    GovernanceProposal,
    ApplicationSpecific,
}

///
/// ExternalUpgradeAuthorizationModeV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum ExternalUpgradeAuthorizationModeV1 {
    ConsentForDirectInstall,
    DelegatedInstallAuthority,
    ExternalControllerExecution,
    ObserveAndVerifyOnly,
}
