use crate::deployment_truth::{
    PromotionReadinessStatusV1, PromotionReadinessV1, RolePromotionReadinessV1, SafetyFindingV1,
    stable_json_sha256_hex,
};
use serde::Serialize;

#[derive(Serialize)]
struct PromotionReadinessDigestInput<'a> {
    schema_version: u32,
    readiness_id: &'a str,
    target_plan_id: &'a str,
    status: PromotionReadinessStatusV1,
    roles: &'a [RolePromotionReadinessV1],
    blockers: &'a [SafetyFindingV1],
    warnings: &'a [SafetyFindingV1],
}

pub(in crate::deployment_truth::promotion) fn promotion_readiness_digest(
    readiness: &PromotionReadinessV1,
) -> String {
    stable_json_sha256_hex(&PromotionReadinessDigestInput {
        schema_version: readiness.schema_version,
        readiness_id: &readiness.readiness_id,
        target_plan_id: &readiness.target_plan_id,
        status: readiness.status,
        roles: &readiness.roles,
        blockers: &readiness.blockers,
        warnings: &readiness.warnings,
    })
}
