use crate::deployment_truth::{
    ArtifactPromotionPlanV1, PromotionArtifactIdentityReportV1, PromotionPlanTransformV1,
    PromotionReadinessStatusV1, PromotionReadinessV1, PromotionTargetExecutionLineageV1,
    SafetyFindingV1, stable_json_sha256_hex,
};
use serde::Serialize;

#[derive(Serialize)]
struct ArtifactPromotionPlanDigestInput<'a> {
    schema_version: u32,
    plan_id: &'a str,
    generated_at: &'a str,
    status: PromotionReadinessStatusV1,
    target_plan_id: &'a str,
    promoted_plan_id: &'a str,
    promotion_plan_lineage_digest: &'a str,
    readiness: &'a PromotionReadinessV1,
    artifact_identity_report: &'a PromotionArtifactIdentityReportV1,
    transform: &'a PromotionPlanTransformV1,
    target_execution_lineage: Option<&'a PromotionTargetExecutionLineageV1>,
    blockers: &'a [SafetyFindingV1],
}

pub(in crate::deployment_truth::promotion) fn artifact_promotion_plan_digest(
    plan: &ArtifactPromotionPlanV1,
) -> String {
    stable_json_sha256_hex(&ArtifactPromotionPlanDigestInput {
        schema_version: plan.schema_version,
        plan_id: &plan.plan_id,
        generated_at: &plan.generated_at,
        status: plan.status,
        target_plan_id: &plan.target_plan_id,
        promoted_plan_id: &plan.promoted_plan_id,
        promotion_plan_lineage_digest: &plan.promotion_plan_lineage_digest,
        readiness: &plan.readiness,
        artifact_identity_report: &plan.artifact_identity_report,
        transform: &plan.transform,
        target_execution_lineage: plan.target_execution_lineage.as_ref(),
        blockers: &plan.blockers,
    })
}
