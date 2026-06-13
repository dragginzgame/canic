use super::*;

#[test]
fn authority_reconciliation_reports_already_correct_controller_state() {
    let check = sample_check(sample_plan(), sample_matching_inventory());

    let plan = build_authority_reconciliation_plan(&check);

    assert_eq!(plan.plan_id, "plan-local-root");
    assert_eq!(plan.inventory_id, "inventory-1");
    assert_eq!(plan.authority_profile_hash.as_deref(), Some("authority"));
    assert!(plan.hard_failures.is_empty());
    assert!(plan.external_actions_required.is_empty());
    assert_eq!(plan.canister_actions.len(), 1);
    assert_eq!(
        plan.canister_actions[0].state,
        AuthorityReconciliationStateV1::AlreadyCorrect
    );
    assert_eq!(plan.canister_actions[0].action, AuthorityActionV1::None);
    assert!(!plan.canister_actions[0].can_apply);
}

#[test]
fn authority_report_summarizes_safe_reconciliation_plan() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let plan = build_authority_reconciliation_plan(&check);

    let report = authority_report_from_plan("authority-report-1", &plan);

    assert_eq!(report.status, SafetyStatusV1::Safe);
    assert_eq!(report.reconciliation_plan_id, "plan-local-root");
    assert_eq!(report.check_id, None);
    assert_eq!(report.inventory_id, "inventory-1");
    assert_eq!(report.authority_profile_hash.as_deref(), Some("authority"));
    assert_eq!(report.counts.already_correct, 1);
    assert_eq!(report.counts.can_apply_automatically, 0);
    assert_eq!(report.counts.requires_external_action, 0);
    assert_eq!(report.counts.unsafe_blocked, 0);
    assert_eq!(report.counts.unknown, 0);
    assert_eq!(report.counts.hard_failures, 0);
    assert_eq!(
        report.apply_readiness,
        AuthorityApplyReadinessV1 {
            can_apply_automatically: false,
            automatic_action_count: 0,
            blockers: Vec::new(),
        }
    );
    assert_eq!(
        report.action_counts,
        vec![AuthorityActionCountV1 {
            action: AuthorityActionV1::None,
            count: 1,
        }]
    );
    assert_eq!(
        report.control_class_counts,
        vec![AuthorityControlClassCountV1 {
            control_class: CanisterControlClassV1::DeploymentControlled,
            count: 1,
        }]
    );
    assert!(report.observation_gaps.is_empty());
    assert!(report.automatic_actions.is_empty());
    assert!(report.external_actions_required.is_empty());
    assert!(report.next_actions.is_empty());
}

#[test]
fn authority_report_can_preserve_source_check_id() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let plan = build_authority_reconciliation_plan(&check);

    let report =
        authority_report_from_plan_with_check_id("authority-report-1", Some(check.check_id), &plan);

    assert_eq!(report.check_id.as_deref(), Some("check-1"));
    assert_eq!(report.reconciliation_plan_id, "plan-local-root");
    assert_eq!(report.inventory_id, "inventory-1");
}

#[test]
fn authority_report_from_check_preserves_source_provenance() {
    let check = sample_check(sample_plan(), sample_matching_inventory());

    let report = authority_report_from_check("authority-report-1", &check);

    assert_eq!(report.check_id.as_deref(), Some("check-1"));
    assert_eq!(report.reconciliation_plan_id, "plan-local-root");
    assert_eq!(report.inventory_id, "inventory-1");
    assert_eq!(report.authority_profile_hash.as_deref(), Some("authority"));
    assert_eq!(report.counts.already_correct, 1);
}

#[test]
fn authority_report_from_check_with_local_id_uses_deployment_identity() {
    let check = sample_check(sample_plan(), sample_matching_inventory());

    let report = authority_report_from_check_with_local_id(&check);

    assert_eq!(report.report_id, "local:local:local-root:authority-report");
    assert_eq!(report.check_id.as_deref(), Some("check-1"));
    assert_eq!(report.reconciliation_plan_id, "plan-local-root");
    assert_eq!(report.inventory_id, "inventory-1");
}

#[test]
fn authority_dry_run_evidence_from_check_with_local_ids_uses_deployment_identity() {
    let check = sample_check(sample_plan(), sample_matching_inventory());

    let evidence =
        authority_dry_run_evidence_from_check_with_local_ids(&check, "2026-05-23T00:00:01Z")
            .expect("build authority evidence");

    assert_eq!(
        evidence.evidence_id,
        "local:local:local-root:authority-evidence"
    );
    assert_eq!(evidence.check_id, "check-1");
    assert_eq!(
        evidence.authority_report.report_id,
        "local:local:local-root:authority-report"
    );
    assert_eq!(
        evidence.authority_receipt.operation_id,
        "local:local:local-root:authority-dry-run-receipt"
    );
    assert_eq!(
        evidence.authority_receipt.authority_report_id,
        evidence.authority_report.report_id
    );
    assert_eq!(evidence.generated_at, "2026-05-23T00:00:01Z");
    assert_eq!(
        evidence.authority_receipt.finished_at.as_deref(),
        Some("2026-05-23T00:00:01Z")
    );
}

#[test]
fn authority_dry_run_receipt_from_check_with_local_id_uses_deployment_identity() {
    let check = sample_check(sample_plan(), sample_matching_inventory());

    let receipt =
        authority_dry_run_receipt_from_check_with_local_id(&check, "2026-05-23T00:00:01Z")
            .expect("build authority receipt");

    assert_eq!(
        receipt.operation_id,
        "local:local:local-root:authority-dry-run-receipt"
    );
    assert_eq!(receipt.check_id.as_deref(), Some("check-1"));
    assert_eq!(receipt.reconciliation_plan_id, "plan-local-root");
    assert_eq!(
        receipt.authority_report_id,
        "local:local:local-root:authority-report"
    );
    assert_eq!(receipt.inventory_id, "inventory-1");
    assert_eq!(receipt.authority_profile_hash.as_deref(), Some("authority"));
    assert_eq!(receipt.finished_at.as_deref(), Some("2026-05-23T00:00:01Z"));
    assert!(receipt.attempted_actions.is_empty());
}

#[test]
fn authority_dry_run_receipt_from_check_preserves_explicit_report_id() {
    let check = sample_check(sample_plan(), sample_matching_inventory());

    let receipt = authority_dry_run_receipt_from_check(
        &check,
        "authority-report-explicit",
        "authority-dry-run-explicit",
        "2026-05-23T00:00:00Z",
        Some("2026-05-23T00:00:01Z".to_string()),
    )
    .expect("build authority receipt");

    assert_eq!(receipt.operation_id, "authority-dry-run-explicit");
    assert_eq!(receipt.authority_report_id, "authority-report-explicit");
    assert_eq!(receipt.check_id.as_deref(), Some("check-1"));
    assert_eq!(receipt.reconciliation_plan_id, "plan-local-root");
}

