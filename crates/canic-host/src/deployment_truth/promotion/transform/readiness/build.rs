use super::super::super::{digest::promotion_readiness_digest, policy};
use super::findings::collect_role_findings;
use crate::deployment_truth::{
    DEPLOYMENT_TRUTH_SCHEMA_VERSION, DeploymentPlanV1, PromotionReadinessStatusV1,
    PromotionReadinessV1, RoleArtifactV1, RolePromotionInputV1, RolePromotionPolicyV1,
    RolePromotionReadinessV1,
};

#[must_use]
pub fn promotion_readiness_from_inputs(
    readiness_id: impl Into<String>,
    target_plan: &DeploymentPlanV1,
    inputs: &[RolePromotionInputV1],
) -> PromotionReadinessV1 {
    let mut roles = Vec::with_capacity(inputs.len());
    let mut blockers = Vec::new();
    let mut warnings = Vec::new();

    for input in inputs {
        let target_artifact = target_plan
            .role_artifacts
            .iter()
            .find(|artifact| artifact.role == input.role);
        let Some(target_artifact) = target_artifact else {
            blockers.push(super::super::super::promotion_finding(
                "promotion_target_role_missing",
                format!("target plan does not contain role {}", input.role),
                crate::deployment_truth::SafetySeverityV1::HardFailure,
                &input.role,
            ));
            continue;
        };

        let role_readiness = role_promotion_readiness(input, target_artifact);
        collect_role_findings(input, &role_readiness, &mut blockers, &mut warnings);
        roles.push(role_readiness);
    }

    let status = if blockers.is_empty() {
        PromotionReadinessStatusV1::Ready
    } else {
        PromotionReadinessStatusV1::Blocked
    };

    let mut readiness = PromotionReadinessV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        readiness_id: readiness_id.into(),
        promotion_readiness_digest: String::new(),
        target_plan_id: target_plan.plan_id.clone(),
        status,
        roles,
        blockers,
        warnings,
    };
    readiness.promotion_readiness_digest = promotion_readiness_digest(&readiness);
    readiness
}

#[must_use]
pub fn promotion_readiness_from_inputs_with_policy(
    readiness_id: impl Into<String>,
    target_plan: &DeploymentPlanV1,
    inputs: &[RolePromotionInputV1],
    policies: &[RolePromotionPolicyV1],
) -> PromotionReadinessV1 {
    let readiness_id = readiness_id.into();
    let policy_check = policy::promotion_policy_check_from_inputs(
        format!("{readiness_id}:policy"),
        inputs,
        policies,
    );
    let mut readiness = promotion_readiness_from_inputs(readiness_id, target_plan, inputs);
    readiness.blockers.extend(policy_check.blockers);
    readiness.status = if readiness.blockers.is_empty() {
        PromotionReadinessStatusV1::Ready
    } else {
        PromotionReadinessStatusV1::Blocked
    };
    readiness.promotion_readiness_digest = promotion_readiness_digest(&readiness);
    readiness
}

fn role_promotion_readiness(
    input: &RolePromotionInputV1,
    target_artifact: &RoleArtifactV1,
) -> RolePromotionReadinessV1 {
    let source_wasm_sha256 = input.source.expected_wasm_sha256.clone();
    let source_wasm_gz_sha256 = input.source.expected_wasm_gz_sha256.clone();
    let target_wasm_sha256 = target_artifact.wasm_sha256.clone();
    let target_wasm_gz_sha256 = target_artifact.wasm_gz_sha256.clone();
    let byte_identical_wasm =
        matching_optional_digest(source_wasm_sha256.as_ref(), target_wasm_sha256.as_ref()).or_else(
            || {
                matching_optional_digest(
                    source_wasm_gz_sha256.as_ref(),
                    target_wasm_gz_sha256.as_ref(),
                )
            },
        );
    let embedded_config_identical = matching_optional_digest(
        input
            .source
            .expected_canonical_embedded_config_sha256
            .as_ref(),
        target_artifact.canonical_embedded_config_sha256.as_ref(),
    );

    RolePromotionReadinessV1 {
        role: input.role.clone(),
        promotion_level: input.promotion_level,
        source_kind: input.source.kind,
        source_locator: input.source.locator.clone(),
        source_wasm_sha256,
        source_wasm_gz_sha256,
        target_wasm_sha256,
        target_wasm_gz_sha256,
        source_canonical_embedded_config_sha256: input
            .source
            .expected_canonical_embedded_config_sha256
            .clone(),
        target_canonical_embedded_config_sha256: target_artifact
            .canonical_embedded_config_sha256
            .clone(),
        byte_identical_wasm,
        embedded_config_identical,
        target_store_has_artifact: input.target_store_has_artifact,
        restage_required: input.target_store_has_artifact == Some(false),
    }
}

fn matching_optional_digest(left: Option<&String>, right: Option<&String>) -> Option<bool> {
    match (left.map(String::as_str), right.map(String::as_str)) {
        (Some(left), Some(right)) => Some(left == right),
        _ => None,
    }
}
