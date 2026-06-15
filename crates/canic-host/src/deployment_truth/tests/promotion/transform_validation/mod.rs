use super::super::*;

#[test]
fn promotion_plan_transform_validation_accepts_generated_transform() {
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    };
    let transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("transform should be produced");

    validate_promotion_plan_transform(&transform).expect("generated transform should validate");
}

#[test]
fn promotion_plan_transform_validation_rejects_schema_drift() {
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    };
    let mut transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("transform should be produced");
    transform.schema_version += 1;

    let err = validate_promotion_plan_transform(&transform).expect_err("schema drift should fail");
    std::assert_matches!(
        err,
        PromotionPlanTransformError::SchemaVersionMismatch { .. }
    );
}

#[test]
fn promotion_plan_transform_validation_rejects_plan_id_mismatch() {
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    };
    let mut transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("transform should be produced");
    transform.promoted_plan.plan_id = "different-plan".to_string();

    let err =
        validate_promotion_plan_transform(&transform).expect_err("plan id mismatch should fail");
    std::assert_matches!(
        err,
        PromotionPlanTransformError::PromotedPlanIdMismatch { .. }
    );
}

#[test]
fn promotion_plan_transform_validation_rejects_duplicate_roles() {
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    };
    let mut transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("transform should be produced");
    transform.roles.push(transform.roles[0].clone());

    let err =
        validate_promotion_plan_transform(&transform).expect_err("duplicate role should fail");
    std::assert_matches!(
        err,
        PromotionPlanTransformError::DuplicateRole { role } if role == "root"
    );
}

#[test]
fn promotion_plan_transform_validation_rejects_missing_promoted_role() {
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    };
    let mut transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("transform should be produced");
    transform.promoted_plan.role_artifacts.clear();

    let err = validate_promotion_plan_transform(&transform)
        .expect_err("missing promoted role should fail");
    std::assert_matches!(
        err,
        PromotionPlanTransformError::PromotedRoleMissing { role } if role == "root"
    );
}

#[test]
fn promotion_plan_transform_validation_rejects_stale_lineage_digest() {
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    };
    let mut transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("transform should be produced");
    transform.promotion_plan_lineage_digest = sample_sha256("9");

    let err = validate_promotion_plan_transform(&transform)
        .expect_err("stale lineage digest should fail");
    std::assert_matches!(
        err,
        PromotionPlanTransformError::RoleStateMismatch {
            role,
            field: "promotion_plan_lineage_digest"
        } if role == "promotion_plan_lineage"
    );
}

#[test]
fn promotion_plan_lineage_digest_changes_when_materialization_link_changes() {
    let mut target_plan = sample_promotion_target_plan();
    target_plan.role_artifacts[0].wasm_sha256 = Some(sample_sha256("5"));
    target_plan.role_artifacts[0].wasm_gz_sha256 = Some(sample_sha256("6"));
    target_plan.role_artifacts[0].installed_module_hash = Some(sample_sha256("7"));
    target_plan.role_artifacts[0].candid_sha256 = Some(sample_sha256("8"));
    let request = PromotionPlanTransformWithMaterializationRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan,
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SourceBuild,
        )],
        materialization_evidence: vec![sample_build_materialization_evidence()],
    };
    let transform = promoted_deployment_plan_transform_from_inputs_with_materialization(&request)
        .expect("source-build transform should link evidence");
    let mut changed_roles = transform.roles.clone();
    changed_roles[0]
        .source_build_materialization
        .as_mut()
        .expect("materialization link should exist")
        .evidence_id = "different-evidence".to_string();

    let changed_digest = promotion_plan_lineage_digest(
        &transform.target_plan_id,
        &transform.promoted_plan_id,
        &transform.promoted_plan,
        &changed_roles,
    );

    assert_ne!(changed_digest, transform.promotion_plan_lineage_digest);
}

#[test]
fn promotion_plan_transform_validation_rejects_stale_after_summary() {
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    };
    let mut transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("transform should be produced");
    transform.roles[0].wasm_gz_sha256_after = Some(sample_sha256("f"));

    let err = validate_promotion_plan_transform(&transform).expect_err("stale summary should fail");
    std::assert_matches!(
        err,
        PromotionPlanTransformError::RoleStateMismatch {
            role,
            field: "wasm_gz_sha256_after"
        } if role == "root"
    );
}

#[test]
fn promotion_plan_transform_validation_rejects_stale_change_flag() {
    let mut target_plan = sample_promotion_target_plan();
    target_plan.role_artifacts[0].wasm_gz_sha256 = Some(sample_sha256("f"));
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    input.require_byte_identical_wasm = false;
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan,
        inputs: vec![input],
    };
    let mut transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("transform should be produced");
    transform.roles[0].artifact_identity_changed = false;

    let err = validate_promotion_plan_transform(&transform).expect_err("stale flag should fail");
    std::assert_matches!(
        err,
        PromotionPlanTransformError::RoleStateMismatch {
            role,
            field: "artifact_identity_changed"
        } if role == "root"
    );
}
