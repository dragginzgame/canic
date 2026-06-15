use super::*;

#[test]
fn promotion_readiness_reports_ready_role_and_restage_warning() {
    let plan = sample_promotion_target_plan();
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    input.target_store_has_artifact = Some(false);

    let readiness = promotion_readiness_from_inputs("promotion-ready-1", &plan, &[input]);

    assert_eq!(readiness.schema_version, DEPLOYMENT_TRUTH_SCHEMA_VERSION);
    assert_eq!(readiness.target_plan_id, plan.plan_id);
    assert_eq!(readiness.status, PromotionReadinessStatusV1::Ready);
    assert!(readiness.blockers.is_empty());
    assert_eq!(readiness.warnings.len(), 1);
    assert_eq!(
        readiness.warnings[0].code,
        "promotion_target_store_restage_required"
    );
    assert_eq!(readiness.roles.len(), 1);
    assert_eq!(readiness.roles[0].byte_identical_wasm, Some(true));
    assert_eq!(readiness.roles[0].embedded_config_identical, Some(true));
    assert!(readiness.roles[0].restage_required);
    validate_promotion_readiness(&readiness).expect("readiness artifact should validate");
}

#[test]
fn promotion_readiness_blocks_sealed_wasm_embedded_config_mismatch() {
    let plan = sample_promotion_target_plan();
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    input.source.expected_canonical_embedded_config_sha256 = Some(sample_sha256("e"));

    let readiness = promotion_readiness_from_inputs("promotion-ready-1", &plan, &[input]);

    assert_eq!(readiness.status, PromotionReadinessStatusV1::Blocked);
    assert!(readiness.blockers.iter().any(|finding| {
        finding.code == "promotion_sealed_wasm_embedded_config_mismatch"
            && finding.subject.as_deref() == Some("root")
    }));
}

#[test]
fn source_build_promotion_allows_target_config_digest_change() {
    let plan = sample_promotion_target_plan();
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SourceBuild);
    input.source.expected_canonical_embedded_config_sha256 = Some(sample_sha256("e"));

    let readiness = promotion_readiness_from_inputs("promotion-ready-1", &plan, &[input]);

    assert_eq!(readiness.status, PromotionReadinessStatusV1::Ready);
    assert!(readiness.blockers.is_empty());
    assert_eq!(readiness.roles[0].embedded_config_identical, Some(false));
    validate_promotion_readiness(&readiness).expect("source-build readiness should validate");
}

#[test]
fn check_promotion_readiness_validates_and_returns_artifact() {
    let request = PromotionReadinessRequest {
        readiness_id: "promotion-ready-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    };

    let readiness = check_promotion_readiness(&request).expect("readiness should be valid");

    assert_eq!(readiness.readiness_id, "promotion-ready-1");
    assert_eq!(readiness.status, PromotionReadinessStatusV1::Ready);
    assert_eq!(readiness.roles.len(), 1);
}

#[test]
fn promotion_readiness_with_policy_blocks_source_build_when_sealed_bytes_are_required() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SourceBuild);
    let policy = sample_role_promotion_policy();

    let readiness = promotion_readiness_from_inputs_with_policy(
        "promotion-ready-1",
        &sample_promotion_target_plan(),
        &[input],
        &[policy],
    );

    assert_eq!(readiness.status, PromotionReadinessStatusV1::Blocked);
    assert!(readiness.blockers.iter().any(|finding| {
        finding.code == "promotion_policy_level_not_allowed"
            || finding.code == "promotion_policy_must_use_sealed_bytes"
    }));
    validate_promotion_readiness(&readiness).expect("policy-blocked readiness should validate");
}

