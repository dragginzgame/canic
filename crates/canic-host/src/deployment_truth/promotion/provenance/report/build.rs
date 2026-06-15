use crate::deployment_truth::{
    ArtifactPromotionProvenanceReportV1, DEPLOYMENT_TRUTH_SCHEMA_VERSION,
    PromotionReadinessStatusV1,
};

use super::super::super::{
    digest::artifact_promotion_provenance_digest, request::ArtifactPromotionProvenanceReportRequest,
};
use super::blockers::artifact_promotion_provenance_blockers;
use super::roles::{
    attach_materialization_provenance, attach_wasm_store_catalog_provenance,
    attach_wasm_store_provenance, role_promotion_provenance_from_transform,
};

pub(super) fn build_artifact_promotion_provenance_report(
    request: ArtifactPromotionProvenanceReportRequest,
) -> ArtifactPromotionProvenanceReportV1 {
    let plan = request.artifact_promotion_plan;
    let mut roles = plan
        .transform
        .roles
        .iter()
        .map(role_promotion_provenance_from_transform)
        .collect::<Vec<_>>();
    attach_wasm_store_provenance(&mut roles, request.wasm_store_identity_report.as_ref());
    attach_wasm_store_catalog_provenance(
        &mut roles,
        request.wasm_store_catalog_verification.as_ref(),
    );
    attach_materialization_provenance(&mut roles, request.materialization_identity_report.as_ref());
    let blockers = artifact_promotion_provenance_blockers(
        &plan,
        request.wasm_store_identity_report.as_ref(),
        request.wasm_store_catalog_verification.as_ref(),
        request.materialization_identity_report.as_ref(),
        &roles,
    );
    let status = if blockers.is_empty() {
        PromotionReadinessStatusV1::Ready
    } else {
        PromotionReadinessStatusV1::Blocked
    };
    let mut report = ArtifactPromotionProvenanceReportV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        report_id: request.report_id,
        status,
        artifact_promotion_plan_id: plan.plan_id,
        artifact_promotion_plan_digest: plan.artifact_promotion_plan_digest,
        target_plan_id: plan.target_plan_id,
        promoted_plan_id: plan.promoted_plan_id,
        promotion_plan_lineage_digest: plan.promotion_plan_lineage_digest,
        provenance_report_digest: String::new(),
        readiness_id: plan.readiness.readiness_id,
        artifact_identity_report_id: plan.artifact_identity_report.report_id,
        transform_id: plan.transform.transform_id,
        target_execution_lineage_id: plan
            .target_execution_lineage
            .map(|lineage| lineage.lineage_id),
        wasm_store_identity_report_id: request
            .wasm_store_identity_report
            .as_ref()
            .map(|report| report.report_id.clone()),
        wasm_store_identity_report_digest: request
            .wasm_store_identity_report
            .map(|report| report.wasm_store_identity_report_digest),
        wasm_store_catalog_verification_id: request
            .wasm_store_catalog_verification
            .as_ref()
            .map(|verification| verification.verification_id.clone()),
        wasm_store_catalog_verification_digest: request
            .wasm_store_catalog_verification
            .map(|verification| verification.wasm_store_catalog_verification_digest),
        materialization_identity_report_id: request
            .materialization_identity_report
            .as_ref()
            .map(|report| report.report_id.clone()),
        materialization_identity_report_digest: request
            .materialization_identity_report
            .map(|report| report.materialization_identity_report_digest),
        execution_attempted: false,
        roles,
        blockers,
    };
    report.provenance_report_digest = artifact_promotion_provenance_digest(&report);
    report
}
