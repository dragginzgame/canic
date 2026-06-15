use super::super::*;

#[test]
fn promotion_readiness_round_trips_through_json() {
    let plan = sample_promotion_target_plan();
    let input = sample_role_promotion_input(PromotionArtifactLevelV1::SealedWasm);
    let readiness = promotion_readiness_from_inputs("promotion-ready-1", &plan, &[input]);

    assert_json_round_trip(&readiness);
    let encoded = serde_json::to_value(&readiness).expect("readiness should encode");
    assert_object_keys(
        &encoded,
        &[
            "schema_version",
            "readiness_id",
            "promotion_readiness_digest",
            "target_plan_id",
            "status",
            "roles",
            "blockers",
            "warnings",
        ],
    );
    assert!(
        encoded["promotion_readiness_digest"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64)
    );
    let role = &encoded["roles"][0];
    assert_object_keys(
        role,
        &[
            "role",
            "promotion_level",
            "source_kind",
            "source_locator",
            "source_wasm_sha256",
            "source_wasm_gz_sha256",
            "target_wasm_sha256",
            "target_wasm_gz_sha256",
            "source_canonical_embedded_config_sha256",
            "target_canonical_embedded_config_sha256",
            "byte_identical_wasm",
            "embedded_config_identical",
            "target_store_has_artifact",
            "restage_required",
        ],
    );
}

#[test]
fn promotion_plan_transform_round_trips_through_json() {
    let request = PromotionPlanTransformRequest {
        promoted_plan_id: "promoted-plan-1".to_string(),
        target_plan: sample_promotion_target_plan(),
        inputs: vec![sample_role_promotion_input(
            PromotionArtifactLevelV1::SealedWasm,
        )],
    };
    let transform = promoted_deployment_plan_transform_from_inputs(&request)
        .expect("transform should be produced");

    assert_json_round_trip(&transform);
    let encoded = serde_json::to_value(&transform).expect("transform should encode");
    assert_object_keys(
        &encoded,
        &[
            "schema_version",
            "transform_id",
            "target_plan_id",
            "promoted_plan_id",
            "promotion_plan_lineage_digest",
            "promoted_plan",
            "roles",
        ],
    );
    let role = &encoded["roles"][0];
    assert_object_keys(
        role,
        &[
            "role",
            "promotion_level",
            "source_kind",
            "source_locator",
            "artifact_source_before",
            "artifact_source_after",
            "wasm_sha256_before",
            "wasm_sha256_after",
            "wasm_gz_sha256_before",
            "wasm_gz_sha256_after",
            "candid_sha256_before",
            "candid_sha256_after",
            "canonical_embedded_config_sha256_before",
            "canonical_embedded_config_sha256_after",
            "artifact_identity_changed",
            "embedded_config_changed",
            "target_materialization_preserved",
            "source_build_materialization",
        ],
    );
}

#[test]
fn promotion_plan_transform_evidence_round_trips_through_json() {
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

    assert_json_round_trip(&evidence);
    let encoded = serde_json::to_value(&evidence).expect("evidence should encode");
    assert_object_keys(
        &encoded,
        &[
            "schema_version",
            "evidence_id",
            "promotion_plan_transform_evidence_digest",
            "generated_at",
            "transform",
        ],
    );
    assert!(
        encoded["promotion_plan_transform_evidence_digest"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64)
    );
}

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
    assert!(text.contains("target_execution_lineage: target-execution-lineage-1"));
    assert!(text.contains("readiness_roles: 1"));
    assert!(text.contains("artifact_identity_roles: 1"));
    assert!(text.contains("transform_roles: 1"));
    assert!(text.contains("  Promotion readiness report"));
    assert!(text.contains("  Promotion artifact identity report"));
    assert!(text.contains("  Promotion plan transform"));
}