#[test]
fn authority_text_renders_plan_and_report_summaries() {
    let mut source_plan = sample_plan();
    source_plan.authority_profile.expected_controllers =
        vec!["aaaaa-aa".to_string(), "ops-principal".to_string()];
    let check = sample_check(source_plan, sample_matching_inventory());
    let plan = build_authority_reconciliation_plan(&check);
    let report =
        authority_report_from_plan_with_check_id("authority-report-1", Some(check.check_id), &plan);

    let plan_text = authority_plan_text(&plan);
    let report_text = authority_report_text(&report);

    assert!(plan_text.contains("Authority reconciliation plan"));
    assert!(plan_text.contains("mode: dry_run"));
    assert!(plan_text.contains("plan_id: plan-local-root"));
    assert!(plan_text.contains("root (aaaaa-aa) CanApplyAutomatically/AddControllers"));
    assert!(plan_text.contains("[add=ops-principal; remove=none]"));
    assert!(report_text.contains("Authority reconciliation report"));
    assert!(report_text.contains("mode: dry_run"));
    assert!(report_text.contains("check_id: check-1"));
    assert!(report_text.contains("status: safe"));
    assert!(report_text.contains("[add=ops-principal; remove=none]"));
}

#[test]
fn authority_text_renders_evidence_and_receipt_details() {
    let mut source_plan = sample_plan();
    source_plan.authority_profile.staging_controllers = vec!["aaaaa-aa".to_string()];
    let check = sample_check(source_plan, sample_matching_inventory());
    let plan = build_authority_reconciliation_plan(&check);
    let report = authority_report_from_plan_with_check_id(
        "authority-report-1",
        Some(check.check_id.clone()),
        &plan,
    );
    let receipt = authority_dry_run_receipt_from_plan(
        &plan,
        &report,
        Some(check.check_id.clone()),
        "authority-dry-run-1",
        "2026-05-23T00:00:00Z",
        Some("2026-05-23T00:00:01Z".to_string()),
    )
    .expect("build receipt");
    let evidence = AuthorityDryRunEvidenceV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        evidence_id: "authority-evidence-1".to_string(),
        check_id: check.check_id,
        generated_at: "2026-05-23T00:00:00Z".to_string(),
        reconciliation_plan: plan,
        authority_report: report,
        authority_receipt: receipt,
    };

    let evidence_text = authority_evidence_text(&evidence);
    let receipt_text = authority_receipt_text(&evidence.authority_receipt);

    assert!(evidence_text.contains("Authority dry-run evidence"));
    assert!(evidence_text.contains("mode: dry_run"));
    assert!(evidence_text.contains("evidence_id: authority-evidence-1"));
    assert!(evidence_text.contains("generated_at: 2026-05-23T00:00:00Z"));
    assert!(evidence_text.contains("controller_mutation: none_attempted"));
    assert!(evidence_text.contains("verified_controller_observations:"));
    assert!(
        evidence_text
            .contains("aaaaa-aa AlreadyCorrect/None: observed=[aaaaa-aa] desired=[aaaaa-aa]")
    );
    assert!(evidence_text.contains(
        "[authority_profile_overlap] aaaaa-aa: staging authority principal aaaaa-aa overlaps"
    ));
    assert!(receipt_text.contains("Authority dry-run receipt"));
    assert!(receipt_text.contains("mode: dry_run"));
    assert!(receipt_text.contains("operation_id: authority-dry-run-1"));
    assert!(receipt_text.contains("controller_mutation: none_attempted"));
    assert!(receipt_text.contains("verified_controller_observations:"));
    assert!(
        receipt_text
            .contains("aaaaa-aa AlreadyCorrect/None: observed=[aaaaa-aa] desired=[aaaaa-aa]")
    );
    assert!(receipt_text.contains(
        "[authority_profile_overlap] aaaaa-aa: staging authority principal aaaaa-aa overlaps"
    ));
}

#[test]
fn authority_receipt_rejects_mismatched_report_provenance() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let reconciliation = build_authority_reconciliation_plan(&check);
    let mut report = authority_report_from_plan_with_check_id(
        "authority-report-1",
        Some(check.check_id.clone()),
        &reconciliation,
    );
    report.inventory_id = "other-inventory".to_string();

    let err = authority_dry_run_receipt_from_plan(
        &reconciliation,
        &report,
        Some(check.check_id),
        "authority-dry-run-1",
        "2026-05-23T00:00:00Z",
        Some("2026-05-23T00:00:01Z".to_string()),
    )
    .expect_err("mismatched report inventory should fail receipt construction");

    std::assert_matches!(
        err,
        AuthorityEvidenceError::PlanReportMismatch {
            field: "inventory_id",
            ..
        }
    );
}

#[test]
fn authority_receipt_rejects_mismatched_report_content() {
    let mut plan = sample_plan();
    plan.authority_profile.expected_controllers =
        vec!["aaaaa-aa".to_string(), "ops-principal".to_string()];
    let check = sample_check(plan, sample_matching_inventory());
    let reconciliation = build_authority_reconciliation_plan(&check);
    let mut report = authority_report_from_plan_with_check_id(
        "authority-report-1",
        Some(check.check_id.clone()),
        &reconciliation,
    );
    report.automatic_actions.clear();

    let err = authority_dry_run_receipt_from_plan(
        &reconciliation,
        &report,
        Some(check.check_id),
        "authority-dry-run-1",
        "2026-05-23T00:00:00Z",
        Some("2026-05-23T00:00:01Z".to_string()),
    )
    .expect_err("mismatched report content should fail receipt construction");

    std::assert_matches!(
        err,
        AuthorityEvidenceError::PlanReportContentMismatch {
            field: "automatic_actions",
        }
    );
}

#[test]
fn authority_receipt_rejects_mismatched_check_id() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let reconciliation = build_authority_reconciliation_plan(&check);
    let report = authority_report_from_plan_with_check_id(
        "authority-report-1",
        Some(check.check_id),
        &reconciliation,
    );

    let err = authority_dry_run_receipt_from_plan(
        &reconciliation,
        &report,
        Some("other-check".to_string()),
        "authority-dry-run-1",
        "2026-05-23T00:00:00Z",
        Some("2026-05-23T00:00:01Z".to_string()),
    )
    .expect_err("mismatched check id should fail receipt construction");

    std::assert_matches!(err, AuthorityEvidenceError::CheckIdMismatch { .. });
}

