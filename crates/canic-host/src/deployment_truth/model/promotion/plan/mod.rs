use super::super::{
    ArtifactSourceV1, DeploymentExecutionPreflightV1, DeploymentPlanV1, SafetyFindingV1,
};
use super::identity::PromotionArtifactIdentityReportV1;
use super::materialization::RolePromotionMaterializationLinkV1;
use super::source::{
    PromotionArtifactLevelV1, PromotionReadinessStatusV1, RoleArtifactSourceKindV1,
};
use serde::{Deserialize, Serialize};

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
