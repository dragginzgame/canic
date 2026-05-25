use serde::{Deserialize, Serialize};

///
/// DeploymentPlanV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeploymentPlanV1 {
    pub schema_version: u32,
    pub plan_id: String,
    pub deployment_identity: DeploymentIdentityV1,
    pub trust_domain: TrustDomainV1,
    pub fleet_template: String,
    pub runtime_variant: String,
    pub authority_profile: AuthorityProfileV1,
    pub role_artifacts: Vec<RoleArtifactV1>,
    pub expected_canisters: Vec<ExpectedCanisterV1>,
    pub expected_pool: Vec<ExpectedPoolCanisterV1>,
    pub expected_verifier_readiness: VerifierReadinessExpectationV1,
    pub unresolved_assumptions: Vec<DeploymentAssumptionV1>,
}

///
/// DeploymentInventoryV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeploymentInventoryV1 {
    pub schema_version: u32,
    pub inventory_id: String,
    pub observed_at: String,
    pub observed_identity: Option<DeploymentIdentityV1>,
    pub local_config: LocalDeploymentConfigV1,
    pub observed_canisters: Vec<ObservedCanisterV1>,
    pub observed_pool: Vec<ObservedPoolCanisterV1>,
    pub observed_artifacts: Vec<ObservedArtifactV1>,
    pub observed_verifier_readiness: VerifierReadinessObservationV1,
    pub unresolved_observations: Vec<DeploymentObservationGapV1>,
}

///
/// DeploymentReceiptV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeploymentReceiptV1 {
    pub schema_version: u32,
    pub operation_id: String,
    pub plan_id: String,
    pub execution_context: Option<DeploymentExecutionContextV1>,
    pub operation_status: DeploymentExecutionStatusV1,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub operator_principal: Option<String>,
    pub root_principal: Option<String>,
    pub previous_observed_deployment_epoch: Option<u64>,
    pub phase_receipts: Vec<PhaseReceiptV1>,
    pub role_phase_receipts: Vec<RolePhaseReceiptV1>,
    pub final_inventory_id: Option<String>,
    pub command_result: DeploymentCommandResultV1,
}

///
/// DeploymentExecutionContextV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeploymentExecutionContextV1 {
    pub workspace_root: Option<String>,
    pub icp_root: Option<String>,
    pub artifact_roots: Vec<String>,
    pub backend: DeploymentExecutorBackendV1,
    pub backend_capabilities: Vec<DeploymentExecutorCapabilityV1>,
}

///
/// DeploymentExecutionPreflightV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeploymentExecutionPreflightV1 {
    pub schema_version: u32,
    pub plan_id: String,
    pub safety_report_id: String,
    pub authority_plan_id: String,
    pub backend: DeploymentExecutorBackendV1,
    pub status: DeploymentExecutionPreflightStatusV1,
    pub planned_phases: Vec<String>,
    pub required_capabilities: Vec<DeploymentExecutorCapabilityV1>,
    pub missing_capabilities: Vec<DeploymentExecutorCapabilityV1>,
    pub blockers: Vec<SafetyFindingV1>,
}

///
/// DeploymentExecutionPreflightStatusV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum DeploymentExecutionPreflightStatusV1 {
    Ready,
    Blocked,
}

///
/// DeploymentExecutorBackendV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum DeploymentExecutorBackendV1 {
    CurrentCli,
    PocketIc,
    DirectAgent,
    Other { name: String },
}

///
/// DeploymentExecutorCapabilityV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum DeploymentExecutorCapabilityV1 {
    CreateCanister,
    CanisterStatus,
    UpdateSettings,
    InstallCode,
    Call,
    Query,
    StageArtifact,
}

///
/// ArtifactTransportV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ArtifactTransportV1 {
    LocalCli,
    WasmStore,
    DirectAgent,
}

///
/// StagingReceiptV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StagingReceiptV1 {
    pub schema_version: u32,
    pub role: String,
    pub artifact_identity: String,
    pub transport: ArtifactTransportV1,
    pub wasm_store_locator: Option<String>,
    pub prepared_chunk_hashes: Vec<String>,
    pub published_chunk_count: usize,
    pub verified_postcondition: VerifiedPostconditionV1,
}

