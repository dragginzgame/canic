use super::*;

#[test]
fn promotion_plan_transform_evidence_validation_accepts_generated_evidence() {
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    };
    let transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("transform should be produced");
    let evidence = promotion_plan_transform_evidence(PromotionPlanTransformEvidenceRequest {
        evidence_id: "promotion-evidence-1".to_string(),
        generated_at: "2026-05-25T00:00:00Z".to_string(),
        transform,
    })
    .expect("evidence should be produced");

    validate_promotion_plan_transform_evidence(&evidence)
        .expect("generated evidence should validate");
}

#[test]
fn promotion_plan_transform_evidence_text_reports_passive_boundary() {
    let mut target_plan = sample_promotion_target_plan();
    target_plan.role_artifacts[0].wasm_gz_sha256 = Some(sample_sha256("f"));
    let mut input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    input.require_byte_identical_wasm = false;
    let transform =
        promoted_deployment_plan_transform_from_inputs(&PromotionPlanTransformRequest {
            promoted_plan_id: "promoted-plan-1".to_string(),
            target_plan,
            inputs: vec![input],
        })
        .expect("transform should be produced");
    let evidence = promotion_plan_transform_evidence(PromotionPlanTransformEvidenceRequest {
        evidence_id: "promotion-evidence-1".to_string(),
        generated_at: "2026-05-25T00:00:00Z".to_string(),
        transform,
    })
    .expect("evidence should be produced");

    let text = promotion_plan_transform_evidence_text(&evidence);

    assert!(text.contains("Promotion plan transform evidence"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("execution: none"));
    assert!(text.contains("evidence_id: promotion-evidence-1"));
    assert!(text.contains("promotion_plan_transform_evidence_digest:"));
    assert!(text.contains("generated_at: 2026-05-25T00:00:00Z"));
    assert!(text.contains("transform_id: promotion-transform:promoted-plan-1"));
    assert!(text.contains("  Promotion plan transform"));
    assert!(text.contains("  mode: passive"));
    assert!(text.contains("  artifact_identity_changed: 1"));
}

#[test]
fn promotion_plan_transform_evidence_validation_rejects_blank_evidence_id() {
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    };
    let transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("transform should be produced");
    let err = promotion_plan_transform_evidence(PromotionPlanTransformEvidenceRequest {
        evidence_id: " ".to_string(),
        generated_at: "2026-05-25T00:00:00Z".to_string(),
        transform,
    })
    .expect_err("blank evidence id should fail");

    std::assert_matches!(
        err,
        PromotionPlanTransformEvidenceError::MissingRequiredField {
            field: "evidence_id"
        }
    );
}

#[test]
fn promotion_plan_transform_evidence_validation_rejects_schema_drift() {
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    };
    let transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("transform should be produced");
    let mut evidence = promotion_plan_transform_evidence(PromotionPlanTransformEvidenceRequest {
        evidence_id: "promotion-evidence-1".to_string(),
        generated_at: "2026-05-25T00:00:00Z".to_string(),
        transform,
    })
    .expect("evidence should be produced");
    evidence.schema_version += 1;

    let err = validate_promotion_plan_transform_evidence(&evidence)
        .expect_err("schema drift should fail");
    std::assert_matches!(
        err,
        PromotionPlanTransformEvidenceError::SchemaVersionMismatch { .. }
    );
}

#[test]
fn promotion_plan_transform_evidence_validation_rejects_stale_digest() {
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    };
    let transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("transform should be produced");
    let mut evidence = promotion_plan_transform_evidence(PromotionPlanTransformEvidenceRequest {
        evidence_id: "promotion-evidence-1".to_string(),
        generated_at: "2026-05-25T00:00:00Z".to_string(),
        transform,
    })
    .expect("evidence should be produced");
    evidence.promotion_plan_transform_evidence_digest = sample_sha256("9");

    let err = validate_promotion_plan_transform_evidence(&evidence)
        .expect_err("stale evidence digest should fail");
    std::assert_matches!(
        err,
        PromotionPlanTransformEvidenceError::LinkageMismatch {
            field: "promotion_plan_transform_evidence_digest"
        }
    );
}

#[test]
fn promotion_plan_transform_evidence_validation_rejects_stale_transform() {
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    };
    let transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("transform should be produced");
    let mut evidence = promotion_plan_transform_evidence(PromotionPlanTransformEvidenceRequest {
        evidence_id: "promotion-evidence-1".to_string(),
        generated_at: "2026-05-25T00:00:00Z".to_string(),
        transform,
    })
    .expect("evidence should be produced");
    evidence.transform.roles[0].artifact_identity_changed = false;

    let err = validate_promotion_plan_transform_evidence(&evidence)
        .expect_err("stale transform should fail");
    std::assert_matches!(
        err,
        PromotionPlanTransformEvidenceError::Transform(
            PromotionPlanTransformError::RoleStateMismatch {
                role,
                field: "artifact_identity_changed"
            }
        ) if role == "root"
    );
}
