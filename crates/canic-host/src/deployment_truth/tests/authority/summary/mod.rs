use super::super::*;

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