///
/// RoleArtifactSourceV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RoleArtifactSourceV1 {
    pub role: String,
    pub kind: RoleArtifactSourceKindV1,
    pub locator: Option<String>,
    pub previous_receipt_kind: Option<PreviousArtifactReceiptKindV1>,
    pub previous_receipt_lineage_digest: Option<String>,
    pub expected_wasm_sha256: Option<String>,
    pub expected_wasm_gz_sha256: Option<String>,
    pub expected_candid_sha256: Option<String>,
    pub expected_canonical_embedded_config_sha256: Option<String>,
}

///
/// RolePromotionInputV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RolePromotionInputV1 {
    pub role: String,
    pub promotion_level: PromotionArtifactLevelV1,
    pub source: RoleArtifactSourceV1,
    pub require_byte_identical_wasm: bool,
    pub require_target_embedded_config: bool,
    pub target_store_has_artifact: Option<bool>,
}

///
/// RolePromotionPolicyV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RolePromotionPolicyV1 {
    pub role: String,
    pub allowed_promotion_levels: Vec<PromotionArtifactLevelV1>,
    pub requirements: Vec<PromotionPolicyRequirementV1>,
}

///
/// PromotionPolicyRequirementV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum PromotionPolicyRequirementV1 {
    SameSourceRevision,
    SameCargoFeatures,
    TargetConfigDigest,
    ByteIdenticalWasm,
    SealedBytes,
}

///
/// PromotionPolicyClaimV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum PromotionPolicyClaimV1 {
    ByteIdenticalWasm,
    TargetConfigDigest,
}

///
/// PromotionPolicyCheckV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PromotionPolicyCheckV1 {
    pub schema_version: u32,
    pub check_id: String,
    pub status: PromotionReadinessStatusV1,
    pub roles: Vec<RolePromotionPolicyDecisionV1>,
    pub blockers: Vec<SafetyFindingV1>,
}

///
/// RolePromotionPolicyDecisionV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RolePromotionPolicyDecisionV1 {
    pub role: String,
    pub requested_promotion_level: PromotionArtifactLevelV1,
    pub allowed_promotion_levels: Vec<PromotionArtifactLevelV1>,
    pub requirements: Vec<PromotionPolicyRequirementV1>,
    pub claims: Vec<PromotionPolicyClaimV1>,
    pub level_allowed: bool,
    pub policy_satisfied: bool,
}

///
/// PromotionArtifactLevelV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum PromotionArtifactLevelV1 {
    SealedWasm,
    SourceBuild,
}

///
/// BuildRecipeIdentityV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BuildRecipeIdentityV1 {
    pub recipe_id: String,
    pub source_kind: RoleArtifactSourceKindV1,
    pub source_revision: String,
    pub source_tree_clean: bool,
    pub package_or_role_selector: String,
    pub cargo_profile: String,
    pub cargo_features_digest: String,
    pub cargo_lock_digest: String,
    pub rust_toolchain: String,
    pub builder_version: String,
    pub target_triple: String,
    pub linker_identity: String,
    pub deterministic_build_mode: String,
    pub wasm_opt_version: String,
    pub compression_identity: String,
}

///
/// BuildMaterializationInputV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BuildMaterializationInputV1 {
    pub materialization_input_id: String,
    pub build_recipe_id: String,
    pub canonical_embedded_config_sha256: String,
    pub network: String,
    pub root_trust_anchor: String,
    pub runtime_variant: String,
}

///
/// BuildMaterializationResultV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BuildMaterializationResultV1 {
    pub materialization_result_id: String,
    pub build_recipe_id: String,
    pub materialization_input_digest: String,
    pub wasm_sha256: String,
    pub wasm_gz_sha256: String,
    pub installed_module_hash: String,
    pub candid_sha256: String,
}

