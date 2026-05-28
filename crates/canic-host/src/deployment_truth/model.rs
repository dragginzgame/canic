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
    pub observed_root: Option<DeploymentRootObservationV1>,
    pub local_config: LocalDeploymentConfigV1,
    pub observed_canisters: Vec<ObservedCanisterV1>,
    pub observed_pool: Vec<ObservedPoolCanisterV1>,
    pub observed_artifacts: Vec<ObservedArtifactV1>,
    pub observed_verifier_readiness: VerifierReadinessObservationV1,
    pub unresolved_observations: Vec<DeploymentObservationGapV1>,
}

///
/// DeploymentRootObservationV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeploymentRootObservationV1 {
    pub deployment_name: String,
    pub network: String,
    pub fleet_template: String,
    pub root_principal: String,
    pub observed_canister_id: String,
    pub observation_source: DeploymentRootObservationSourceV1,
    pub control_class: CanisterControlClassV1,
    pub controllers: Vec<String>,
    pub module_hash: Option<String>,
    pub status: Option<String>,
    pub role_assignment_source: Option<String>,
}

///
/// DeploymentRootObservationSourceV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum DeploymentRootObservationSourceV1 {
    IcpCanisterStatus,
    LocalDeploymentState,
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
    pub promotion_policy_check_digest: String,
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
    pub materialization_evidence_digest: String,
    pub recipe: BuildRecipeIdentityV1,
    pub materialization_input: BuildMaterializationInputV1,
    pub materialization_result: BuildMaterializationResultV1,
    pub computed_materialization_input_digest: String,
    pub recipe_id_matches_input: bool,
    pub recipe_id_matches_result: bool,
    pub materialization_input_digest_matches_result: bool,
}

///
/// PromotionMaterializationIdentityReportV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PromotionMaterializationIdentityReportV1 {
    pub schema_version: u32,
    pub report_id: String,
    pub materialization_identity_report_digest: String,
    pub status: PromotionReadinessStatusV1,
    pub roles: Vec<RolePromotionMaterializationIdentityV1>,
    pub output_groups: Vec<PromotionMaterializationOutputGroupV1>,
    pub blockers: Vec<SafetyFindingV1>,
}

///
/// RolePromotionMaterializationIdentityV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RolePromotionMaterializationIdentityV1 {
    pub role: String,
    pub evidence_id: String,
    pub materialization_evidence_digest: String,
    pub recipe_id: String,
    pub materialization_input_id: String,
    pub materialization_result_id: String,
    pub materialization_input_digest: String,
    pub canonical_embedded_config_sha256: String,
    pub network: String,
    pub root_trust_anchor: String,
    pub runtime_variant: String,
    pub wasm_sha256: String,
    pub wasm_gz_sha256: String,
    pub installed_module_hash: String,
    pub candid_sha256: String,
}

///
/// PromotionMaterializationOutputGroupV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PromotionMaterializationOutputGroupV1 {
    pub output_identity_key: String,
    pub roles: Vec<String>,
    pub wasm_sha256: String,
    pub wasm_gz_sha256: String,
    pub installed_module_hash: String,
    pub candid_sha256: String,
}

///
/// PromotionArtifactIdentityReportV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PromotionArtifactIdentityReportV1 {
    pub schema_version: u32,
    pub report_id: String,
    pub artifact_identity_report_digest: String,
    pub status: PromotionReadinessStatusV1,
    pub summary: PromotionArtifactIdentitySummaryV1,
    pub roles: Vec<RolePromotionArtifactIdentityV1>,
    pub identity_groups: Vec<PromotionArtifactIdentityGroupV1>,
    pub blockers: Vec<SafetyFindingV1>,
}

///
/// PromotionArtifactIdentitySummaryV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PromotionArtifactIdentitySummaryV1 {
    pub role_count: usize,
    pub identity_group_count: usize,
    pub shared_identity_group_count: usize,
    pub digest_pinned_role_count: usize,
    pub source_build_role_count: usize,
    pub deferred_identity_role_count: usize,
}

