use super::super::{
    ArtifactPromotionPlanV1, ArtifactPromotionProvenanceReportV1, BuildMaterializationEvidenceV1,
    BuildMaterializationInputV1, BuildMaterializationResultV1, BuildRecipeIdentityV1,
    DeploymentExecutionPreflightV1, DeploymentPlanV1, DeploymentReceiptV1,
    PromotionArtifactIdentityReportV1, PromotionMaterializationIdentityReportV1,
    PromotionPlanTransformV1, PromotionReadinessV1, PromotionTargetExecutionLineageV1,
    PromotionWasmStoreCatalogEntryV1, PromotionWasmStoreCatalogVerificationV1,
    PromotionWasmStoreIdentityReportV1, RolePromotionInputV1, RolePromotionPolicyV1,
    StagingReceiptV1,
};

///
/// PromotionReadinessRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionReadinessRequest {
    pub readiness_id: String,
    pub target_plan: DeploymentPlanV1,
    pub inputs: Vec<RolePromotionInputV1>,
}

///
/// PromotionReadinessWithPolicyRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionReadinessWithPolicyRequest {
    pub readiness_id: String,
    pub target_plan: DeploymentPlanV1,
    pub inputs: Vec<RolePromotionInputV1>,
    pub policies: Vec<RolePromotionPolicyV1>,
}

///
/// PromotionPlanTransformRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionPlanTransformRequest {
    pub promoted_plan_id: String,
    pub target_plan: DeploymentPlanV1,
    pub inputs: Vec<RolePromotionInputV1>,
}

///
/// PromotionPlanTransformWithMaterializationRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionPlanTransformWithMaterializationRequest {
    pub promoted_plan_id: String,
    pub target_plan: DeploymentPlanV1,
    pub inputs: Vec<RolePromotionInputV1>,
    pub materialization_evidence: Vec<BuildMaterializationEvidenceV1>,
}

///
/// PromotionPlanTransformEvidenceRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionPlanTransformEvidenceRequest {
    pub evidence_id: String,
    pub generated_at: String,
    pub transform: PromotionPlanTransformV1,
}

///
/// ArtifactPromotionPlanRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactPromotionPlanRequest {
    pub plan_id: String,
    pub generated_at: String,
    pub readiness: PromotionReadinessV1,
    pub artifact_identity_report: PromotionArtifactIdentityReportV1,
    pub transform: PromotionPlanTransformV1,
    pub target_execution_lineage: Option<PromotionTargetExecutionLineageV1>,
}

///
/// ArtifactPromotionProvenanceReportRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactPromotionProvenanceReportRequest {
    pub report_id: String,
    pub artifact_promotion_plan: ArtifactPromotionPlanV1,
    pub wasm_store_identity_report: Option<PromotionWasmStoreIdentityReportV1>,
    pub wasm_store_catalog_verification: Option<PromotionWasmStoreCatalogVerificationV1>,
    pub materialization_identity_report: Option<PromotionMaterializationIdentityReportV1>,
}

///
/// ArtifactPromotionExecutionReceiptRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactPromotionExecutionReceiptRequest {
    pub receipt_id: String,
    pub provenance_report: ArtifactPromotionProvenanceReportV1,
    pub deployment_receipt: DeploymentReceiptV1,
}

///
/// PromotionTargetExecutionLineageRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionTargetExecutionLineageRequest {
    pub lineage_id: String,
    pub generated_at: String,
    pub transform: PromotionPlanTransformV1,
    pub execution_preflight: DeploymentExecutionPreflightV1,
}

///
/// PromotionArtifactIdentityReportRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionArtifactIdentityReportRequest {
    pub report_id: String,
    pub inputs: Vec<RolePromotionInputV1>,
}

///
/// PromotionWasmStoreIdentityReportRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionWasmStoreIdentityReportRequest {
    pub report_id: String,
    pub staging_receipts: Vec<StagingReceiptV1>,
}

///
/// PromotionWasmStoreCatalogVerificationRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionWasmStoreCatalogVerificationRequest {
    pub verification_id: String,
    pub wasm_store_identity_report: PromotionWasmStoreIdentityReportV1,
    pub catalog_entries: Vec<PromotionWasmStoreCatalogEntryV1>,
}

///
/// BuildMaterializationEvidenceRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildMaterializationEvidenceRequest {
    pub evidence_id: String,
    pub recipe: BuildRecipeIdentityV1,
    pub materialization_input: BuildMaterializationInputV1,
    pub materialization_result: BuildMaterializationResultV1,
}

///
/// PromotionMaterializationIdentityReportRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionMaterializationIdentityReportRequest {
    pub report_id: String,
    pub evidence: Vec<BuildMaterializationEvidenceV1>,
}

///
/// PromotionPolicyCheckRequest
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionPolicyCheckRequest {
    pub check_id: String,
    pub inputs: Vec<RolePromotionInputV1>,
    pub policies: Vec<RolePromotionPolicyV1>,
}
