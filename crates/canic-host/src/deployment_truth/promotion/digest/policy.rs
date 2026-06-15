use crate::deployment_truth::{
    PromotionPolicyCheckV1, PromotionReadinessStatusV1, RolePromotionPolicyDecisionV1,
    SafetyFindingV1, stable_json_sha256_hex,
};
use serde::Serialize;

#[derive(Serialize)]
struct PromotionPolicyCheckDigestInput<'a> {
    schema_version: u32,
    check_id: &'a str,
    status: PromotionReadinessStatusV1,
    roles: &'a [RolePromotionPolicyDecisionV1],
    blockers: &'a [SafetyFindingV1],
}

pub(in crate::deployment_truth::promotion) fn promotion_policy_check_digest(
    check: &PromotionPolicyCheckV1,
) -> String {
    stable_json_sha256_hex(&PromotionPolicyCheckDigestInput {
        schema_version: check.schema_version,
        check_id: &check.check_id,
        status: check.status,
        roles: &check.roles,
        blockers: &check.blockers,
    })
}