///
/// PromotionWasmStoreIdentityReportV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PromotionWasmStoreIdentityReportV1 {
    pub schema_version: u32,
    pub report_id: String,
    pub wasm_store_identity_report_digest: String,
    pub status: PromotionReadinessStatusV1,
    pub roles: Vec<RolePromotionWasmStoreIdentityV1>,
    pub blockers: Vec<SafetyFindingV1>,
}

///
/// RolePromotionWasmStoreIdentityV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RolePromotionWasmStoreIdentityV1 {
    pub role: String,
    pub artifact_identity: String,
    pub transport: ArtifactTransportV1,
    pub wasm_store_locator: Option<String>,
    pub prepared_chunk_hashes: Vec<String>,
    pub published_chunk_count: usize,
    pub verified_postcondition: VerifiedPostconditionV1,
}

///
/// PromotionWasmStoreCatalogEntryV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PromotionWasmStoreCatalogEntryV1 {
    pub locator: String,
    pub artifact_identity: String,
    pub published_chunk_count: usize,
}

///
/// PromotionWasmStoreCatalogVerificationV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PromotionWasmStoreCatalogVerificationV1 {
    pub schema_version: u32,
    pub verification_id: String,
    pub wasm_store_catalog_verification_digest: String,
    pub wasm_store_identity_report_id: String,
    pub status: PromotionReadinessStatusV1,
    pub roles: Vec<RolePromotionWasmStoreCatalogVerificationV1>,
    pub blockers: Vec<SafetyFindingV1>,
}

///
/// RolePromotionWasmStoreCatalogVerificationV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RolePromotionWasmStoreCatalogVerificationV1 {
    pub role: String,
    pub wasm_store_locator: String,
    pub expected_artifact_identity: String,
    pub observed_artifact_identity: Option<String>,
    pub expected_published_chunk_count: usize,
    pub observed_published_chunk_count: Option<usize>,
    pub catalog_entry_present: bool,
    pub catalog_matches: bool,
    pub catalog_observation_digest: String,
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
    pub promotion_readiness_digest: String,
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
    pub artifact_promotion_plan_digest: String,
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
/// ArtifactPromotionProvenanceReportV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ArtifactPromotionProvenanceReportV1 {
    pub schema_version: u32,
    pub report_id: String,
    pub status: PromotionReadinessStatusV1,
    pub artifact_promotion_plan_id: String,
    pub artifact_promotion_plan_digest: String,
    pub target_plan_id: String,
    pub promoted_plan_id: String,
    pub promotion_plan_lineage_digest: String,
    pub provenance_report_digest: String,
    pub readiness_id: String,
    pub artifact_identity_report_id: String,
    pub transform_id: String,
    pub target_execution_lineage_id: Option<String>,
    pub wasm_store_identity_report_id: Option<String>,
    pub wasm_store_identity_report_digest: Option<String>,
    pub wasm_store_catalog_verification_id: Option<String>,
    pub wasm_store_catalog_verification_digest: Option<String>,
    pub materialization_identity_report_id: Option<String>,
    pub materialization_identity_report_digest: Option<String>,
    pub execution_attempted: bool,
    pub roles: Vec<RolePromotionProvenanceV1>,
    pub blockers: Vec<SafetyFindingV1>,
}

///
/// ArtifactPromotionExecutionReceiptV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ArtifactPromotionExecutionReceiptV1 {
    pub schema_version: u32,
    pub receipt_id: String,
    pub execution_receipt_digest: String,
    pub artifact_promotion_plan_id: String,
    pub artifact_promotion_plan_digest: String,
    pub provenance_report_id: String,
    pub provenance_report_digest: String,
    pub provenance_status: PromotionReadinessStatusV1,
    pub promoted_plan_id: String,
    pub promotion_plan_lineage_digest: String,
    pub operation_id: String,
    pub operation_status: DeploymentExecutionStatusV1,
    pub command_result: DeploymentCommandResultV1,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub deployment_receipt: DeploymentReceiptV1,
    pub roles: Vec<RolePromotionExecutionReceiptV1>,
}