///
/// BuildMaterializationEvidenceV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BuildMaterializationEvidenceV1 {
    pub schema_version: u32,
    pub evidence_id: String,
    pub recipe: BuildRecipeIdentityV1,
    pub materialization_input: BuildMaterializationInputV1,
    pub materialization_result: BuildMaterializationResultV1,
    pub computed_materialization_input_digest: String,
    pub recipe_id_matches_input: bool,
    pub recipe_id_matches_result: bool,
    pub materialization_input_digest_matches_result: bool,
}

///
/// PromotionArtifactIdentityReportV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PromotionArtifactIdentityReportV1 {
    pub schema_version: u32,
    pub report_id: String,
    pub status: PromotionReadinessStatusV1,
    pub roles: Vec<RolePromotionArtifactIdentityV1>,
    pub identity_groups: Vec<PromotionArtifactIdentityGroupV1>,
    pub blockers: Vec<SafetyFindingV1>,
}

///
/// PromotionArtifactIdentityGroupV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PromotionArtifactIdentityGroupV1 {
    pub identity_key: String,
    pub identity_kind: PromotionArtifactIdentityKindV1,
    pub roles: Vec<String>,
    pub source_kinds: Vec<RoleArtifactSourceKindV1>,
    pub source_locators: Vec<String>,
    pub digest_pinned: bool,
    pub wasm_sha256: Option<String>,
    pub wasm_gz_sha256: Option<String>,
    pub candid_sha256: Option<String>,
    pub canonical_embedded_config_sha256: Option<String>,
}

///
/// RolePromotionArtifactIdentityV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RolePromotionArtifactIdentityV1 {
    pub role: String,
    pub promotion_level: PromotionArtifactLevelV1,
    pub source_kind: RoleArtifactSourceKindV1,
    pub source_locator: Option<String>,
    pub identity_kind: PromotionArtifactIdentityKindV1,
    pub digest_pinned: bool,
    pub wasm_sha256: Option<String>,
    pub wasm_gz_sha256: Option<String>,
    pub candid_sha256: Option<String>,
    pub canonical_embedded_config_sha256: Option<String>,
}

///
/// PromotionArtifactIdentityKindV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PromotionArtifactIdentityKindV1 {
    SealedWasm,
    SealedCompressedWasm,
    SealedWasmAndCompressedWasm,
    SourceBuild,
    Deferred,
}

///
/// PromotionReadinessV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PromotionReadinessV1 {
    pub schema_version: u32,
    pub readiness_id: String,
    pub target_plan_id: String,
    pub status: PromotionReadinessStatusV1,
    pub roles: Vec<RolePromotionReadinessV1>,
    pub blockers: Vec<SafetyFindingV1>,
    pub warnings: Vec<SafetyFindingV1>,
}

///
/// PromotionPlanTransformV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PromotionPlanTransformV1 {
    pub schema_version: u32,
    pub transform_id: String,
    pub target_plan_id: String,
    pub promoted_plan_id: String,
    pub promotion_plan_lineage_digest: String,
    pub promoted_plan: DeploymentPlanV1,
    pub roles: Vec<RolePromotionPlanTransformV1>,
}

///
/// ArtifactPromotionPlanV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ArtifactPromotionPlanV1 {
    pub schema_version: u32,
    pub plan_id: String,
    pub generated_at: String,
    pub status: PromotionReadinessStatusV1,
    pub target_plan_id: String,
    pub promoted_plan_id: String,
    pub promotion_plan_lineage_digest: String,
    pub readiness: PromotionReadinessV1,
    pub artifact_identity_report: PromotionArtifactIdentityReportV1,
    pub transform: PromotionPlanTransformV1,
    pub target_execution_lineage: Option<PromotionTargetExecutionLineageV1>,
    pub blockers: Vec<SafetyFindingV1>,
}

///
/// PromotionPlanTransformEvidenceV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PromotionPlanTransformEvidenceV1 {
    pub schema_version: u32,
    pub evidence_id: String,
    pub generated_at: String,
    pub transform: PromotionPlanTransformV1,
}

///
/// PromotionTargetExecutionLineageV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PromotionTargetExecutionLineageV1 {
    pub schema_version: u32,
    pub lineage_id: String,
    pub generated_at: String,
    pub target_execution_lineage_digest: String,
    pub transform: PromotionPlanTransformV1,
    pub execution_preflight: DeploymentExecutionPreflightV1,
    pub execution_attempted: bool,
}

