use super::{CanisterControlClassV1, DeploymentCheckV1};
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
/// ExternalUpgradeAuthorizationModeV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum ExternalUpgradeAuthorizationModeV1 {
    ConsentForDirectInstall,
    DelegatedInstallAuthority,
    ExternalControllerExecution,
    ObserveAndVerifyOnly,
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

///
/// ExternalUpgradeCompletionReportV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExternalUpgradeCompletionReportV1 {
    pub schema_version: u32,
    pub report_id: String,
    pub report_digest: String,
    pub proposal_id: String,
    pub proposal_digest: String,
    pub consent_evidence_id: String,
    pub consent_evidence_digest: String,
    pub verification_check_id: String,
    pub verification_check_digest: String,
    pub subject: String,
    pub canister_id: Option<String>,
    pub role: Option<String>,
    pub consent_state: ExternalUpgradeConsentStateV1,
    pub verification_result: ExternalUpgradeVerificationResultV1,
    pub verification_observation_source: ExternalVerificationObservationSourceV1,
    pub completion_status: ExternalUpgradeCompletionStatusV1,
    pub blockers: Vec<String>,
    pub next_actions: Vec<String>,
    pub status_summary: String,
}

///
/// ExternalUpgradeCompletionStatusV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum ExternalUpgradeCompletionStatusV1 {
    AwaitingConsent,
    ConsentRefused,
    SuppliedEvidenceConsistent,
    AwaitingVerification,
    VerifiedComplete,
    VerificationFailed,
}

///
/// ExternalUpgradeCompletionReportRequest
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExternalUpgradeCompletionReportRequest {
    pub report_id: String,
    pub proposal: ExternalUpgradeProposalV1,
    pub consent_evidence: ExternalUpgradeConsentEvidenceV1,
    pub verification_check: ExternalUpgradeVerificationCheckV1,
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