#[test]
fn authority_receipt_rejects_unsupported_source_schema_version() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let mut reconciliation = build_authority_reconciliation_plan(&check);
    let report = authority_report_from_plan_with_check_id(
        "authority-report-1",
        Some(check.check_id.clone()),
        &reconciliation,
    );
    reconciliation.schema_version = DEPLOYMENT_TRUTH_SCHEMA_VERSION + 1;

    let err = authority_dry_run_receipt_from_plan(
        &reconciliation,
        &report,
        Some(check.check_id),
        "authority-dry-run-1",
        "2026-05-23T00:00:00Z",
        Some("2026-05-23T00:00:01Z".to_string()),
    )
    .expect_err("unsupported plan schema should fail receipt construction");

    std::assert_matches!(
        err,
        AuthorityEvidenceError::SchemaVersionMismatch {
            component: "plan",
            expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            found
        } if found == DEPLOYMENT_TRUTH_SCHEMA_VERSION + 1
    );
}

#[test]
fn authority_receipt_rejects_blank_receipt_identity_inputs() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let reconciliation = build_authority_reconciliation_plan(&check);
    let report = authority_report_from_plan_with_check_id(
        "authority-report-1",
        Some(check.check_id.clone()),
        &reconciliation,
    );

    let err = authority_dry_run_receipt_from_plan(
        &reconciliation,
        &report,
        Some(check.check_id),
        " ",
        "2026-05-23T00:00:00Z",
        Some("2026-05-23T00:00:01Z".to_string()),
    )
    .expect_err("blank receipt operation id should fail receipt construction");

    std::assert_matches!(
        err,
        AuthorityEvidenceError::MissingRequiredField {
            field: "receipt.operation_id",
        }
    );
}

#[test]
fn authority_receipt_rejects_missing_report_check_provenance() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let reconciliation = build_authority_reconciliation_plan(&check);
    let mut report = authority_report_from_plan_with_check_id(
        "authority-report-1",
        Some(check.check_id.clone()),
        &reconciliation,
    );
    report.check_id = None;

    let err = authority_dry_run_receipt_from_plan(
        &reconciliation,
        &report,
        Some(check.check_id),
        "authority-dry-run-1",
        "2026-05-23T00:00:00Z",
        Some("2026-05-23T00:00:01Z".to_string()),
    )
    .expect_err("receipt construction should require report check provenance");

    std::assert_matches!(
        err,
        AuthorityEvidenceError::MissingRequiredField {
            field: "report.check_id",
        }
    );
}

#[test]
fn authority_receipt_rejects_missing_finished_at() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let reconciliation = build_authority_reconciliation_plan(&check);
    let report = authority_report_from_plan_with_check_id(
        "authority-report-1",
        Some(check.check_id.clone()),
        &reconciliation,
    );

    let err = authority_dry_run_receipt_from_plan(
        &reconciliation,
        &report,
        Some(check.check_id),
        "authority-dry-run-1",
        "2026-05-23T00:00:00Z",
        None,
    )
    .expect_err("completed dry-run receipt should require finished_at");

    std::assert_matches!(
        err,
        AuthorityEvidenceError::MissingRequiredField {
            field: "receipt.finished_at",
        }
    );
}

#[test]
fn authority_receipt_rejects_finished_before_started() {
    let check = sample_check(sample_plan(), sample_matching_inventory());
    let reconciliation = build_authority_reconciliation_plan(&check);
    let report = authority_report_from_plan_with_check_id(
        "authority-report-1",
        Some(check.check_id.clone()),
        &reconciliation,
    );

    let err = authority_dry_run_receipt_from_plan(
        &reconciliation,
        &report,
        Some(check.check_id),
        "authority-dry-run-1",
        "2026-05-23T00:00:02Z",
        Some("2026-05-23T00:00:01Z".to_string()),
    )
    .expect_err("receipt construction should reject invalid timestamp order");

    std::assert_matches!(
        err,
        AuthorityEvidenceError::DryRunReceiptTimestampOrder {
            field: "receipt.started_at",
            other_field: "receipt.finished_at",
            ..
        }
    );
}

#[test]
fn authority_dry_run_evidence_rejects_mismatched_nested_check_id() {
    let mut evidence = sample_authority_evidence();
    evidence.check_id = "other-check".to_string();

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("mismatched nested check id should fail evidence validation");

    std::assert_matches!(
        err,
        AuthorityEvidenceError::EvidenceCheckIdMismatch {
            component: "report",
            ..
        }
    );
}

#[test]
fn authority_dry_run_evidence_rejects_unsupported_schema_version() {
    let mut evidence = sample_authority_evidence();
    evidence.schema_version = DEPLOYMENT_TRUTH_SCHEMA_VERSION + 1;

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("unsupported evidence schema should fail validation");

    std::assert_matches!(
        err,
        AuthorityEvidenceError::SchemaVersionMismatch {
            component: "evidence",
            expected: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
            found
        } if found == DEPLOYMENT_TRUTH_SCHEMA_VERSION + 1
    );
}

#[test]
fn authority_dry_run_evidence_rejects_nested_schema_version_drift() {
    let mut evidence = sample_authority_evidence();
    evidence.authority_report.schema_version = DEPLOYMENT_TRUTH_SCHEMA_VERSION + 1;

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("nested schema drift should fail evidence validation");

    std::assert_matches!(
        err,
        AuthorityEvidenceError::SchemaVersionMismatch {
            component: "report",
            ..
        }
    );
}

#[test]
fn authority_dry_run_evidence_rejects_blank_required_identity() {
    let mut evidence = sample_authority_evidence();
    evidence.evidence_id = "  ".to_string();

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("blank evidence identity should fail validation");

    std::assert_matches!(
        err,
        AuthorityEvidenceError::MissingRequiredField {
            field: "evidence.evidence_id"
        }
    );
}

#[test]
fn authority_dry_run_evidence_rejects_missing_nested_check_provenance() {
    let mut evidence = sample_authority_evidence();
    evidence.authority_report.check_id = None;

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("full evidence should carry nested report check provenance");

    std::assert_matches!(
        err,
        AuthorityEvidenceError::MissingRequiredField {
            field: "report.check_id"
        }
    );
}

