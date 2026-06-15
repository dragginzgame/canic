use super::*;

#[test]
fn promoted_deployment_plan_leaves_source_build_materialization_to_target_plan() {
    let mut target_plan = sample_promotion_target_plan();
    target_plan.role_artifacts[0].wasm_gz_sha256 = Some(sample_sha256("f"));
    target_plan.role_artifacts[0].canonical_embedded_config_sha256 = Some(sample_sha256("1"));
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SourceBuild);
    input.source.expected_canonical_embedded_config_sha256 = Some(sample_sha256("c"));
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: target_plan.clone(),
        inputs: vec![input],
    };

    let promoted = promoted_deployment_plan_from_inputs(&request)
        .expect("source-build plan should be produced");

    assert_eq!(promoted.plan_id, "promoted-plan-1");
    assert_eq!(
        promoted.role_artifacts[0].wasm_gz_sha256,
        target_plan.role_artifacts[0].wasm_gz_sha256
    );
    assert_eq!(
        promoted.role_artifacts[0].canonical_embedded_config_sha256,
        target_plan.role_artifacts[0].canonical_embedded_config_sha256
    );
}

#[test]
fn promoted_deployment_plan_transform_marks_source_build_target_materialization_preserved() {
    let mut target_plan = sample_promotion_target_plan();
    target_plan.role_artifacts[0].wasm_gz_sha256 = Some(sample_sha256("f"));
    target_plan.role_artifacts[0].canonical_embedded_config_sha256 = Some(sample_sha256("1"));
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SourceBuild);
    input.source.expected_canonical_embedded_config_sha256 = Some(sample_sha256("c"));
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan,
        inputs: vec![input],
    };

    let transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("source-build transform should be produced");

    let role = &transform.roles[0];
    assert_eq!(role.promotion_level, PromotionArtifactLevelV1::SourceBuild);
    assert_eq!(role.wasm_gz_sha256_before, Some(sample_sha256("f")));
    assert_eq!(role.wasm_gz_sha256_after, Some(sample_sha256("f")));
    assert_eq!(
        role.canonical_embedded_config_sha256_before,
        Some(sample_sha256("1"))
    );
    assert_eq!(
        role.canonical_embedded_config_sha256_after,
        Some(sample_sha256("1"))
    );
    assert!(!role.artifact_identity_changed);
    assert!(!role.embedded_config_changed);
    assert!(role.target_materialization_preserved);
}

#[test]
fn promoted_deployment_plan_transform_links_source_build_materialization_evidence() {
    let mut target_plan = sample_promotion_target_plan();
    target_plan.role_artifacts[0].wasm_sha256 = Some(sample_sha256("5"));
    target_plan.role_artifacts[0].wasm_gz_sha256 = Some(sample_sha256("6"));
    target_plan.role_artifacts[0].installed_module_hash = Some(sample_sha256("7"));
    target_plan.role_artifacts[0].candid_sha256 = Some(sample_sha256("8"));
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SourceBuild);
    input.source.expected_canonical_embedded_config_sha256 = target_plan.role_artifacts[0]
        .canonical_embedded_config_sha256
        .clone();
    let request = PromotionPlanTransformWithMaterializationRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan,
        inputs: vec![input],
        materialization_evidence: vec![sample_build_materialization_evidence()],
    };

    let transform = promoted_deployment_plan_transform_from_inputs_with_materialization(&request)
        .expect("source-build transform should link materialization evidence");

    let link = transform.roles[0]
        .source_build_materialization
        .as_ref()
        .expect("source-build role should carry materialization link");
    let expected_input_digest =
        build_materialization_input_digest(&sample_build_materialization_input());
    assert_eq!(link.role, "root");
    assert_eq!(link.evidence_id, "materialization-evidence-1");
    assert_eq!(link.materialization_evidence_digest.len(), 64);
    assert_eq!(link.materialization_input_digest, expected_input_digest);
    assert_eq!(link.wasm_gz_sha256, sample_sha256("6"));
    validate_promotion_plan_transform(&transform)
        .expect("materialization-linked transform should validate");
}

#[test]
fn promoted_deployment_plan_transform_requires_source_build_materialization_evidence() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SourceBuild);
    let request = PromotionPlanTransformWithMaterializationRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![input],
        materialization_evidence: Vec::new(),
    };

    let err = promoted_deployment_plan_transform_from_inputs_with_materialization(&request)
        .expect_err("source-build transform should require materialization evidence");

    std::assert_matches!(
        err,
        PromotionPlanTransformError::MaterializationRoleMissing { role } if role == "root"
    );
}

#[test]
fn promoted_deployment_plan_transform_rejects_duplicate_materialization_evidence() {
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
        materialization_evidence: vec![
            sample_build_materialization_evidence(),
            sample_build_materialization_evidence(),
        ],
    };

    let err = promoted_deployment_plan_transform_from_inputs_with_materialization(&request)
        .expect_err("duplicate materialization evidence should fail");

    std::assert_matches!(
        err,
        PromotionPlanTransformError::DuplicateMaterializationRole { role } if role == "root"
    );
}

#[test]
fn promotion_plan_transform_text_reports_source_build_materialization_link() {
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

    let text = promotion_plan_transform_text(&transform);
    let expected_input_digest =
        build_materialization_input_digest(&sample_build_materialization_input());

    assert!(text.contains("materialization_evidence_id: materialization-evidence-1"));
    assert!(text.contains(&format!(
        "materialization_input_digest: {expected_input_digest}"
    )));
    assert!(text.contains(
        "materialized_wasm_gz_sha256: 6666666666666666666666666666666666666666666666666666666666666666"
    ));
}