///
/// RolePromotionPlanTransformV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RolePromotionPlanTransformV1 {
    pub role: String,
    pub promotion_level: PromotionArtifactLevelV1,
    pub source_kind: RoleArtifactSourceKindV1,
    pub source_locator: Option<String>,
    pub artifact_source_before: ArtifactSourceV1,
    pub artifact_source_after: ArtifactSourceV1,
    pub wasm_sha256_before: Option<String>,
    pub wasm_sha256_after: Option<String>,
    pub wasm_gz_sha256_before: Option<String>,
    pub wasm_gz_sha256_after: Option<String>,
    pub candid_sha256_before: Option<String>,
    pub candid_sha256_after: Option<String>,
    pub canonical_embedded_config_sha256_before: Option<String>,
    pub canonical_embedded_config_sha256_after: Option<String>,
    pub artifact_identity_changed: bool,
    pub embedded_config_changed: bool,
    pub target_materialization_preserved: bool,
    pub source_build_materialization: Option<RolePromotionMaterializationLinkV1>,
}

///
/// RolePromotionMaterializationLinkV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RolePromotionMaterializationLinkV1 {
    pub role: String,
    pub evidence_id: String,
    pub recipe_id: String,
    pub materialization_input_id: String,
    pub materialization_result_id: String,
    pub materialization_input_digest: String,
    pub wasm_sha256: String,
    pub wasm_gz_sha256: String,
    pub installed_module_hash: String,
    pub candid_sha256: String,
}

///
/// PromotionReadinessStatusV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PromotionReadinessStatusV1 {
    Ready,
    Blocked,
}

///
/// RolePromotionReadinessV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RolePromotionReadinessV1 {
    pub role: String,
    pub promotion_level: PromotionArtifactLevelV1,
    pub source_kind: RoleArtifactSourceKindV1,
    pub source_locator: Option<String>,
    pub source_wasm_sha256: Option<String>,
    pub source_wasm_gz_sha256: Option<String>,
    pub target_wasm_sha256: Option<String>,
    pub target_wasm_gz_sha256: Option<String>,
    pub source_canonical_embedded_config_sha256: Option<String>,
    pub target_canonical_embedded_config_sha256: Option<String>,
    pub byte_identical_wasm: Option<bool>,
    pub embedded_config_identical: Option<bool>,
    pub target_store_has_artifact: Option<bool>,
    pub restage_required: bool,
}

///
/// RoleArtifactSourceKindV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RoleArtifactSourceKindV1 {
    WorkspacePackage,
    PublishedPackage,
    LocalWasm,
    LocalWasmGz,
    PreviousReceiptArtifact,
    CanonicalWasmStoreDefault,
}

///
/// PreviousArtifactReceiptKindV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PreviousArtifactReceiptKindV1 {
    DeploymentReceipt,
    StagingReceipt,
}

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
/// RoleArtifactManifestV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RoleArtifactManifestV1 {
    pub schema_version: u32,
    pub manifest_id: String,
    pub network: String,
    pub artifact_root: Option<String>,
    pub role_artifacts: Vec<RoleArtifactV1>,
    pub unresolved_artifacts: Vec<DeploymentObservationGapV1>,
}

///
/// DeploymentDiffV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeploymentDiffV1 {
    pub schema_version: u32,
    pub plan_identity: DeploymentIdentityV1,
    pub observed_identity: Option<DeploymentIdentityV1>,
    pub artifact_diff: Vec<DiffItemV1>,
    pub controller_diff: Vec<DiffItemV1>,
    pub pool_diff: Vec<DiffItemV1>,
    pub embedded_config_diff: Vec<DiffItemV1>,
    pub module_hash_diff: Vec<DiffItemV1>,
    pub verifier_readiness_diff: Vec<DiffItemV1>,
    pub resume_safety: ResumeSafetyV1,
    pub hard_failures: Vec<SafetyFindingV1>,
    pub warnings: Vec<SafetyFindingV1>,
    pub resumable_phases: Vec<String>,
}