#[test]
fn authority_dry_run_evidence_rejects_mismatched_receipt_content() {
    let mut evidence = sample_authority_evidence();
    evidence
        .authority_receipt
        .hard_failures
        .push(SafetyFindingV1 {
            code: "extra".to_string(),
            message: "extra hard finding".to_string(),
            severity: SafetySeverityV1::HardFailure,
            subject: None,
        });

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("mismatched receipt content should fail evidence validation");

    std::assert_matches!(
        err,
        AuthorityEvidenceError::PlanReportContentMismatch {
            field: "receipt.hard_failures",
        }
    );
}

#[test]
fn authority_dry_run_evidence_rejects_mutated_report_counts() {
    let mut evidence = sample_authority_evidence();
    evidence.authority_report.counts.already_correct = 0;

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("mutated report counts should fail evidence validation");

    std::assert_matches!(
        err,
        AuthorityEvidenceError::PlanReportContentMismatch {
            field: "report.counts",
        }
    );
}

#[test]
fn authority_dry_run_evidence_rejects_mutated_report_readiness() {
    let mut evidence = sample_authority_evidence();
    evidence
        .authority_report
        .apply_readiness
        .can_apply_automatically = true;

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("mutated report readiness should fail evidence validation");

    std::assert_matches!(
        err,
        AuthorityEvidenceError::PlanReportContentMismatch {
            field: "report.apply_readiness",
        }
    );
}

#[test]
fn authority_dry_run_evidence_rejects_mutated_unsafe_blocker_readiness() {
    let mut evidence = sample_authority_evidence_from_check(sample_unknown_unsafe_check());
    assert_eq!(
        evidence.authority_report.apply_readiness.blockers,
        vec![AuthorityApplyBlockerV1::UnsafeBlocked]
    );

    evidence.authority_report.apply_readiness.blockers =
        vec![AuthorityApplyBlockerV1::HardFailures];

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("mutated unsafe blocker readiness should fail evidence validation");

    std::assert_matches!(
        err,
        AuthorityEvidenceError::PlanReportContentMismatch {
            field: "report.apply_readiness",
        }
    );
}

#[test]
fn authority_dry_run_evidence_rejects_attempted_actions() {
    let mut evidence = sample_authority_evidence();
    evidence
        .authority_receipt
        .attempted_actions
        .push(AuthorityAttemptedActionV1 {
            subject: "aaaaa-aa".to_string(),
            canister_id: Some("aaaaa-aa".to_string()),
            role: Some("root".to_string()),
            action: AuthorityActionV1::AddControllers,
            result: RolePhaseResultV1::NotAttempted,
            error: None,
        });

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("attempted dry-run actions should fail evidence validation");

    std::assert_matches!(
        err,
        AuthorityEvidenceError::DryRunReceiptAttemptedActions { count: 1 }
    );
}

#[test]
fn authority_dry_run_evidence_rejects_non_complete_receipt_status() {
    let mut evidence = sample_authority_evidence();
    evidence.authority_receipt.operation_status = DeploymentExecutionStatusV1::FailedBeforeMutation;

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("non-complete dry-run receipts should fail evidence validation");

    std::assert_matches!(
        err,
        AuthorityEvidenceError::DryRunReceiptStatus {
            status: DeploymentExecutionStatusV1::FailedBeforeMutation
        }
    );
}

#[test]
fn authority_dry_run_evidence_rejects_failed_receipt_command_result() {
    let mut evidence = sample_authority_evidence();
    evidence.authority_receipt.command_result = DeploymentCommandResultV1::Failed {
        code: "dry_run_failed".to_string(),
        message: "dry run failed".to_string(),
    };

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("failed dry-run command results should fail evidence validation");

    std::assert_matches!(
        err,
        AuthorityEvidenceError::DryRunReceiptCommandResult {
            result: DeploymentCommandResultV1::Failed { .. }
        }
    );
}

#[test]
fn authority_dry_run_evidence_rejects_complete_receipt_without_finished_at() {
    let mut evidence = sample_authority_evidence();
    evidence.authority_receipt.finished_at = None;

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("complete dry-run receipts should record finished_at");

    std::assert_matches!(err, AuthorityEvidenceError::DryRunReceiptMissingFinishedAt);
}

#[test]
fn authority_dry_run_evidence_rejects_generated_at_mismatch() {
    let mut evidence = sample_authority_evidence();
    evidence.generated_at = "2026-05-23T00:00:02Z".to_string();

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("evidence generated_at should match receipt completion time");

    std::assert_matches!(
        err,
        AuthorityEvidenceError::EvidenceGeneratedAtMismatch {
            evidence_value,
            receipt_value,
        } if evidence_value == "2026-05-23T00:00:02Z"
            && receipt_value == "2026-05-23T00:00:01Z"
    );
}

#[test]
fn authority_dry_run_evidence_rejects_receipt_finished_before_started() {
    let mut evidence = sample_authority_evidence();
    evidence.authority_receipt.started_at = "2026-05-23T00:00:02Z".to_string();

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("dry-run receipt finish time should not precede start time");

    std::assert_matches!(
        err,
        AuthorityEvidenceError::DryRunReceiptTimestampOrder {
            field: "receipt.started_at",
            other_field: "receipt.finished_at",
            ..
        }
    );
}

#[test]
fn authority_dry_run_evidence_rejects_mismatched_controller_observations() {
    let mut evidence = sample_authority_evidence();
    evidence
        .authority_receipt
        .verified_controller_observations
        .clear();

    let err = validate_authority_dry_run_evidence(&evidence)
        .expect_err("mismatched controller observations should fail evidence validation");

    std::assert_matches!(
        err,
        AuthorityEvidenceError::PlanReportContentMismatch {
            field: "receipt.verified_controller_observations",
        }
    );
}

