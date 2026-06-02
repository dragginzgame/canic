use super::super::DeployCommandError;
use canic_host::deployment_truth::{
    ArtifactPromotionExecutionReceiptRequest, ArtifactPromotionExecutionReceiptV1,
    ArtifactPromotionPlanRequest, ArtifactPromotionPlanV1,
    ArtifactPromotionProvenanceReportRequest, ArtifactPromotionProvenanceReportV1,
    BuildMaterializationEvidenceV1, DeploymentExecutionPreflightV1, DeploymentPlanV1,
    DeploymentReceiptV1, PromotionArtifactIdentityReportRequest, PromotionArtifactIdentityReportV1,
    PromotionMaterializationIdentityReportRequest, PromotionMaterializationIdentityReportV1,
    PromotionPlanTransformEvidenceRequest, PromotionPlanTransformEvidenceV1,
    PromotionPlanTransformRequest, PromotionPlanTransformV1,
    PromotionPlanTransformWithMaterializationRequest, PromotionPolicyCheckRequest,
    PromotionPolicyCheckV1, PromotionReadinessRequest, PromotionReadinessV1,
    PromotionTargetExecutionLineageRequest, PromotionTargetExecutionLineageV1,
    PromotionWasmStoreCatalogEntryV1, PromotionWasmStoreCatalogVerificationRequest,
    PromotionWasmStoreCatalogVerificationV1, PromotionWasmStoreIdentityReportRequest,
    PromotionWasmStoreIdentityReportV1, RolePromotionInputV1, RolePromotionPolicyV1,
    StagingReceiptV1, artifact_promotion_execution_receipt, artifact_promotion_plan,
    artifact_promotion_provenance_report, check_promotion_policy, check_promotion_readiness,
    promoted_deployment_plan_transform_from_inputs,
    promoted_deployment_plan_transform_from_inputs_with_materialization,
    promotion_artifact_identity_report_from_inputs,
    promotion_materialization_identity_report_from_evidence, promotion_plan_transform_evidence,
    promotion_target_execution_lineage, promotion_wasm_store_catalog_verification,
    promotion_wasm_store_identity_report_from_staging,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub(super) struct PromotionReadinessFile {
    readiness_id: String,
    target_plan: DeploymentPlanV1,
    inputs: Vec<RolePromotionInputV1>,
}

#[derive(Deserialize)]
pub(super) struct PromotionArtifactIdentityFile {
    report_id: String,
    inputs: Vec<RolePromotionInputV1>,
}

#[derive(Deserialize)]
pub(super) struct PromotionPlanTransformFile {
    promoted_plan_id: String,
    target_plan: DeploymentPlanV1,
    inputs: Vec<RolePromotionInputV1>,
    materialization_evidence: Option<Vec<BuildMaterializationEvidenceV1>>,
}

#[derive(Deserialize)]
pub(super) struct PromotionPlanTransformEvidenceFile {
    evidence_id: String,
    generated_at: String,
    transform: PromotionPlanTransformV1,
}

#[derive(Deserialize)]
pub(super) struct PromotionTargetExecutionLineageFile {
    lineage_id: String,
    generated_at: String,
    transform: PromotionPlanTransformV1,
    execution_preflight: DeploymentExecutionPreflightV1,
}

#[derive(Deserialize)]
pub(super) struct ArtifactPromotionPlanFile {
    plan_id: String,
    generated_at: String,
    readiness: PromotionReadinessV1,
    artifact_identity_report: PromotionArtifactIdentityReportV1,
    transform: PromotionPlanTransformV1,
    target_execution_lineage: Option<PromotionTargetExecutionLineageV1>,
}

#[derive(Deserialize)]
pub(super) struct ArtifactPromotionProvenanceFile {
    report_id: String,
    artifact_promotion_plan: ArtifactPromotionPlanV1,
    wasm_store_identity_report: Option<PromotionWasmStoreIdentityReportV1>,
    wasm_store_catalog_verification: Option<PromotionWasmStoreCatalogVerificationV1>,
    materialization_identity_report: Option<PromotionMaterializationIdentityReportV1>,
}

#[derive(Deserialize)]
pub(super) struct PromotionWasmStoreIdentityFile {
    report_id: String,
    staging_receipts: Vec<StagingReceiptV1>,
}

#[derive(Deserialize)]
pub(super) struct PromotionWasmStoreCatalogVerificationFile {
    verification_id: String,
    wasm_store_identity_report: PromotionWasmStoreIdentityReportV1,
    catalog_entries: Vec<PromotionWasmStoreCatalogEntryV1>,
}

#[derive(Deserialize)]
pub(super) struct ArtifactPromotionExecutionReceiptFile {
    receipt_id: String,
    provenance_report: ArtifactPromotionProvenanceReportV1,
    deployment_receipt: DeploymentReceiptV1,
}

#[derive(Deserialize)]
pub(super) struct PromotionPolicyCheckFile {
    check_id: String,
    inputs: Vec<RolePromotionInputV1>,
    policies: Vec<RolePromotionPolicyV1>,
}

#[derive(Deserialize)]
pub(super) struct PromotionMaterializationIdentityFile {
    report_id: String,
    evidence: Vec<BuildMaterializationEvidenceV1>,
}
pub(super) fn build_promotion_readiness(
    request: PromotionReadinessFile,
) -> Result<PromotionReadinessV1, DeployCommandError> {
    check_promotion_readiness(&PromotionReadinessRequest {
        readiness_id: request.readiness_id,
        target_plan: request.target_plan,
        inputs: request.inputs,
    })
    .map_err(|err| DeployCommandError::Check(Box::new(err)))
}

pub(super) fn build_promotion_artifact_identity_report(
    request: PromotionArtifactIdentityFile,
) -> Result<PromotionArtifactIdentityReportV1, DeployCommandError> {
    promotion_artifact_identity_report_from_inputs(PromotionArtifactIdentityReportRequest {
        report_id: request.report_id,
        inputs: request.inputs,
    })
    .map_err(|err| DeployCommandError::Check(Box::new(err)))
}

pub(super) fn build_promotion_plan_transform(
    request: PromotionPlanTransformFile,
) -> Result<PromotionPlanTransformV1, DeployCommandError> {
    if let Some(materialization_evidence) = request.materialization_evidence {
        return promoted_deployment_plan_transform_from_inputs_with_materialization(
            &PromotionPlanTransformWithMaterializationRequest {
                promoted_plan_id: request.promoted_plan_id,
                target_plan: request.target_plan,
                inputs: request.inputs,
                materialization_evidence,
            },
        )
        .map_err(|err| DeployCommandError::Check(Box::new(err)));
    }

    promoted_deployment_plan_transform_from_inputs(&PromotionPlanTransformRequest {
        promoted_plan_id: request.promoted_plan_id,
        target_plan: request.target_plan,
        inputs: request.inputs,
    })
    .map_err(|err| DeployCommandError::Check(Box::new(err)))
}

pub(super) fn build_promotion_plan_transform_evidence(
    request: PromotionPlanTransformEvidenceFile,
) -> Result<PromotionPlanTransformEvidenceV1, DeployCommandError> {
    promotion_plan_transform_evidence(PromotionPlanTransformEvidenceRequest {
        evidence_id: request.evidence_id,
        generated_at: request.generated_at,
        transform: request.transform,
    })
    .map_err(|err| DeployCommandError::Check(Box::new(err)))
}

pub(super) fn build_promotion_target_execution_lineage(
    request: PromotionTargetExecutionLineageFile,
) -> Result<PromotionTargetExecutionLineageV1, DeployCommandError> {
    promotion_target_execution_lineage(PromotionTargetExecutionLineageRequest {
        lineage_id: request.lineage_id,
        generated_at: request.generated_at,
        transform: request.transform,
        execution_preflight: request.execution_preflight,
    })
    .map_err(|err| DeployCommandError::Check(Box::new(err)))
}

pub(super) fn build_artifact_promotion_plan(
    request: ArtifactPromotionPlanFile,
) -> Result<ArtifactPromotionPlanV1, DeployCommandError> {
    artifact_promotion_plan(ArtifactPromotionPlanRequest {
        plan_id: request.plan_id,
        generated_at: request.generated_at,
        readiness: request.readiness,
        artifact_identity_report: request.artifact_identity_report,
        transform: request.transform,
        target_execution_lineage: request.target_execution_lineage,
    })
    .map_err(|err| DeployCommandError::Check(Box::new(err)))
}

pub(super) fn build_artifact_promotion_provenance_report(
    request: ArtifactPromotionProvenanceFile,
) -> Result<ArtifactPromotionProvenanceReportV1, DeployCommandError> {
    artifact_promotion_provenance_report(ArtifactPromotionProvenanceReportRequest {
        report_id: request.report_id,
        artifact_promotion_plan: request.artifact_promotion_plan,
        wasm_store_identity_report: request.wasm_store_identity_report,
        wasm_store_catalog_verification: request.wasm_store_catalog_verification,
        materialization_identity_report: request.materialization_identity_report,
    })
    .map_err(|err| DeployCommandError::Check(Box::new(err)))
}

pub(super) fn build_promotion_wasm_store_identity_report(
    request: PromotionWasmStoreIdentityFile,
) -> Result<PromotionWasmStoreIdentityReportV1, DeployCommandError> {
    promotion_wasm_store_identity_report_from_staging(PromotionWasmStoreIdentityReportRequest {
        report_id: request.report_id,
        staging_receipts: request.staging_receipts,
    })
    .map_err(|err| DeployCommandError::Check(Box::new(err)))
}

pub(super) fn build_promotion_wasm_store_catalog_verification(
    request: PromotionWasmStoreCatalogVerificationFile,
) -> Result<PromotionWasmStoreCatalogVerificationV1, DeployCommandError> {
    promotion_wasm_store_catalog_verification(PromotionWasmStoreCatalogVerificationRequest {
        verification_id: request.verification_id,
        wasm_store_identity_report: request.wasm_store_identity_report,
        catalog_entries: request.catalog_entries,
    })
    .map_err(|err| DeployCommandError::Check(Box::new(err)))
}

pub(super) fn build_artifact_promotion_execution_receipt(
    request: ArtifactPromotionExecutionReceiptFile,
) -> Result<ArtifactPromotionExecutionReceiptV1, DeployCommandError> {
    artifact_promotion_execution_receipt(ArtifactPromotionExecutionReceiptRequest {
        receipt_id: request.receipt_id,
        provenance_report: request.provenance_report,
        deployment_receipt: request.deployment_receipt,
    })
    .map_err(|err| DeployCommandError::Check(Box::new(err)))
}

pub(super) fn build_promotion_policy_check(
    request: PromotionPolicyCheckFile,
) -> Result<PromotionPolicyCheckV1, DeployCommandError> {
    check_promotion_policy(PromotionPolicyCheckRequest {
        check_id: request.check_id,
        inputs: request.inputs,
        policies: request.policies,
    })
    .map_err(|err| DeployCommandError::Check(Box::new(err)))
}

pub(super) fn build_promotion_materialization_identity_report(
    request: PromotionMaterializationIdentityFile,
) -> Result<PromotionMaterializationIdentityReportV1, DeployCommandError> {
    promotion_materialization_identity_report_from_evidence(
        PromotionMaterializationIdentityReportRequest {
            report_id: request.report_id,
            evidence: request.evidence,
        },
    )
    .map_err(|err| DeployCommandError::Check(Box::new(err)))
}