///
/// SafetyReportV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SafetyReportV1 {
    pub schema_version: u32,
    pub report_id: String,
    pub diff_id: Option<String>,
    pub status: SafetyStatusV1,
    pub summary: String,
    pub hard_failures: Vec<SafetyFindingV1>,
    pub warnings: Vec<SafetyFindingV1>,
    pub next_actions: Vec<String>,
}

///
/// DeploymentCheckV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeploymentCheckV1 {
    pub schema_version: u32,
    pub check_id: String,
    pub plan: DeploymentPlanV1,
    pub inventory: DeploymentInventoryV1,
    pub diff: DeploymentDiffV1,
    pub report: SafetyReportV1,
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

///
/// DeploymentIdentityV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeploymentIdentityV1 {
    pub deployment_name: String,
    pub network: String,
    pub root_principal: Option<String>,
    pub authority_profile_hash: Option<String>,
    pub role_topology_hash: Option<String>,
    pub deployment_manifest_digest: Option<String>,
    pub canonical_runtime_config_digest: Option<String>,
    pub role_embedded_config_set_digest: Option<String>,
    pub artifact_set_digest: Option<String>,
    pub pool_identity_set_digest: Option<String>,
    pub canic_version: Option<String>,
    pub ic_memory_version: Option<String>,
}

///
/// TrustDomainV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TrustDomainV1 {
    pub root_trust_anchor: Option<String>,
    pub migration_from: Option<String>,
}

///
/// AuthorityProfileV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AuthorityProfileV1 {
    pub profile_id: String,
    pub expected_controllers: Vec<String>,
    pub staging_controllers: Vec<String>,
    pub emergency_controllers: Vec<String>,
}

///
/// RoleArtifactV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RoleArtifactV1 {
    pub role: String,
    pub source: ArtifactSourceV1,
    pub build_profile: String,
    pub wasm_path: Option<String>,
    pub wasm_gz_path: Option<String>,
    pub wasm_gz_size_bytes: Option<u64>,
    pub wasm_sha256: Option<String>,
    pub wasm_gz_sha256: Option<String>,
    pub wasm_gz_sha256_source: Option<ArtifactDigestSourceV1>,
    pub observed_wasm_gz_file_sha256: Option<String>,
    pub observed_wasm_gz_file_sha256_source: Option<ArtifactDigestSourceV1>,
    pub installed_module_hash: Option<String>,
    pub candid_path: Option<String>,
    pub candid_sha256: Option<String>,
    pub raw_config_sha256: Option<String>,
    pub canonical_embedded_config_sha256: Option<String>,
    pub embedded_topology_sha256: Option<String>,
    pub builder_version: Option<String>,
    pub rust_toolchain: Option<String>,
    pub package_version: Option<String>,
}

///
/// ArtifactDigestSourceV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ArtifactDigestSourceV1 {
    ReleaseSetManifest,
    ObservedFileDigest,
    InstalledModuleHash,
}

///
/// ArtifactSourceV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ArtifactSourceV1 {
    LocalBuild,
    ReleaseSet,
    WasmStore,
    External,
    Unknown,
}

///
/// ExpectedCanisterV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExpectedCanisterV1 {
    pub role: String,
    pub canister_id: Option<String>,
    pub control_class: CanisterControlClassV1,
}

///
/// ObservedCanisterV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ObservedCanisterV1 {
    pub canister_id: String,
    pub role: Option<String>,
    pub control_class: CanisterControlClassV1,
    pub controllers: Vec<String>,
    pub module_hash: Option<String>,
    pub status: Option<String>,
    pub root_trust_anchor: Option<String>,
    pub canonical_embedded_config_digest: Option<String>,
    pub role_assignment_source: Option<String>,
}

///
/// CanisterControlClassV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CanisterControlClassV1 {
    DeploymentControlled,
    CanicManagedPool,
    ExternallyImported,
    JointlyControlled,
    UserControlled,
    UnknownUnsafe,
}

///
/// ExpectedPoolCanisterV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExpectedPoolCanisterV1 {
    pub pool: String,
    pub canister_id: Option<String>,
    pub role: Option<String>,
}