#[test]
fn authority_reconciliation_marks_deployment_controlled_delta_as_automatic_dry_run() {
    let mut plan = sample_plan();
    plan.authority_profile.expected_controllers =
        vec!["aaaaa-aa".to_string(), "ops-principal".to_string()];
    let check = sample_check(plan, sample_matching_inventory());

    let reconciliation = build_authority_reconciliation_plan(&check);

    assert!(reconciliation.hard_failures.is_empty());
    assert!(reconciliation.external_actions_required.is_empty());
    assert_eq!(
        reconciliation.canister_actions[0].state,
        AuthorityReconciliationStateV1::CanApplyAutomatically
    );
    assert_eq!(
        reconciliation.canister_actions[0].action,
        AuthorityActionV1::AddControllers
    );
    assert!(reconciliation.canister_actions[0].can_apply);
    assert!(
        reconciliation.canister_actions[0]
            .reason
            .contains("ops-principal")
    );
    assert_eq!(reconciliation.automatic_actions.len(), 1);
    assert_eq!(reconciliation.automatic_actions[0].subject, "aaaaa-aa");
    assert_eq!(reconciliation.automatic_actions[0].canister_id, "aaaaa-aa");
    assert_eq!(
        reconciliation.automatic_actions[0].action,
        AuthorityActionV1::AddControllers
    );
    assert_eq!(
        reconciliation.automatic_actions[0].observed_controllers,
        vec!["aaaaa-aa".to_string()]
    );
    assert_eq!(
        reconciliation.automatic_actions[0].desired_controllers,
        vec!["aaaaa-aa".to_string(), "ops-principal".to_string()]
    );
    assert_eq!(
        reconciliation.automatic_actions[0].controller_delta,
        AuthorityControllerDeltaV1 {
            add_controllers: vec!["ops-principal".to_string()],
            remove_controllers: Vec::new(),
        }
    );

    let report = authority_report_from_plan("authority-report-1", &reconciliation);
    assert_eq!(report.status, SafetyStatusV1::Safe);
    assert_eq!(report.counts.can_apply_automatically, 1);
    assert_eq!(
        report.apply_readiness,
        AuthorityApplyReadinessV1 {
            can_apply_automatically: true,
            automatic_action_count: 1,
            blockers: Vec::new(),
        }
    );
    assert_eq!(
        report.action_counts,
        vec![AuthorityActionCountV1 {
            action: AuthorityActionV1::AddControllers,
            count: 1,
        }]
    );
    assert!(report.observation_gaps.is_empty());
    assert_eq!(report.automatic_actions, reconciliation.automatic_actions);
    assert_eq!(
        report.next_actions,
        vec![
            "review automatic authority dry-run actions before enabling an apply path".to_string()
        ]
    );
}

#[test]
fn authority_apply_readiness_blocks_automatic_candidates_when_external_actions_remain() {
    let mut plan = sample_plan();
    plan.authority_profile.expected_controllers =
        vec!["aaaaa-aa".to_string(), "ops-principal".to_string()];
    plan.expected_canisters.push(ExpectedCanisterV1 {
        role: "user_hub".to_string(),
        canister_id: Some("user-hub-canister".to_string()),
        control_class: CanisterControlClassV1::UserControlled,
    });
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters.push(ObservedCanisterV1 {
        canister_id: "user-hub-canister".to_string(),
        role: Some("user_hub".to_string()),
        control_class: CanisterControlClassV1::UserControlled,
        controllers: vec!["user-controller".to_string()],
        module_hash: None,
        status: Some("running".to_string()),
        root_trust_anchor: Some("aaaaa-aa".to_string()),
        canonical_embedded_config_digest: None,
        role_assignment_source: Some("icp_canister_status".to_string()),
    });
    let check = sample_check(plan, inventory);

    let reconciliation = build_authority_reconciliation_plan(&check);
    let report = authority_report_from_plan("authority-report-1", &reconciliation);

    assert_eq!(report.counts.can_apply_automatically, 1);
    assert_eq!(report.counts.requires_external_action, 1);
    assert_eq!(
        report.apply_readiness,
        AuthorityApplyReadinessV1 {
            can_apply_automatically: false,
            automatic_action_count: 1,
            blockers: vec![AuthorityApplyBlockerV1::ExternalActions],
        }
    );
    assert_eq!(
        report.next_actions,
        vec![
            "review external authority actions before applying controller changes",
            "review automatic authority dry-run actions before enabling an apply path",
        ]
    );
}

#[test]
fn authority_reconciliation_blocks_staging_or_emergency_controller_overlap() {
    let mut plan = sample_plan();
    plan.authority_profile.staging_controllers = vec!["aaaaa-aa".to_string()];
    plan.authority_profile.emergency_controllers = vec!["aaaaa-aa".to_string()];
    let check = sample_check(plan, sample_matching_inventory());

    let reconciliation = build_authority_reconciliation_plan(&check);

    assert_eq!(reconciliation.hard_failures.len(), 2);
    assert!(
        reconciliation
            .hard_failures
            .iter()
            .all(|finding| finding.code == "authority_profile_overlap"
                && finding.severity == SafetySeverityV1::HardFailure
                && finding.subject.as_deref() == Some("aaaaa-aa"))
    );
    assert_eq!(
        reconciliation.canister_actions[0].state,
        AuthorityReconciliationStateV1::AlreadyCorrect
    );

    let report = authority_report_from_plan("authority-report-1", &reconciliation);
    assert_eq!(report.status, SafetyStatusV1::Blocked);
    assert_eq!(report.counts.already_correct, 1);
    assert_eq!(report.counts.unsafe_blocked, 0);
    assert_eq!(report.counts.hard_failures, 2);
    assert_eq!(
        report.apply_readiness,
        AuthorityApplyReadinessV1 {
            can_apply_automatically: false,
            automatic_action_count: 0,
            blockers: vec![AuthorityApplyBlockerV1::HardFailures],
        }
    );
    assert_eq!(report.hard_failures, reconciliation.hard_failures);
    assert_eq!(
        report.next_actions,
        vec!["resolve hard authority findings before applying controller changes"]
    );
}

#[test]
fn authority_reconciliation_requires_external_action_for_user_controlled_drift() {
    let mut plan = sample_plan();
    plan.authority_profile.expected_controllers = vec!["aaaaa-aa".to_string()];
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    inventory.observed_canisters[0].controllers = vec!["user-controller".to_string()];
    let check = sample_check(plan, inventory);

    let reconciliation = build_authority_reconciliation_plan(&check);

    assert!(reconciliation.hard_failures.is_empty());
    assert_eq!(reconciliation.external_actions_required.len(), 1);
    let external = &reconciliation.external_actions_required[0];
    assert_eq!(external.subject, "aaaaa-aa");
    assert_eq!(external.canister_id.as_deref(), Some("aaaaa-aa"));
    assert_eq!(external.role.as_deref(), Some("root"));
    assert_eq!(
        external.control_classification,
        CanisterControlClassV1::UserControlled
    );
    assert_eq!(
        external.state,
        AuthorityReconciliationStateV1::RequiresExternalAction
    );
    assert_eq!(
        external.action,
        AuthorityActionV1::RequiresExternalController
    );
    assert_eq!(
        external.observed_controllers,
        vec!["user-controller".to_string()]
    );
    assert_eq!(external.desired_controllers, vec!["aaaaa-aa".to_string()]);
    assert_eq!(
        external.controller_delta,
        AuthorityControllerDeltaV1 {
            add_controllers: vec!["aaaaa-aa".to_string()],
            remove_controllers: vec!["user-controller".to_string()],
        }
    );
    assert_eq!(
        reconciliation.canister_actions[0].state,
        AuthorityReconciliationStateV1::RequiresExternalAction
    );
    assert_eq!(
        reconciliation.canister_actions[0].action,
        AuthorityActionV1::RequiresExternalController
    );
    assert!(!reconciliation.canister_actions[0].can_apply);

    let report = authority_report_from_plan("authority-report-1", &reconciliation);
    assert_eq!(report.status, SafetyStatusV1::Warning);
    assert_eq!(report.counts.requires_external_action, 1);
    assert_eq!(
        report.apply_readiness,
        AuthorityApplyReadinessV1 {
            can_apply_automatically: false,
            automatic_action_count: 0,
            blockers: vec![AuthorityApplyBlockerV1::ExternalActions],
        }
    );
    assert_eq!(report.external_actions_required.len(), 1);
    assert_eq!(report.external_actions_required[0], *external);
    assert_eq!(
        report.next_actions,
        vec!["review external authority actions before applying controller changes"]
    );
}

