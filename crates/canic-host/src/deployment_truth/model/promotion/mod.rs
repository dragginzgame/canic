use super::{
    ArtifactSourceV1, DeploymentCommandResultV1, DeploymentExecutionPreflightV1,
    DeploymentExecutionStatusV1, DeploymentPlanV1, DeploymentReceiptV1, RolePhaseResultV1,
    SafetyFindingV1, VerifiedPostconditionV1,
};
use serde::{Deserialize, Serialize};

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
