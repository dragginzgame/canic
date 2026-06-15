use super::super::super::{
    digest::promotion_plan_lineage_digest,
    ensure::ensure_transform_field,
    error::PromotionPlanTransformError,
    identity::{artifact_identity_changed, role_materialization_identity_matches},
    materialization,
    request::{PromotionPlanTransformRequest, PromotionPlanTransformWithMaterializationRequest},
};
use super::super::{
    readiness::{promotion_readiness_from_inputs, validate_promotion_readiness},
    source::apply_promotion_input_to_role_artifact,
};
use super::validation::validate_promotion_plan_transform;
use crate::deployment_truth::{
    DEPLOYMENT_TRUTH_SCHEMA_VERSION, DeploymentPlanV1, PromotionArtifactLevelV1,
    PromotionPlanTransformV1, PromotionReadinessStatusV1, RoleArtifactV1, RolePromotionInputV1,
    RolePromotionPlanTransformV1,
};

pub fn promoted_deployment_plan_from_inputs(
    request: &PromotionPlanTransformRequest,
) -> Result<DeploymentPlanV1, PromotionPlanTransformError> {
    Ok(promoted_deployment_plan_transform_from_inputs(request)?.promoted_plan)
}

pub fn promoted_deployment_plan_transform_from_inputs(
    request: &PromotionPlanTransformRequest,
) -> Result<PromotionPlanTransformV1, PromotionPlanTransformError> {
    ensure_transform_field("promoted_plan_id", &request.promoted_plan_id)?;
    let readiness = promotion_readiness_from_inputs(
        &request.promoted_plan_id,
        &request.target_plan,
        &request.inputs,
    );
    validate_promotion_readiness(&readiness)?;
    if readiness.status == PromotionReadinessStatusV1::Blocked {
        return Err(PromotionPlanTransformError::ReadinessBlocked {
            blocker_count: readiness.blockers.len(),
        });
    }

    let mut promoted_plan = request.target_plan.clone();
    promoted_plan.plan_id.clone_from(&request.promoted_plan_id);
    for input in &request.inputs {
        let Some(role_artifact) = promoted_plan
            .role_artifacts
            .iter_mut()
            .find(|artifact| artifact.role == input.role)
        else {
            return Err(PromotionPlanTransformError::TargetRoleMissing {
                role: input.role.clone(),
            });
        };
        apply_promotion_input_to_role_artifact(role_artifact, input);
    }
    let transform =
        promotion_plan_transform_from_parts(&request.target_plan, promoted_plan, &request.inputs);
    validate_promotion_plan_transform(&transform)?;
    Ok(transform)
}

pub fn promoted_deployment_plan_transform_from_inputs_with_materialization(
    request: &PromotionPlanTransformWithMaterializationRequest,
) -> Result<PromotionPlanTransformV1, PromotionPlanTransformError> {
    let base_request = PromotionPlanTransformRequest {
        promoted_plan_id: request.promoted_plan_id.clone(),
        target_plan: request.target_plan.clone(),
        inputs: request.inputs.clone(),
    };
    let mut transform = promoted_deployment_plan_transform_from_inputs(&base_request)?;
    materialization::attach_source_build_materialization(
        &mut transform,
        &request.inputs,
        &request.materialization_evidence,
    )?;
    refresh_promotion_plan_lineage_digest(&mut transform);
    validate_promotion_plan_transform(&transform)?;
    Ok(transform)
}

fn promotion_plan_transform_from_parts(
    target_plan: &DeploymentPlanV1,
    promoted_plan: DeploymentPlanV1,
    inputs: &[RolePromotionInputV1],
) -> PromotionPlanTransformV1 {
    let roles = inputs
        .iter()
        .filter_map(|input| {
            let before = target_plan
                .role_artifacts
                .iter()
                .find(|artifact| artifact.role == input.role)?;
            let after = promoted_plan
                .role_artifacts
                .iter()
                .find(|artifact| artifact.role == input.role)?;
            Some(role_plan_transform(input, before, after))
        })
        .collect::<Vec<_>>();
    let promotion_plan_lineage_digest = promotion_plan_lineage_digest(
        &target_plan.plan_id,
        &promoted_plan.plan_id,
        &promoted_plan,
        &roles,
    );

    PromotionPlanTransformV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        transform_id: format!("promotion-transform:{}", promoted_plan.plan_id),
        target_plan_id: target_plan.plan_id.clone(),
        promoted_plan_id: promoted_plan.plan_id.clone(),
        promotion_plan_lineage_digest,
        promoted_plan,
        roles,
    }
}

fn role_plan_transform(
    input: &RolePromotionInputV1,
    before: &RoleArtifactV1,
    after: &RoleArtifactV1,
) -> RolePromotionPlanTransformV1 {
    RolePromotionPlanTransformV1 {
        role: input.role.clone(),
        promotion_level: input.promotion_level,
        source_kind: input.source.kind,
        source_locator: input.source.locator.clone(),
        artifact_source_before: before.source,
        artifact_source_after: after.source,
        wasm_sha256_before: before.wasm_sha256.clone(),
        wasm_sha256_after: after.wasm_sha256.clone(),
        wasm_gz_sha256_before: before.wasm_gz_sha256.clone(),
        wasm_gz_sha256_after: after.wasm_gz_sha256.clone(),
        candid_sha256_before: before.candid_sha256.clone(),
        candid_sha256_after: after.candid_sha256.clone(),
        canonical_embedded_config_sha256_before: before.canonical_embedded_config_sha256.clone(),
        canonical_embedded_config_sha256_after: after.canonical_embedded_config_sha256.clone(),
        artifact_identity_changed: artifact_identity_changed(before, after),
        embedded_config_changed: before.canonical_embedded_config_sha256
            != after.canonical_embedded_config_sha256,
        target_materialization_preserved: input.promotion_level
            == PromotionArtifactLevelV1::SourceBuild
            && role_materialization_identity_matches(before, after),
        source_build_materialization: None,
    }
}

fn refresh_promotion_plan_lineage_digest(transform: &mut PromotionPlanTransformV1) {
    transform.promotion_plan_lineage_digest = promotion_plan_lineage_digest(
        &transform.target_plan_id,
        &transform.promoted_plan_id,
        &transform.promoted_plan,
        &transform.roles,
    );
}
