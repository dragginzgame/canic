use super::*;

#[test]
fn promotion_target_execution_lineage_round_trips_through_json() {
    let transform = sample_promotion_transform();
    let preflight = sample_execution_preflight_for_plan("promoted-plan-1");

    let lineage = promotion_target_execution_lineage(PromotionTargetExecutionLineageRequest {
        lineage_id: "target-execution-lineage-1".to_string(),
        generated_at: "2026-05-25T00:00:00Z".to_string(),
        transform,
        execution_preflight: preflight,
    })
    .expect("target execution lineage should be produced");

    assert_json_round_trip(&lineage);
    let encoded = serde_json::to_value(&lineage).expect("lineage should encode");
    assert_object_keys(
        &encoded,
        &[
            "schema_version",
            "lineage_id",
            "generated_at",
            "target_execution_lineage_digest",
            "transform",
            "execution_preflight",
            "execution_attempted",
        ],
    );
    assert_eq!(encoded["lineage_id"], "target-execution-lineage-1");
    assert_eq!(encoded["execution_attempted"], false);
}

#[test]
fn promotion_target_execution_lineage_validation_accepts_generated_lineage() {
    let transform = sample_promotion_transform();
    let preflight = sample_execution_preflight_for_plan("promoted-plan-1");

    let lineage = promotion_target_execution_lineage(PromotionTargetExecutionLineageRequest {
        lineage_id: "target-execution-lineage-1".to_string(),
        generated_at: "2026-05-25T00:00:00Z".to_string(),
        transform,
        execution_preflight: preflight,
    })
    .expect("target execution lineage should be produced");

    validate_promotion_target_execution_lineage(&lineage)
        .expect("generated lineage should validate");
}

#[test]
fn promotion_target_execution_lineage_rejects_preflight_for_other_plan() {
    let transform = sample_promotion_transform();
    let preflight = sample_execution_preflight_for_plan("other-promoted-plan");

    let err = promotion_target_execution_lineage(PromotionTargetExecutionLineageRequest {
        lineage_id: "target-execution-lineage-1".to_string(),
        generated_at: "2026-05-25T00:00:00Z".to_string(),
        transform,
        execution_preflight: preflight,
    })
    .expect_err("preflight for another plan should fail");

    std::assert_matches!(
        err,
        PromotionTargetExecutionLineageError::LinkageMismatch {
            field: "execution_preflight.plan_id"
        }
    );
}

#[test]
fn promotion_target_execution_lineage_rejects_execution_claim() {
    let transform = sample_promotion_transform();
    let preflight = sample_execution_preflight_for_plan("promoted-plan-1");
    let mut lineage = promotion_target_execution_lineage(PromotionTargetExecutionLineageRequest {
        lineage_id: "target-execution-lineage-1".to_string(),
        generated_at: "2026-05-25T00:00:00Z".to_string(),
        transform,
        execution_preflight: preflight,
    })
    .expect("target execution lineage should be produced");
    lineage.execution_attempted = true;

    let err = validate_promotion_target_execution_lineage(&lineage)
        .expect_err("execution claim should fail");

    std::assert_matches!(
        err,
        PromotionTargetExecutionLineageError::ExecutionAttempted
    );
}

#[test]
fn promotion_target_execution_lineage_rejects_stale_digest() {
    let transform = sample_promotion_transform();
    let preflight = sample_execution_preflight_for_plan("promoted-plan-1");
    let mut lineage = promotion_target_execution_lineage(PromotionTargetExecutionLineageRequest {
        lineage_id: "target-execution-lineage-1".to_string(),
        generated_at: "2026-05-25T00:00:00Z".to_string(),
        transform,
        execution_preflight: preflight,
    })
    .expect("target execution lineage should be produced");
    lineage.target_execution_lineage_digest = sample_sha256("9");

    let err = validate_promotion_target_execution_lineage(&lineage)
        .expect_err("stale lineage digest should fail");

    std::assert_matches!(
        err,
        PromotionTargetExecutionLineageError::LinkageMismatch {
            field: "target_execution_lineage_digest"
        }
    );
}

#[test]
fn promotion_target_execution_lineage_text_reports_passive_boundary() {
    let transform = sample_promotion_transform();
    let preflight = sample_execution_preflight_for_plan("promoted-plan-1");
    let lineage = promotion_target_execution_lineage(PromotionTargetExecutionLineageRequest {
        lineage_id: "target-execution-lineage-1".to_string(),
        generated_at: "2026-05-25T00:00:00Z".to_string(),
        transform,
        execution_preflight: preflight,
    })
    .expect("target execution lineage should be produced");

    let text = promotion_target_execution_lineage_text(&lineage);

    assert!(text.contains("Promotion target execution lineage"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("execution: none"));
    assert!(text.contains("execution_attempted: false"));
    assert!(text.contains("lineage_id: target-execution-lineage-1"));
    assert!(text.contains("target_execution_lineage_digest: "));
    assert!(text.contains("transform_id: promotion-transform:promoted-plan-1"));
    assert!(text.contains("preflight_plan_id: promoted-plan-1"));
    assert!(text.contains("  Deployment execution preflight"));
}
