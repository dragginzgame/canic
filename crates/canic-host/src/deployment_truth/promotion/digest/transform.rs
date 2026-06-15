use crate::deployment_truth::{
    DeploymentExecutionPreflightStatusV1, DeploymentExecutionPreflightV1,
    DeploymentExecutorBackendV1, DeploymentExecutorCapabilityV1, DeploymentPlanV1,
    PromotionPlanTransformEvidenceV1, PromotionPlanTransformV1, RolePromotionPlanTransformV1,
    stable_json_sha256_hex,
};
use serde::Serialize;

#[derive(Serialize)]
struct PromotionPlanLineageInput<'a> {
    target_plan_id: &'a str,
    promoted_plan_id: &'a str,
    promoted_plan: &'a DeploymentPlanV1,
    roles: &'a [RolePromotionPlanTransformV1],
}

#[derive(Serialize)]
struct PromotionTargetExecutionLineageInput<'a> {
    promotion_plan_lineage_digest: &'a str,
    promoted_plan_id: &'a str,
    preflight_plan_id: &'a str,
    preflight_safety_report_id: &'a str,
    preflight_authority_plan_id: &'a str,
    preflight_backend: &'a DeploymentExecutorBackendV1,
    preflight_status: DeploymentExecutionPreflightStatusV1,
    planned_phases: &'a [String],
    required_capabilities: &'a [DeploymentExecutorCapabilityV1],
    missing_capabilities: &'a [DeploymentExecutorCapabilityV1],
    execution_attempted: bool,
}

#[derive(Serialize)]
struct PromotionPlanTransformEvidenceDigestInput<'a> {
    schema_version: u32,
    evidence_id: &'a str,
    generated_at: &'a str,
    transform: &'a PromotionPlanTransformV1,
}

pub(in crate::deployment_truth::promotion) fn promotion_plan_transform_evidence_digest(
    evidence: &PromotionPlanTransformEvidenceV1,
) -> String {
    stable_json_sha256_hex(&PromotionPlanTransformEvidenceDigestInput {
        schema_version: evidence.schema_version,
        evidence_id: &evidence.evidence_id,
        generated_at: &evidence.generated_at,
        transform: &evidence.transform,
    })
}

#[must_use]
pub fn promotion_plan_lineage_digest(
    target_plan_id: &str,
    promoted_plan_id: &str,
    promoted_plan: &DeploymentPlanV1,
    roles: &[RolePromotionPlanTransformV1],
) -> String {
    stable_json_sha256_hex(&PromotionPlanLineageInput {
        target_plan_id,
        promoted_plan_id,
        promoted_plan,
        roles,
    })
}

#[must_use]
pub fn promotion_target_execution_lineage_digest(
    transform: &PromotionPlanTransformV1,
    preflight: &DeploymentExecutionPreflightV1,
    execution_attempted: bool,
) -> String {
    stable_json_sha256_hex(&PromotionTargetExecutionLineageInput {
        promotion_plan_lineage_digest: &transform.promotion_plan_lineage_digest,
        promoted_plan_id: &transform.promoted_plan_id,
        preflight_plan_id: &preflight.plan_id,
        preflight_safety_report_id: &preflight.safety_report_id,
        preflight_authority_plan_id: &preflight.authority_plan_id,
        preflight_backend: &preflight.backend,
        preflight_status: preflight.status,
        planned_phases: &preflight.planned_phases,
        required_capabilities: &preflight.required_capabilities,
        missing_capabilities: &preflight.missing_capabilities,
        execution_attempted,
    })
}