///
/// ObservedPoolCanisterV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ObservedPoolCanisterV1 {
    pub pool: String,
    pub canister_id: String,
    pub role: Option<String>,
    pub control_class: CanisterControlClassV1,
}

///
/// LocalDeploymentConfigV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LocalDeploymentConfigV1 {
    pub config_path: Option<String>,
    pub raw_config_sha256: Option<String>,
    pub canonical_embedded_config_sha256: Option<String>,
}

///
/// ObservedArtifactV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ObservedArtifactV1 {
    pub role: String,
    pub artifact_path: String,
    pub file_sha256: Option<String>,
    pub file_sha256_source: Option<ArtifactDigestSourceV1>,
    pub payload_sha256: Option<String>,
    pub payload_size_bytes: Option<u64>,
    pub source: ArtifactSourceV1,
}

///
/// VerifierReadinessExpectationV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct VerifierReadinessExpectationV1 {
    pub required: bool,
    pub expected_role_epochs: Vec<RoleEpochExpectationV1>,
}

///
/// VerifierReadinessObservationV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct VerifierReadinessObservationV1 {
    pub status: ObservationStatusV1,
    pub role_epochs: Vec<RoleEpochObservationV1>,
}

///
/// RoleEpochExpectationV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RoleEpochExpectationV1 {
    pub role: String,
    pub minimum_epoch: u64,
}

///
/// RoleEpochObservationV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RoleEpochObservationV1 {
    pub role: String,
    pub observed_epoch: Option<u64>,
    pub status: ObservationStatusV1,
}

///
/// DeploymentAssumptionV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeploymentAssumptionV1 {
    pub key: String,
    pub description: String,
}

///
/// DeploymentObservationGapV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeploymentObservationGapV1 {
    pub key: String,
    pub description: String,
}

///
/// PhaseReceiptV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PhaseReceiptV1 {
    pub phase: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub attempted_action: String,
    pub verified_postcondition: VerifiedPostconditionV1,
}

///
/// VerifiedPostconditionV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct VerifiedPostconditionV1 {
    pub status: ObservationStatusV1,
    pub evidence: Vec<String>,
}

///
/// DeploymentExecutionStatusV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum DeploymentExecutionStatusV1 {
    NotStarted,
    InProgress,
    FailedBeforeMutation,
    PartiallyApplied,
    FailedAfterMutation,
    Complete,
}

///
/// DeploymentCommandResultV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum DeploymentCommandResultV1 {
    NotFinished,
    Succeeded,
    Failed { code: String, message: String },
}

///
/// RolePhaseReceiptV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RolePhaseReceiptV1 {
    pub role: String,
    pub phase: String,
    pub result: RolePhaseResultV1,
    pub previous_module_hash: Option<String>,
    pub target_module_hash: Option<String>,
    pub observed_module_hash_after: Option<String>,
    pub artifact_digest: Option<String>,
    pub canonical_embedded_config_sha256: Option<String>,
    pub error: Option<String>,
}

///
/// RolePhaseResultV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RolePhaseResultV1 {
    Applied,
    Failed,
    Skipped,
    NotAttempted,
    VerifiedAlreadyApplied,
}

///
/// DiffItemV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DiffItemV1 {
    pub category: String,
    pub subject: String,
    pub expected: Option<String>,
    pub observed: Option<String>,
    pub severity: SafetySeverityV1,
}

///
/// ResumeSafetyV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ResumeSafetyV1 {
    pub status: SafetyStatusV1,
    pub reasons: Vec<String>,
}

///
/// SafetyFindingV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SafetyFindingV1 {
    pub code: String,
    pub message: String,
    pub severity: SafetySeverityV1,
    pub subject: Option<String>,
}

///
/// SafetyStatusV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum SafetyStatusV1 {
    NotEvaluated,
    Safe,
    Warning,
    Blocked,
}

///
/// SafetySeverityV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum SafetySeverityV1 {
    Info,
    Warning,
    HardFailure,
}

///
/// ObservationStatusV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ObservationStatusV1 {
    NotObserved,
    Observed,
    Missing,
    Inconclusive,
}