#[test]
fn authority_dry_run_receipt_records_observations_without_attempts() {
    let mut plan = sample_plan();
    plan.authority_profile.expected_controllers = vec!["aaaaa-aa".to_string()];
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    inventory.observed_canisters[0].controllers = vec!["user-controller".to_string()];
    let check = sample_check(plan, inventory);
    let reconciliation = build_authority_reconciliation_plan(&check);
    let report = authority_report_from_plan_with_check_id(
        "authority-report-1",
        Some(check.check_id.clone()),
        &reconciliation,
    );

    let receipt = authority_dry_run_receipt_from_plan(
        &reconciliation,
        &report,
        Some(check.check_id.clone()),
        "authority-dry-run-1",
        "2026-05-23T00:00:00Z",
        Some("2026-05-23T00:00:01Z".to_string()),
    )
    .expect("build authority receipt");

    assert_eq!(receipt.operation_id, "authority-dry-run-1");
    assert_eq!(receipt.check_id.as_deref(), Some("check-1"));
    assert_eq!(receipt.reconciliation_plan_id, "plan-local-root");
    assert_eq!(receipt.authority_report_id, "authority-report-1");
    assert_eq!(receipt.inventory_id, "inventory-1");
    assert_eq!(receipt.authority_profile_hash.as_deref(), Some("authority"));
    assert_eq!(
        receipt.operation_status,
        DeploymentExecutionStatusV1::Complete
    );
    assert_eq!(receipt.command_result, DeploymentCommandResultV1::Succeeded);
    assert!(receipt.attempted_actions.is_empty());
    assert_eq!(receipt.verified_controller_observations.len(), 1);
    assert_eq!(
        receipt.verified_controller_observations[0],
        AuthorityControllerObservationV1 {
            subject: "aaaaa-aa".to_string(),
            canister_id: Some("aaaaa-aa".to_string()),
            role: Some("root".to_string()),
            state: AuthorityReconciliationStateV1::RequiresExternalAction,
            action: AuthorityActionV1::RequiresExternalController,
            observed_controllers: vec!["user-controller".to_string()],
            desired_controllers: vec!["aaaaa-aa".to_string()],
            controller_delta: AuthorityControllerDeltaV1 {
                add_controllers: vec!["aaaaa-aa".to_string()],
                remove_controllers: vec!["user-controller".to_string()],
            },
        }
    );
    assert_eq!(
        receipt.unresolved_external_actions,
        report.external_actions_required
    );
    assert_eq!(receipt.hard_failures, report.hard_failures);
    assert_eq!(receipt.unresolved_observation_gaps, report.observation_gaps);

    let evidence = AuthorityDryRunEvidenceV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        evidence_id: "authority-evidence-1".to_string(),
        check_id: check.check_id,
        generated_at: "2026-05-23T00:00:01Z".to_string(),
        reconciliation_plan: reconciliation,
        authority_report: report,
        authority_receipt: receipt,
    };

    assert_json_round_trip(&evidence);
}

#[test]
fn authority_v1_json_schema_shape_is_stable() {
    let evidence = sample_authority_evidence();
    let value = serde_json::to_value(&evidence).expect("encode authority evidence");

    assert_object_keys(
        &value,
        &[
            "schema_version",
            "evidence_id",
            "check_id",
            "generated_at",
            "reconciliation_plan",
            "authority_report",
            "authority_receipt",
        ],
    );

    assert_object_keys(
        &value["reconciliation_plan"],
        &[
            "schema_version",
            "plan_id",
            "inventory_id",
            "authority_profile_hash",
            "canister_actions",
            "automatic_actions",
            "hard_failures",
            "external_actions_required",
        ],
    );
    assert_object_keys(
        &value["authority_report"],
        &[
            "schema_version",
            "report_id",
            "check_id",
            "reconciliation_plan_id",
            "inventory_id",
            "authority_profile_hash",
            "status",
            "summary",
            "counts",
            "apply_readiness",
            "action_counts",
            "control_class_counts",
            "observation_gaps",
            "automatic_actions",
            "hard_failures",
            "external_actions_required",
            "next_actions",
        ],
    );
    assert_object_keys(
        &value["authority_receipt"],
        &[
            "schema_version",
            "operation_id",
            "check_id",
            "reconciliation_plan_id",
            "authority_report_id",
            "inventory_id",
            "authority_profile_hash",
            "operation_status",
            "started_at",
            "finished_at",
            "attempted_actions",
            "verified_controller_observations",
            "hard_failures",
            "unresolved_observation_gaps",
            "unresolved_external_actions",
            "command_result",
        ],
    );

    assert_eq!(value["authority_report"]["status"], "Safe");
    assert_eq!(
        value["reconciliation_plan"]["canister_actions"][0]["state"],
        "AlreadyCorrect"
    );
    assert_eq!(
        value["reconciliation_plan"]["canister_actions"][0]["action"],
        "None"
    );
    assert_eq!(
        value["reconciliation_plan"]["canister_actions"][0]["control_classification"],
        "DeploymentControlled"
    );
    assert_eq!(value["authority_receipt"]["operation_status"], "Complete");
    assert_eq!(value["authority_receipt"]["command_result"], "Succeeded");
}

#[test]
fn deployment_truth_authority_paths_have_no_controller_mutation_primitives() {
    for (path, source) in [
        ("authority.rs", include_str!("../authority.rs")),
        ("lifecycle.rs", include_str!("../lifecycle.rs")),
        ("receipt.rs", include_str!("../receipt.rs")),
        ("text.rs", include_str!("../text.rs")),
    ] {
        for forbidden in [
            "update_settings",
            "install_code",
            "create_canister",
            "delete_canister",
            "stop_canister",
            "uninstall_code",
            "provisional_create_canister",
            "dfx",
        ] {
            assert!(
                !source.contains(forbidden),
                "deployment truth authority path {path} must stay dry-run; found forbidden token {forbidden}"
            );
        }
    }
}