///
/// RolePromotionExecutionReceiptV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RolePromotionExecutionReceiptV1 {
    pub role: String,
    pub promotion_level: PromotionArtifactLevelV1,
    pub materialization_evidence_id: Option<String>,
    pub materialization_evidence_digest: Option<String>,
    pub wasm_store_locator: Option<String>,
    pub wasm_store_catalog_observation_digest: Option<String>,
    pub role_phase_result: Option<RolePhaseResultV1>,
    pub artifact_digest: Option<String>,
    pub observed_module_hash_after: Option<String>,
    pub canonical_embedded_config_sha256: Option<String>,
}

///
/// RolePromotionProvenanceV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RolePromotionProvenanceV1 {
    pub role: String,
    pub promotion_level: PromotionArtifactLevelV1,
    pub source_kind: RoleArtifactSourceKindV1,
    pub artifact_identity_changed: bool,
    pub embedded_config_changed: bool,
    pub target_materialization_preserved: bool,
    pub materialization_evidence_id: Option<String>,
    pub materialization_evidence_digest: Option<String>,
    pub wasm_store_locator: Option<String>,
    pub wasm_store_catalog_observation_digest: Option<String>,
}

///
/// PromotionPlanTransformEvidenceV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PromotionPlanTransformEvidenceV1 {
    pub schema_version: u32,
    pub evidence_id: String,
    pub promotion_plan_transform_evidence_digest: String,
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
    pub materialization_evidence_digest: String,
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

///
/// DeploymentRootVerificationEvidenceStatusV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum DeploymentRootVerificationEvidenceStatusV1 {
    EvidenceSatisfied,
    VerificationFailed,
    NotApplicable,
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

///
/// DeploymentRootVerificationStateV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum DeploymentRootVerificationStateV1 {
    NotVerified,
    Verified,
}

///
/// DeploymentComparisonReportV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeploymentComparisonReportV1 {
    pub schema_version: u32,
    pub report_id: String,
    pub report_digest: String,
    pub compared_at: String,
    pub left: DeploymentComparisonTargetV1,
    pub right: DeploymentComparisonTargetV1,
    pub status: SafetyStatusV1,
    pub identity_diff: Vec<DeploymentComparisonDiffV1>,
    pub artifact_diff: Vec<DeploymentComparisonDiffV1>,
    pub module_hash_diff: Vec<DeploymentComparisonDiffV1>,
    pub embedded_config_diff: Vec<DeploymentComparisonDiffV1>,
    pub authority_diff: Vec<DeploymentComparisonDiffV1>,
    pub pool_diff: Vec<DeploymentComparisonDiffV1>,
    pub verifier_readiness_diff: Vec<DeploymentComparisonDiffV1>,
    pub external_lifecycle_diff: Vec<DeploymentComparisonDiffV1>,
    pub hard_failures: Vec<SafetyFindingV1>,
    pub warnings: Vec<SafetyFindingV1>,
    pub next_actions: Vec<String>,
}

///
/// DeploymentComparisonTargetV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeploymentComparisonTargetV1 {
    pub label: String,
    pub check_id: String,
    pub check_digest: String,
    pub plan_id: String,
    pub plan_digest: String,
    pub inventory_id: String,
    pub inventory_digest: String,
    pub deployment_identity: DeploymentIdentityV1,
}

///
/// DeploymentComparisonDiffV1
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeploymentComparisonDiffV1 {
    pub category: DeploymentComparisonCategoryV1,
    pub subject: String,
    pub left: Option<String>,
    pub right: Option<String>,
    pub severity: SafetySeverityV1,
    pub message: String,
}

///
/// DeploymentComparisonCategoryV1
///
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum DeploymentComparisonCategoryV1 {
    Identity,
    TrustDomain,
    Artifact,
    ModuleHash,
    EmbeddedConfig,
    Authority,
    Pool,
    VerifierReadiness,
    ExternalLifecycle,
}

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
