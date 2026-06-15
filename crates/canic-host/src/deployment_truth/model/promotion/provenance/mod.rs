use super::super::SafetyFindingV1;
use super::source::{
    PromotionArtifactLevelV1, PromotionReadinessStatusV1, RoleArtifactSourceKindV1,
};
use serde::{Deserialize, Serialize};

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