#[test]
fn authority_dry_run_receipt_preserves_hard_findings() {
    let mut plan = sample_plan();
    plan.authority_profile.staging_controllers = vec!["aaaaa-aa".to_string()];
    let check = sample_check(plan, sample_matching_inventory());
    let reconciliation = build_authority_reconciliation_plan(&check);
    let report = authority_report_from_plan_with_check_id(
        "authority-report-1",
        Some(check.check_id.clone()),
        &reconciliation,
    );

    let receipt = authority_dry_run_receipt_from_plan(
        &reconciliation,
        &report,
        Some(check.check_id),
        "authority-dry-run-1",
        "2026-05-23T00:00:00Z",
        Some("2026-05-23T00:00:01Z".to_string()),
    )
    .expect("build authority receipt");

    assert_eq!(report.status, SafetyStatusV1::Blocked);
    assert_eq!(report.hard_failures.len(), 1);
    assert_eq!(receipt.hard_failures, report.hard_failures);
    assert!(receipt.unresolved_observation_gaps.is_empty());
    assert!(receipt.attempted_actions.is_empty());
    assert_eq!(receipt.verified_controller_observations.len(), 1);
}

#[test]
fn authority_reconciliation_blocks_unknown_unsafe_canister() {
    let check = sample_unknown_unsafe_check();

    let reconciliation = build_authority_reconciliation_plan(&check);

    assert_eq!(reconciliation.hard_failures.len(), 1);
    assert_eq!(
        reconciliation.hard_failures[0].code,
        "authority_unsafe_blocked"
    );
    assert!(reconciliation.canister_actions.iter().any(|action| {
        action.canister_id.as_deref() == Some("unsafe-canister")
            && action.state == AuthorityReconciliationStateV1::UnsafeBlocked
            && action.action == AuthorityActionV1::BlockedByPolicy
    }));

    let report = authority_report_from_plan("authority-report-1", &reconciliation);
    assert_eq!(report.status, SafetyStatusV1::Blocked);
    assert_eq!(report.counts.unsafe_blocked, 1);
    assert_eq!(report.counts.hard_failures, 0);
    assert_eq!(
        report.apply_readiness,
        AuthorityApplyReadinessV1 {
            can_apply_automatically: false,
            automatic_action_count: 0,
            blockers: vec![AuthorityApplyBlockerV1::UnsafeBlocked],
        }
    );
    assert!(report.external_actions_required.is_empty());
    assert_eq!(
        report.control_class_counts,
        vec![
            AuthorityControlClassCountV1 {
                control_class: CanisterControlClassV1::DeploymentControlled,
                count: 1,
            },
            AuthorityControlClassCountV1 {
                control_class: CanisterControlClassV1::UnknownUnsafe,
                count: 1,
            },
        ]
    );
    assert_eq!(
        report.next_actions,
        vec!["resolve unsafe canister authority findings before applying controller changes"]
    );
    let report_text = authority_report_text(&report);
    assert!(report_text.contains("    - unsafe_blocked"));
    assert!(!report_text.contains("    - hard_failures"));
}

#[test]
fn unsafe_authority_receipt_preserves_finding_without_hard_readiness_double_count() {
    let check = sample_unknown_unsafe_check();
    let reconciliation = build_authority_reconciliation_plan(&check);
    let report = authority_report_from_plan_with_check_id(
        "authority-report-1",
        Some(check.check_id.clone()),
        &reconciliation,
    );

    let receipt = authority_dry_run_receipt_from_plan(
        &reconciliation,
        &report,
        Some(check.check_id),
        "authority-dry-run-1",
        "2026-05-23T00:00:00Z",
        Some("2026-05-23T00:00:01Z".to_string()),
    )
    .expect("build authority receipt");

    assert_eq!(report.counts.unsafe_blocked, 1);
    assert_eq!(report.counts.hard_failures, 0);
    assert_eq!(
        report.apply_readiness.blockers,
        vec![AuthorityApplyBlockerV1::UnsafeBlocked]
    );
    assert_eq!(receipt.hard_failures, report.hard_failures);
    assert_eq!(receipt.hard_failures.len(), 1);
    assert_eq!(receipt.hard_failures[0].code, "authority_unsafe_blocked");
}

#[test]
fn authority_report_distinguishes_unsafe_and_hard_authority_blockers() {
    let mut plan = sample_plan();
    plan.authority_profile.staging_controllers = vec!["aaaaa-aa".to_string()];
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters.push(ObservedCanisterV1 {
        canister_id: "unsafe-canister".to_string(),
        role: Some("surprise".to_string()),
        control_class: CanisterControlClassV1::UnknownUnsafe,
        controllers: vec!["unknown-controller".to_string()],
        module_hash: None,
        status: None,
        root_trust_anchor: None,
        canonical_embedded_config_digest: None,
        role_assignment_source: Some("icp_canister_status".to_string()),
    });
    let check = sample_check(plan, inventory);

    let reconciliation = build_authority_reconciliation_plan(&check);
    let report = authority_report_from_plan("authority-report-1", &reconciliation);

    assert_eq!(reconciliation.hard_failures.len(), 2);
    assert_eq!(report.status, SafetyStatusV1::Blocked);
    assert_eq!(report.counts.unsafe_blocked, 1);
    assert_eq!(report.counts.hard_failures, 1);
    assert_eq!(
        report.apply_readiness,
        AuthorityApplyReadinessV1 {
            can_apply_automatically: false,
            automatic_action_count: 0,
            blockers: vec![
                AuthorityApplyBlockerV1::UnsafeBlocked,
                AuthorityApplyBlockerV1::HardFailures,
            ],
        }
    );
    assert_eq!(
        report.next_actions,
        vec![
            "resolve unsafe canister authority findings before applying controller changes",
            "resolve hard authority findings before applying controller changes",
        ]
    );
}

