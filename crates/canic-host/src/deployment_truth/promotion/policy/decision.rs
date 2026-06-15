use crate::deployment_truth::{
    PromotionArtifactLevelV1, PromotionPolicyClaimV1, PromotionPolicyRequirementV1,
    RolePromotionInputV1, RolePromotionPolicyDecisionV1, RolePromotionPolicyV1, SafetyFindingV1,
    SafetySeverityV1,
};

pub(super) fn role_promotion_policy_decision(
    input: &RolePromotionInputV1,
    policy: &RolePromotionPolicyV1,
) -> RolePromotionPolicyDecisionV1 {
    let level_allowed = policy
        .allowed_promotion_levels
        .contains(&input.promotion_level);
    let claims = promotion_policy_claims_for_input(input);
    let policy_satisfied = level_allowed
        && (!policy
            .requirements
            .contains(&PromotionPolicyRequirementV1::SealedBytes)
            || input.promotion_level == PromotionArtifactLevelV1::SealedWasm)
        && (!policy
            .requirements
            .contains(&PromotionPolicyRequirementV1::ByteIdenticalWasm)
            || claims.contains(&PromotionPolicyClaimV1::ByteIdenticalWasm))
        && (!policy
            .requirements
            .contains(&PromotionPolicyRequirementV1::TargetConfigDigest)
            || claims.contains(&PromotionPolicyClaimV1::TargetConfigDigest));
    RolePromotionPolicyDecisionV1 {
        role: input.role.clone(),
        requested_promotion_level: input.promotion_level,
        allowed_promotion_levels: policy.allowed_promotion_levels.clone(),
        requirements: policy.requirements.clone(),
        claims,
        level_allowed,
        policy_satisfied,
    }
}

fn promotion_policy_claims_for_input(input: &RolePromotionInputV1) -> Vec<PromotionPolicyClaimV1> {
    let mut claims = Vec::with_capacity(2);
    if input.require_byte_identical_wasm {
        claims.push(PromotionPolicyClaimV1::ByteIdenticalWasm);
    }
    if input.require_target_embedded_config {
        claims.push(PromotionPolicyClaimV1::TargetConfigDigest);
    }
    claims
}

pub(super) fn collect_policy_findings(
    decision: &RolePromotionPolicyDecisionV1,
    blockers: &mut Vec<SafetyFindingV1>,
) {
    if !decision.level_allowed {
        blockers.push(super::super::promotion_finding(
            "promotion_policy_level_not_allowed",
            format!(
                "role {} cannot use promotion level {:?}",
                decision.role, decision.requested_promotion_level
            ),
            SafetySeverityV1::HardFailure,
            &decision.role,
        ));
    }
    if decision
        .requirements
        .contains(&PromotionPolicyRequirementV1::SealedBytes)
        && decision.requested_promotion_level != PromotionArtifactLevelV1::SealedWasm
    {
        blockers.push(super::super::promotion_finding(
            "promotion_policy_must_use_sealed_bytes",
            format!("role {} must use sealed bytes", decision.role),
            SafetySeverityV1::HardFailure,
            &decision.role,
        ));
    }
    if decision
        .requirements
        .contains(&PromotionPolicyRequirementV1::ByteIdenticalWasm)
        && !decision
            .claims
            .contains(&PromotionPolicyClaimV1::ByteIdenticalWasm)
    {
        blockers.push(super::super::promotion_finding(
            "promotion_policy_byte_identity_required",
            format!("role {} requires byte-identical wasm", decision.role),
            SafetySeverityV1::HardFailure,
            &decision.role,
        ));
    }
    if decision
        .requirements
        .contains(&PromotionPolicyRequirementV1::TargetConfigDigest)
        && !decision
            .claims
            .contains(&PromotionPolicyClaimV1::TargetConfigDigest)
    {
        blockers.push(super::super::promotion_finding(
            "promotion_policy_target_config_digest_required",
            format!("role {} requires target config digest", decision.role),
            SafetySeverityV1::HardFailure,
            &decision.role,
        ));
    }
}
