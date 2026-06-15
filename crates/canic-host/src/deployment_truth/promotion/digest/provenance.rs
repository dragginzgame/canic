use crate::deployment_truth::{
    ArtifactPromotionExecutionReceiptV1, ArtifactPromotionProvenanceReportV1,
    DeploymentCommandResultV1, DeploymentExecutionStatusV1, DeploymentReceiptV1,
    PromotionReadinessStatusV1, RolePromotionExecutionReceiptV1, RolePromotionProvenanceV1,
    SafetyFindingV1, stable_json_sha256_hex,
};
use serde::Serialize;

#[derive(Serialize)]
struct ArtifactPromotionProvenanceDigestInput<'a> {
    schema_version: u32,
    report_id: &'a str,
    status: PromotionReadinessStatusV1,
    artifact_promotion_plan_id: &'a str,
    artifact_promotion_plan_digest: &'a str,
    target_plan_id: &'a str,
    promoted_plan_id: &'a str,
    promotion_plan_lineage_digest: &'a str,
    readiness_id: &'a str,
    artifact_identity_report_id: &'a str,
    transform_id: &'a str,
    target_execution_lineage_id: Option<&'a str>,
    wasm_store_identity_report_id: Option<&'a str>,
    wasm_store_identity_report_digest: Option<&'a str>,
    wasm_store_catalog_verification_id: Option<&'a str>,
    wasm_store_catalog_verification_digest: Option<&'a str>,
    materialization_identity_report_id: Option<&'a str>,
    materialization_identity_report_digest: Option<&'a str>,
    execution_attempted: bool,
    roles: &'a [RolePromotionProvenanceV1],
    blockers: &'a [SafetyFindingV1],
}

#[derive(Serialize)]
struct ArtifactPromotionExecutionReceiptDigestInput<'a> {
    schema_version: u32,
    receipt_id: &'a str,
    artifact_promotion_plan_id: &'a str,
    artifact_promotion_plan_digest: &'a str,
    provenance_report_id: &'a str,
    provenance_report_digest: &'a str,
    provenance_status: PromotionReadinessStatusV1,
    promoted_plan_id: &'a str,
    promotion_plan_lineage_digest: &'a str,
    operation_id: &'a str,
    operation_status: DeploymentExecutionStatusV1,
    command_result: &'a DeploymentCommandResultV1,
    started_at: &'a str,
    finished_at: Option<&'a str>,
    deployment_receipt: &'a DeploymentReceiptV1,
    roles: &'a [RolePromotionExecutionReceiptV1],
}

pub(in crate::deployment_truth::promotion) fn artifact_promotion_provenance_digest(
    report: &ArtifactPromotionProvenanceReportV1,
) -> String {
    stable_json_sha256_hex(&ArtifactPromotionProvenanceDigestInput {
        schema_version: report.schema_version,
        report_id: &report.report_id,
        status: report.status,
        artifact_promotion_plan_id: &report.artifact_promotion_plan_id,
        artifact_promotion_plan_digest: &report.artifact_promotion_plan_digest,
        target_plan_id: &report.target_plan_id,
        promoted_plan_id: &report.promoted_plan_id,
        promotion_plan_lineage_digest: &report.promotion_plan_lineage_digest,
        readiness_id: &report.readiness_id,
        artifact_identity_report_id: &report.artifact_identity_report_id,
        transform_id: &report.transform_id,
        target_execution_lineage_id: report.target_execution_lineage_id.as_deref(),
        wasm_store_identity_report_id: report.wasm_store_identity_report_id.as_deref(),
        wasm_store_identity_report_digest: report.wasm_store_identity_report_digest.as_deref(),
        wasm_store_catalog_verification_id: report.wasm_store_catalog_verification_id.as_deref(),
        wasm_store_catalog_verification_digest: report
            .wasm_store_catalog_verification_digest
            .as_deref(),
        materialization_identity_report_id: report.materialization_identity_report_id.as_deref(),
        materialization_identity_report_digest: report
            .materialization_identity_report_digest
            .as_deref(),
        execution_attempted: report.execution_attempted,
        roles: &report.roles,
        blockers: &report.blockers,
    })
}

pub(in crate::deployment_truth::promotion) fn artifact_promotion_execution_receipt_digest(
    receipt: &ArtifactPromotionExecutionReceiptV1,
) -> String {
    stable_json_sha256_hex(&ArtifactPromotionExecutionReceiptDigestInput {
        schema_version: receipt.schema_version,
        receipt_id: &receipt.receipt_id,
        artifact_promotion_plan_id: &receipt.artifact_promotion_plan_id,
        artifact_promotion_plan_digest: &receipt.artifact_promotion_plan_digest,
        provenance_report_id: &receipt.provenance_report_id,
        provenance_report_digest: &receipt.provenance_report_digest,
        provenance_status: receipt.provenance_status,
        promoted_plan_id: &receipt.promoted_plan_id,
        promotion_plan_lineage_digest: &receipt.promotion_plan_lineage_digest,
        operation_id: &receipt.operation_id,
        operation_status: receipt.operation_status,
        command_result: &receipt.command_result,
        started_at: &receipt.started_at,
        finished_at: receipt.finished_at.as_deref(),
        deployment_receipt: &receipt.deployment_receipt,
        roles: &receipt.roles,
    })
}
