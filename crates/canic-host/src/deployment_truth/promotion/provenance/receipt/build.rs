use super::super::super::{
    digest::artifact_promotion_execution_receipt_digest,
    request::ArtifactPromotionExecutionReceiptRequest,
};
use crate::deployment_truth::{
    ArtifactPromotionExecutionReceiptV1, DEPLOYMENT_TRUTH_SCHEMA_VERSION, DeploymentReceiptV1,
    RolePromotionExecutionReceiptV1, RolePromotionProvenanceV1,
};

pub(super) fn build_artifact_promotion_execution_receipt(
    request: ArtifactPromotionExecutionReceiptRequest,
) -> ArtifactPromotionExecutionReceiptV1 {
    let roles = request
        .provenance_report
        .roles
        .iter()
        .map(|role| role_promotion_execution_receipt(role, &request.deployment_receipt))
        .collect::<Vec<_>>();
    let mut receipt = ArtifactPromotionExecutionReceiptV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        receipt_id: request.receipt_id,
        execution_receipt_digest: String::new(),
        artifact_promotion_plan_id: request.provenance_report.artifact_promotion_plan_id.clone(),
        artifact_promotion_plan_digest: request
            .provenance_report
            .artifact_promotion_plan_digest
            .clone(),
        provenance_report_id: request.provenance_report.report_id.clone(),
        provenance_report_digest: request.provenance_report.provenance_report_digest,
        provenance_status: request.provenance_report.status,
        promoted_plan_id: request.provenance_report.promoted_plan_id.clone(),
        promotion_plan_lineage_digest: request.provenance_report.promotion_plan_lineage_digest,
        operation_id: request.deployment_receipt.operation_id.clone(),
        operation_status: request.deployment_receipt.operation_status,
        command_result: request.deployment_receipt.command_result.clone(),
        started_at: request.deployment_receipt.started_at.clone(),
        finished_at: request.deployment_receipt.finished_at.clone(),
        deployment_receipt: request.deployment_receipt,
        roles,
    };
    receipt.execution_receipt_digest = artifact_promotion_execution_receipt_digest(&receipt);
    receipt
}

fn role_promotion_execution_receipt(
    role: &RolePromotionProvenanceV1,
    deployment_receipt: &DeploymentReceiptV1,
) -> RolePromotionExecutionReceiptV1 {
    let role_receipt = deployment_receipt
        .role_phase_receipts
        .iter()
        .rev()
        .find(|receipt| receipt.role == role.role);
    RolePromotionExecutionReceiptV1 {
        role: role.role.clone(),
        promotion_level: role.promotion_level,
        materialization_evidence_id: role.materialization_evidence_id.clone(),
        materialization_evidence_digest: role.materialization_evidence_digest.clone(),
        wasm_store_locator: role.wasm_store_locator.clone(),
        wasm_store_catalog_observation_digest: role.wasm_store_catalog_observation_digest.clone(),
        role_phase_result: role_receipt.map(|receipt| receipt.result),
        artifact_digest: role_receipt.and_then(|receipt| receipt.artifact_digest.clone()),
        observed_module_hash_after: role_receipt
            .and_then(|receipt| receipt.observed_module_hash_after.clone()),
        canonical_embedded_config_sha256: role_receipt
            .and_then(|receipt| receipt.canonical_embedded_config_sha256.clone()),
    }
}
