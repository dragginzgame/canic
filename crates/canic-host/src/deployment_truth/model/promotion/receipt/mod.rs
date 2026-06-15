use super::super::{
    DeploymentCommandResultV1, DeploymentExecutionStatusV1, DeploymentReceiptV1, RolePhaseResultV1,
};
use super::PromotionReadinessStatusV1;
use super::source::PromotionArtifactLevelV1;
use serde::{Deserialize, Serialize};

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
