use super::*;

#[test]
fn artifact_promotion_plan_round_trips_through_json() {
    let plan = sample_artifact_promotion_plan();

    assert_json_round_trip(&plan);
    let encoded = serde_json::to_value(&plan).expect("promotion plan should encode");
    assert_object_keys(
        &encoded,
        &[
            "schema_version",
            "plan_id",
            "artifact_promotion_plan_digest",
            "generated_at",
            "status",
            "target_plan_id",
            "promoted_plan_id",
            "promotion_plan_lineage_digest",
            "readiness",
            "artifact_identity_report",
            "transform",
            "target_execution_lineage",
            "blockers",
        ],
    );
    assert_eq!(encoded["plan_id"], "artifact-promotion-plan-1");
    assert_eq!(encoded["status"], "Ready");
    assert!(
        encoded["artifact_promotion_plan_digest"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64)
    );
}

#[test]
fn artifact_promotion_plan_validation_accepts_generated_plan() {
    let plan = sample_artifact_promotion_plan();

    validate_artifact_promotion_plan(&plan).expect("generated promotion plan should validate");
}

#[test]
fn artifact_promotion_plan_validation_rejects_status_blocker_mismatch() {
    let mut plan = sample_artifact_promotion_plan();
    plan.blockers.push(SafetyFindingV1 {
        code: "promotion_blocker".to_string(),
        message: "blocked".to_string(),
        severity: SafetySeverityV1::HardFailure,
        subject: Some("root".to_string()),
    });

    let err =
        validate_artifact_promotion_plan(&plan).expect_err("ready plan with blockers should fail");

    std::assert_matches!(
        err,
        ArtifactPromotionPlanError::StatusBlockerMismatch { .. }
    );
}

#[test]
fn artifact_promotion_plan_validation_rejects_stale_lineage_copy() {
    let mut plan = sample_artifact_promotion_plan();
    plan.promotion_plan_lineage_digest = sample_sha256("9");

    let err =
        validate_artifact_promotion_plan(&plan).expect_err("stale plan lineage copy should fail");

    std::assert_matches!(
        err,
        ArtifactPromotionPlanError::LinkageMismatch {
            field: "promotion_plan_lineage_digest"
        }
    );
}

#[test]
fn artifact_promotion_plan_validation_rejects_stale_digest() {
    let mut plan = sample_artifact_promotion_plan();
    plan.artifact_promotion_plan_digest = sample_sha256("9");

    let err = validate_artifact_promotion_plan(&plan).expect_err("stale plan digest should fail");

    std::assert_matches!(
        err,
        ArtifactPromotionPlanError::LinkageMismatch {
            field: "artifact_promotion_plan_digest"
        }
    );
}

#[test]
fn artifact_promotion_plan_validation_rejects_mismatched_target_lineage() {
    let mut plan = sample_artifact_promotion_plan();
    let mut lineage = plan
        .target_execution_lineage
        .clone()
        .expect("sample plan should carry target lineage");
    lineage.transform.transform_id = "different-transform".to_string();
    plan.target_execution_lineage = Some(lineage);

    let err = validate_artifact_promotion_plan(&plan)
        .expect_err("target lineage with different transform should fail");

    std::assert_matches!(
        err,
        ArtifactPromotionPlanError::LinkageMismatch {
            field: "target_execution_lineage.transform"
        }
    );
}

#[test]
fn artifact_promotion_plan_for_check_accepts_matching_promoted_plan_check() {
    let plan = sample_artifact_promotion_plan();
    let check = sample_check(
        plan.transform.promoted_plan.clone(),
        sample_matching_inventory(),
    );

    validate_artifact_promotion_plan_for_check(&plan, &check)
        .expect("promotion plan should validate against target check");
}

#[test]
fn artifact_promotion_plan_for_check_rejects_other_target_plan() {
    let plan = sample_artifact_promotion_plan();
    let mut other_plan = sample_promotion_target_plan();
    other_plan.plan_id = "other-target-plan".to_string();
    let check = sample_check(other_plan, sample_matching_inventory());

    let err = validate_artifact_promotion_plan_for_check(&plan, &check)
        .expect_err("target check for another plan should fail");

    std::assert_matches!(
        err,
        ArtifactPromotionPlanError::LinkageMismatch {
            field: "target_check.plan"
        }
    );
}

#[test]
fn artifact_promotion_plan_for_check_rejects_missing_target_execution_lineage() {
    let sample = sample_artifact_promotion_plan();
    let promoted_plan = sample.transform.promoted_plan.clone();
    let plan = artifact_promotion_plan(ArtifactPromotionPlanRequest {
        plan_id: sample.plan_id,
        generated_at: sample.generated_at,
        readiness: sample.readiness,
        artifact_identity_report: sample.artifact_identity_report,
        transform: sample.transform,
        target_execution_lineage: None,
    })
    .expect("sample plan without lineage should still validate");
    let check = sample_check(promoted_plan, sample_matching_inventory());

    let err = validate_artifact_promotion_plan_for_check(&plan, &check)
        .expect_err("target check validation should require execution lineage");

    std::assert_matches!(
        err,
        ArtifactPromotionPlanError::MissingTargetExecutionLineage
    );
}

#[test]
fn artifact_promotion_plan_for_check_rejects_preflight_check_mismatch() {
    let plan = sample_artifact_promotion_plan();
    let mut check = sample_check(
        plan.transform.promoted_plan.clone(),
        sample_matching_inventory(),
    );
    check.report.report_id = "other-report".to_string();

    let err = validate_artifact_promotion_plan_for_check(&plan, &check)
        .expect_err("preflight mismatch should fail");

    std::assert_matches!(
        err,
        ArtifactPromotionPlanError::TargetCheck(
            DeploymentExecutionPreflightError::SourceCheckMismatch {
                field: "safety_report_id",
                ..
            }
        )
    );
}

#[test]
fn artifact_promotion_plan_text_reports_passive_summary() {
    let plan = sample_artifact_promotion_plan();

    let text = artifact_promotion_plan_text(&plan);

    assert!(text.contains("Artifact promotion plan"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("execution: none"));
    assert!(text.contains("plan_id: artifact-promotion-plan-1"));
    assert!(text.contains("artifact_promotion_plan_digest:"));
    assert!(text.contains("status: Ready"));
    assert!(text.contains("target_execution_lineage: target-execution-lineage-1"));
    assert!(text.contains("readiness_roles: 1"));
    assert!(text.contains("artifact_identity_roles: 1"));
    assert!(text.contains("transform_roles: 1"));
    assert!(text.contains("  Promotion readiness report"));
    assert!(text.contains("  Promotion artifact identity report"));
    assert!(text.contains("  Promotion plan transform"));
}