#[test]
fn promotion_readiness_with_policy_accepts_byte_identical_source_build_policy() {
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SourceBuild);
    input.require_byte_identical_wasm = true;
    let mut policy = sample_role_promotion_policy();
    policy.allowed_promotion_levels = vec![PromotionArtifactLevelV1::SourceBuild];
    policy.requirements = vec![PromotionPolicyRequirementV1::ByteIdenticalWasm];

    let readiness = check_promotion_readiness_with_policy(&PromotionReadinessWithPolicyRequest {
        readiness_id: "promotion-ready-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![input],
        policies: vec![policy],
    })
    .expect("source-build policy readiness should validate");

    assert_eq!(readiness.status, PromotionReadinessStatusV1::Ready);
    assert!(readiness.blockers.is_empty());
}

#[test]
fn promotion_readiness_with_policy_reports_missing_role_policy() {
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);

    let readiness = promotion_readiness_from_inputs_with_policy(
        "promotion-ready-1",
        &sample_promotion_target_plan(),
        &[input],
        &[],
    );

    assert_eq!(readiness.status, PromotionReadinessStatusV1::Blocked);
    assert!(readiness.blockers.iter().any(|finding| {
        finding.code == "promotion_policy_missing" && finding.subject.as_deref() == Some("root")
    }));
    validate_promotion_readiness(&readiness).expect("missing-policy readiness should validate");
}

#[test]
fn check_promotion_readiness_rejects_blank_readiness_id() {
    let request = PromotionReadinessRequest {
        readiness_id: " ".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    };

    let err = check_promotion_readiness(&request).expect_err("blank readiness id should fail");
    std::assert_matches!(
        err,
        PromotionReadinessError::MissingRequiredField {
            field: "readiness_id"
        }
    );
}

#[test]
fn promoted_deployment_plan_rejects_blocked_readiness() {
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    input.source.expected_canonical_embedded_config_sha256 = Some(sample_sha256("e"));
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![input],
    };

    let err =
        promoted_deployment_plan_from_inputs(&request).expect_err("blocked readiness should fail");
    std::assert_matches!(
        err,
        PromotionPlanTransformError::ReadinessBlocked { blocker_count: 1 }
    );
}

#[test]
fn promoted_deployment_plan_rejects_blank_plan_id() {
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: " ".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    };

    let err =
        promoted_deployment_plan_from_inputs(&request).expect_err("blank plan id should fail");
    std::assert_matches!(
        err,
        PromotionPlanTransformError::MissingRequiredField {
            field: "promoted_plan_id"
        }
    );
}

#[test]
fn promotion_readiness_blocks_source_role_mismatch() {
    let plan = sample_promotion_target_plan();
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SourceBuild);
    input.source.role = "other".to_string();

    let readiness = promotion_readiness_from_inputs("promotion-ready-1", &plan, &[input]);

    assert_eq!(readiness.status, PromotionReadinessStatusV1::Blocked);
    assert!(readiness.blockers.iter().any(|finding| {
        finding.code == "promotion_source_role_mismatch"
            && finding.subject.as_deref() == Some("root")
    }));
}

#[test]
fn promotion_readiness_blocks_missing_target_role() {
    let plan = sample_plan();
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SourceBuild);
    input.role = "missing".to_string();
    input.source.role = "missing".to_string();

    let readiness = promotion_readiness_from_inputs("promotion-ready-1", &plan, &[input]);

    assert_eq!(readiness.status, PromotionReadinessStatusV1::Blocked);
    assert!(readiness.roles.is_empty());
    assert!(readiness.blockers.iter().any(|finding| {
        finding.code == "promotion_target_role_missing"
            && finding.subject.as_deref() == Some("missing")
    }));
}

#[test]
fn promotion_readiness_blocks_invalid_artifact_source() {
    let plan = sample_promotion_target_plan();
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SourceBuild);
    input.source.expected_wasm_sha256 = None;
    input.source.expected_wasm_gz_sha256 = None;

    let readiness = promotion_readiness_from_inputs("promotion-ready-1", &plan, &[input]);

    assert_eq!(readiness.status, PromotionReadinessStatusV1::Blocked);
    assert!(readiness.blockers.iter().any(|finding| {
        finding.code == "promotion_artifact_source_invalid"
            && finding.subject.as_deref() == Some("root")
    }));
    validate_promotion_readiness(&readiness).expect("blocked readiness artifact should validate");
}