#[test]
fn blocked_authority_report_keeps_external_and_gap_next_actions() {
    let mut plan = sample_plan();
    plan.authority_profile.expected_controllers =
        vec!["aaaaa-aa".to_string(), "ops-principal".to_string()];
    plan.authority_profile.staging_controllers = vec!["aaaaa-aa".to_string()];
    plan.expected_canisters.push(ExpectedCanisterV1 {
        role: "user_hub".to_string(),
        canister_id: Some("user-hub-canister".to_string()),
        control_class: CanisterControlClassV1::UserControlled,
    });
    plan.expected_pool.push(ExpectedPoolCanisterV1 {
        pool: "user-shards".to_string(),
        canister_id: Some("pool-canister".to_string()),
        role: Some("user_shard".to_string()),
    });
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters.push(ObservedCanisterV1 {
        canister_id: "user-hub-canister".to_string(),
        role: Some("user_hub".to_string()),
        control_class: CanisterControlClassV1::UserControlled,
        controllers: vec!["user-controller".to_string()],
        module_hash: None,
        status: Some("running".to_string()),
        root_trust_anchor: Some("aaaaa-aa".to_string()),
        canonical_embedded_config_digest: None,
        role_assignment_source: Some("icp_canister_status".to_string()),
    });
    inventory.observed_pool.push(ObservedPoolCanisterV1 {
        pool: "user-shards".to_string(),
        canister_id: "pool-canister".to_string(),
        role: Some("user_shard".to_string()),
        control_class: CanisterControlClassV1::CanicManagedPool,
    });
    let check = sample_check(plan, inventory);

    let reconciliation = build_authority_reconciliation_plan(&check);
    let report = authority_report_from_plan("authority-report-1", &reconciliation);

    assert_eq!(report.status, SafetyStatusV1::Blocked);
    assert_eq!(
        report.summary,
        "authority reconciliation is blocked by 0 unsafe canister(s) and 1 hard authority finding(s); also requires 1 external action(s) and has 1 unknown observation(s)"
    );
    assert_eq!(report.counts.hard_failures, 1);
    assert_eq!(report.counts.requires_external_action, 1);
    assert_eq!(report.counts.unknown, 1);
    assert_eq!(
        report.next_actions,
        vec![
            "resolve hard authority findings before applying controller changes",
            "review external authority actions before applying controller changes",
            "collect missing controller observations before applying controller changes",
            "review automatic authority dry-run actions before enabling an apply path",
        ]
    );
}

#[test]
fn authority_reconciliation_reports_expected_pool_controller_observation_gap() {
    let mut plan = sample_plan();
    plan.expected_pool.push(ExpectedPoolCanisterV1 {
        pool: "user-shards".to_string(),
        canister_id: Some("pool-canister".to_string()),
        role: Some("user_shard".to_string()),
    });
    let mut inventory = sample_matching_inventory();
    inventory.observed_pool.push(ObservedPoolCanisterV1 {
        pool: "user-shards".to_string(),
        canister_id: "pool-canister".to_string(),
        role: Some("user_shard".to_string()),
        control_class: CanisterControlClassV1::CanicManagedPool,
    });
    let check = sample_check(plan, inventory);

    let reconciliation = build_authority_reconciliation_plan(&check);

    let pool_action = reconciliation
        .canister_actions
        .iter()
        .find(|action| action.canister_id.as_deref() == Some("pool-canister"))
        .expect("pool action should be reported");
    assert_eq!(pool_action.state, AuthorityReconciliationStateV1::Unknown);
    assert_eq!(pool_action.action, AuthorityActionV1::UnknownObservation);
    assert_eq!(
        pool_action.reason,
        "pool canister controller set was not observed"
    );
    assert!(reconciliation.external_actions_required.is_empty());
    let report = authority_report_from_plan_with_check_id(
        "authority-report-1",
        Some(check.check_id.clone()),
        &reconciliation,
    );
    assert_eq!(report.counts.unknown, 1);
    assert!(report.external_actions_required.is_empty());
    assert_eq!(
        report.apply_readiness,
        AuthorityApplyReadinessV1 {
            can_apply_automatically: false,
            automatic_action_count: 0,
            blockers: vec![AuthorityApplyBlockerV1::ObservationGaps],
        }
    );
    assert_eq!(report.observation_gaps.len(), 1);
    assert_eq!(
        report.observation_gaps[0],
        DeploymentObservationGapV1 {
            key: "authority.controllers.pool-canister".to_string(),
            description: "pool canister controller set was not observed".to_string(),
        }
    );
    let receipt = authority_dry_run_receipt_from_plan(
        &reconciliation,
        &report,
        Some(check.check_id),
        "authority-dry-run-1",
        "2026-05-23T00:00:00Z",
        Some("2026-05-23T00:00:01Z".to_string()),
    )
    .expect("build authority receipt");
    assert_eq!(receipt.unresolved_observation_gaps, report.observation_gaps);
    assert!(receipt.unresolved_external_actions.is_empty());
    assert_eq!(
        report.action_counts,
        vec![
            AuthorityActionCountV1 {
                action: AuthorityActionV1::None,
                count: 1,
            },
            AuthorityActionCountV1 {
                action: AuthorityActionV1::UnknownObservation,
                count: 1,
            },
        ]
    );
    assert_eq!(
        report.control_class_counts,
        vec![
            AuthorityControlClassCountV1 {
                control_class: CanisterControlClassV1::DeploymentControlled,
                count: 1,
            },
            AuthorityControlClassCountV1 {
                control_class: CanisterControlClassV1::CanicManagedPool,
                count: 1,
            },
        ]
    );
    assert_eq!(
        report.next_actions,
        vec!["collect missing controller observations before applying controller changes"]
    );
}

#[test]
fn authority_reconciliation_reports_unplanned_pool_canister_for_external_action() {
    let mut inventory = sample_matching_inventory();
    inventory.observed_pool.push(ObservedPoolCanisterV1 {
        pool: "user-shards".to_string(),
        canister_id: "unplanned-pool".to_string(),
        role: Some("user_shard".to_string()),
        control_class: CanisterControlClassV1::CanicManagedPool,
    });
    let check = sample_check(sample_plan(), inventory);

    let reconciliation = build_authority_reconciliation_plan(&check);

    let pool_action = reconciliation
        .canister_actions
        .iter()
        .find(|action| action.canister_id.as_deref() == Some("unplanned-pool"))
        .expect("unplanned pool action should be reported");
    assert_eq!(
        pool_action.state,
        AuthorityReconciliationStateV1::RequiresExternalAction
    );
    assert_eq!(pool_action.action, AuthorityActionV1::AdoptPlanAvailable);
    assert!(
        reconciliation
            .external_actions_required
            .iter()
            .any(|external| {
                external.subject == "unplanned-pool"
                    && external.action == AuthorityActionV1::AdoptPlanAvailable
                    && external.reason
                        == "observed pool canister is not present in the expected pool plan"
            })
    );
}
