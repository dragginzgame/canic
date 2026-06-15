use super::super::*;

#[test]
fn external_lifecycle_pending_report_summarizes_external_work() {
    let mut plan = sample_plan();
    plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];
    let check = sample_check(plan, inventory);
    let lifecycle_plan = external_lifecycle_plan_from_check(
        "external-lifecycle-plan-1",
        "lifecycle-authority-1",
        &check,
    );
    let proposal_report = external_upgrade_proposal_report_from_lifecycle_plan(
        "external-upgrade-proposals-1",
        &lifecycle_plan,
        &check,
    );

    let report = external_lifecycle_pending_report_from_plan(
        "external-lifecycle-pending-1",
        &lifecycle_plan,
        &proposal_report,
    );

    assert_eq!(
        report.status,
        ExternalLifecyclePlanStatusV1::PendingExternalAction
    );
    assert_eq!(report.direct_upgrade_count, 0);
    assert_eq!(report.pending_external_count, 1);
    assert_eq!(report.blocked_count, 0);
    assert_eq!(
        report.pending_external_actions[0].proposal_id,
        proposal_report.proposals[0].proposal_id
    );
    assert_eq!(
        report.pending_external_actions[0].proposal_digest,
        proposal_report.proposals[0].proposal_digest
    );
    assert_eq!(report.report_digest.len(), 64);
    validate_external_lifecycle_pending_report(&report).expect("pending report should validate");
    validate_external_lifecycle_pending_report_for_plan(&report, &lifecycle_plan, &proposal_report)
        .expect("pending report should validate against source artifacts");
    assert_json_round_trip(&report);
}

#[test]
fn external_lifecycle_pending_report_validation_rejects_stale_artifacts() {
    let mut plan = sample_plan();
    plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];
    let check = sample_check(plan, inventory);
    let lifecycle_plan = external_lifecycle_plan_from_check(
        "external-lifecycle-plan-1",
        "lifecycle-authority-1",
        &check,
    );
    let mut proposal_report = external_upgrade_proposal_report_from_lifecycle_plan(
        "external-upgrade-proposals-1",
        &lifecycle_plan,
        &check,
    );
    let mut report = external_lifecycle_pending_report_from_plan(
        "external-lifecycle-pending-1",
        &lifecycle_plan,
        &proposal_report,
    );
    report.pending_external_count = 0;

    let err = validate_external_lifecycle_pending_report(&report)
        .expect_err("stale pending report count should fail");
    assert_eq!(err, ExternalLifecyclePendingReportError::CountMismatch);

    let report = external_lifecycle_pending_report_from_plan(
        "external-lifecycle-pending-1",
        &lifecycle_plan,
        &proposal_report,
    );
    proposal_report.report_id = "other-proposal-report".to_string();
    let err = validate_external_lifecycle_pending_report_for_plan(
        &report,
        &lifecycle_plan,
        &proposal_report,
    )
    .expect_err("source drift should fail");
    assert_eq!(
        err,
        ExternalLifecyclePendingReportError::SourceMismatch {
            field: "proposal_report_id"
        }
    );
}

#[test]
fn external_lifecycle_pending_report_text_reports_passive_boundary() {
    let mut plan = sample_plan();
    plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];
    let check = sample_check(plan, inventory);
    let lifecycle_plan = external_lifecycle_plan_from_check(
        "external-lifecycle-plan-1",
        "lifecycle-authority-1",
        &check,
    );
    let proposal_report = external_upgrade_proposal_report_from_lifecycle_plan(
        "external-upgrade-proposals-1",
        &lifecycle_plan,
        &check,
    );
    let report = external_lifecycle_pending_report_from_plan(
        "external-lifecycle-pending-1",
        &lifecycle_plan,
        &proposal_report,
    );

    let text = external_lifecycle_pending_report_text(&report);

    assert!(text.contains("External lifecycle pending report"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("execution: none"));
    assert!(text.contains("pending_external: 1"));
    assert!(text.contains("pending_external_actions"));
}

#[test]
fn external_lifecycle_check_summarizes_pending_report() {
    let mut plan = sample_plan();
    plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];
    let check = sample_check(plan, inventory);
    let lifecycle_plan = external_lifecycle_plan_from_check(
        "external-lifecycle-plan-1",
        "lifecycle-authority-1",
        &check,
    );
    let proposal_report = external_upgrade_proposal_report_from_lifecycle_plan(
        "external-upgrade-proposals-1",
        &lifecycle_plan,
        &check,
    );
    let pending_report = external_lifecycle_pending_report_from_plan(
        "external-lifecycle-pending-1",
        &lifecycle_plan,
        &proposal_report,
    );

    let check_report = external_lifecycle_check_from_reports(
        "external-lifecycle-check-1",
        &lifecycle_plan,
        &proposal_report,
        &pending_report,
    );

    assert_eq!(check_report.check_id, "external-lifecycle-check-1");
    assert_eq!(
        check_report.status,
        ExternalLifecyclePlanStatusV1::PendingExternalAction
    );
    assert_eq!(check_report.direct_upgrade_count, 0);
    assert_eq!(check_report.pending_external_count, 1);
    assert_eq!(check_report.blocked_count, 0);
    assert_eq!(
        check_report.pending_report_digest,
        pending_report.report_digest
    );
    assert!(check_report.summary.contains("pending external action"));
    assert_eq!(check_report.next_actions.len(), 1);
    assert_eq!(check_report.check_digest.len(), 64);
    validate_external_lifecycle_check(&check_report).expect("check should validate");
    validate_external_lifecycle_check_for_reports(
        &check_report,
        &lifecycle_plan,
        &proposal_report,
        &pending_report,
    )
    .expect("check should validate against source reports");
    assert_json_round_trip(&check_report);
}

#[test]
fn external_lifecycle_check_validation_rejects_stale_source() {
    let (lifecycle_plan, mut pending_report) = sample_external_lifecycle_pending_artifacts();
    let proposal_report = ExternalUpgradeProposalReportV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        report_id: pending_report.proposal_report_id.clone(),
        report_digest: pending_report.proposal_report_digest.clone(),
        lifecycle_plan_id: lifecycle_plan.lifecycle_plan_id.clone(),
        lifecycle_plan_digest: lifecycle_plan.lifecycle_plan_digest.clone(),
        deployment_plan_id: lifecycle_plan.deployment_plan_id.clone(),
        deployment_plan_digest: lifecycle_plan.deployment_plan_digest.clone(),
        inventory_id: lifecycle_plan.inventory_id.clone(),
        proposals: Vec::new(),
        blocked_subjects: Vec::new(),
    };
    let check_report = external_lifecycle_check_from_reports(
        "external-lifecycle-check-1",
        &lifecycle_plan,
        &proposal_report,
        &pending_report,
    );
    pending_report.pending_external_count = 0;

    let err = validate_external_lifecycle_check_for_reports(
        &check_report,
        &lifecycle_plan,
        &proposal_report,
        &pending_report,
    )
    .expect_err("stale check count should fail");

    assert_eq!(err, ExternalLifecycleCheckError::CountMismatch);
}

#[test]
fn external_lifecycle_check_text_reports_passive_boundary() {
    let (lifecycle_plan, pending_report) = sample_external_lifecycle_pending_artifacts();
    let proposal_report = ExternalUpgradeProposalReportV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        report_id: pending_report.proposal_report_id.clone(),
        report_digest: pending_report.proposal_report_digest.clone(),
        lifecycle_plan_id: lifecycle_plan.lifecycle_plan_id.clone(),
        lifecycle_plan_digest: lifecycle_plan.lifecycle_plan_digest.clone(),
        deployment_plan_id: lifecycle_plan.deployment_plan_id.clone(),
        deployment_plan_digest: lifecycle_plan.deployment_plan_digest.clone(),
        inventory_id: lifecycle_plan.inventory_id.clone(),
        proposals: Vec::new(),
        blocked_subjects: Vec::new(),
    };
    let check_report = external_lifecycle_check_from_reports(
        "external-lifecycle-check-1",
        &lifecycle_plan,
        &proposal_report,
        &pending_report,
    );

    let text = external_lifecycle_check_text(&check_report);

    assert!(text.contains("External lifecycle check"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("execution: none"));
    assert!(text.contains("pending_external: 1"));
    assert!(text.contains("next_actions"));
}

#[test]
fn external_lifecycle_handoff_packages_pending_proposals() {
    let mut plan = sample_plan();
    plan.expected_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    let mut inventory = sample_matching_inventory();
    inventory.observed_canisters[0].control_class = CanisterControlClassV1::UserControlled;
    inventory.observed_canisters[0].controllers = vec!["user-principal".to_string()];
    let check = sample_check(plan, inventory);
    let lifecycle_plan = external_lifecycle_plan_from_check(
        "external-lifecycle-plan-1",
        "lifecycle-authority-1",
        &check,
    );
    let proposal_report = external_upgrade_proposal_report_from_lifecycle_plan(
        "external-upgrade-proposals-1",
        &lifecycle_plan,
        &check,
    );
    let pending_report = external_lifecycle_pending_report_from_plan(
        "external-lifecycle-pending-1",
        &lifecycle_plan,
        &proposal_report,
    );
    let lifecycle_check = external_lifecycle_check_from_reports(
        "external-lifecycle-check-1",
        &lifecycle_plan,
        &proposal_report,
        &pending_report,
    );

    let handoff = external_lifecycle_handoff_from_reports(
        "external-lifecycle-handoff-1",
        &lifecycle_check,
        &proposal_report,
        &pending_report,
    );

    assert_eq!(handoff.handoff_id, "external-lifecycle-handoff-1");
    assert_eq!(handoff.lifecycle_check_id, lifecycle_check.check_id);
    assert_eq!(handoff.handoff_actions.len(), 1);
    assert_eq!(
        handoff.handoff_actions[0].proposal_id,
        proposal_report.proposals[0].proposal_id
    );
    assert!(
        handoff.handoff_actions[0]
            .operator_instructions
            .iter()
            .any(|instruction| instruction.contains("verify live inventory"))
    );
    assert_eq!(handoff.handoff_digest.len(), 64);
    validate_external_lifecycle_handoff(&handoff).expect("handoff should validate");
    validate_external_lifecycle_handoff_for_reports(
        &handoff,
        &lifecycle_check,
        &proposal_report,
        &pending_report,
    )
    .expect("handoff should validate against source reports");
    assert_json_round_trip(&handoff);
}

#[test]
fn external_lifecycle_handoff_validation_rejects_stale_source() {
    let (lifecycle_plan, pending_report) = sample_external_lifecycle_pending_artifacts();
    let proposal_report = ExternalUpgradeProposalReportV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        report_id: pending_report.proposal_report_id.clone(),
        report_digest: pending_report.proposal_report_digest.clone(),
        lifecycle_plan_id: lifecycle_plan.lifecycle_plan_id.clone(),
        lifecycle_plan_digest: lifecycle_plan.lifecycle_plan_digest.clone(),
        deployment_plan_id: lifecycle_plan.deployment_plan_id.clone(),
        deployment_plan_digest: lifecycle_plan.deployment_plan_digest.clone(),
        inventory_id: lifecycle_plan.inventory_id.clone(),
        proposals: Vec::new(),
        blocked_subjects: Vec::new(),
    };
    let mut lifecycle_check = external_lifecycle_check_from_reports(
        "external-lifecycle-check-1",
        &lifecycle_plan,
        &proposal_report,
        &pending_report,
    );
    let handoff = external_lifecycle_handoff_from_reports(
        "external-lifecycle-handoff-1",
        &lifecycle_check,
        &proposal_report,
        &pending_report,
    );
    lifecycle_check.check_id = "other-check".to_string();

    let err = validate_external_lifecycle_handoff_for_reports(
        &handoff,
        &lifecycle_check,
        &proposal_report,
        &pending_report,
    )
    .expect_err("stale handoff source should fail");

    assert_eq!(
        err,
        ExternalLifecycleHandoffError::SourceMismatch {
            field: "lifecycle_check_id"
        }
    );
}

#[test]
fn external_lifecycle_handoff_text_reports_passive_boundary() {
    let (lifecycle_plan, pending_report) = sample_external_lifecycle_pending_artifacts();
    let proposal_report = ExternalUpgradeProposalReportV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        report_id: pending_report.proposal_report_id.clone(),
        report_digest: pending_report.proposal_report_digest.clone(),
        lifecycle_plan_id: lifecycle_plan.lifecycle_plan_id.clone(),
        lifecycle_plan_digest: lifecycle_plan.lifecycle_plan_digest.clone(),
        deployment_plan_id: lifecycle_plan.deployment_plan_id.clone(),
        deployment_plan_digest: lifecycle_plan.deployment_plan_digest.clone(),
        inventory_id: lifecycle_plan.inventory_id.clone(),
        proposals: Vec::new(),
        blocked_subjects: Vec::new(),
    };
    let lifecycle_check = external_lifecycle_check_from_reports(
        "external-lifecycle-check-1",
        &lifecycle_plan,
        &proposal_report,
        &pending_report,
    );
    let handoff = external_lifecycle_handoff_from_reports(
        "external-lifecycle-handoff-1",
        &lifecycle_check,
        &proposal_report,
        &pending_report,
    );

    let text = external_lifecycle_handoff_text(&handoff);

    assert!(text.contains("External lifecycle handoff"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("execution: none"));
    assert!(text.contains("handoff_actions"));
}

#[test]
fn critical_external_fix_report_summarizes_residual_exposure() {
    let (lifecycle_plan, pending_report) = sample_external_lifecycle_pending_artifacts();

    let report = critical_external_fix_report_from_pending(
        "critical-fix-1",
        "fix-2026-05-critical",
        "critical",
        &lifecycle_plan,
        &pending_report,
    );

    assert_eq!(report.fix_id, "fix-2026-05-critical");
    assert_eq!(report.severity, "critical");
    assert_eq!(report.lifecycle_plan_id, lifecycle_plan.lifecycle_plan_id);
    assert_eq!(report.pending_report_id, pending_report.report_id);
    assert_eq!(report.affected_roles, vec!["root"]);
    assert_eq!(report.affected_canisters, vec!["aaaaa-aa"]);
    assert!(report.directly_patchable_roles.is_empty());
    assert_eq!(report.externally_blocked_roles, vec!["root"]);
    assert!(report.dependency_blocked_roles.is_empty());
    assert_eq!(report.required_external_actions.len(), 1);
    assert!(!report.protected_call_implications.is_empty());
    assert!(!report.residual_exposure.is_empty());
    assert!(
        report
            .operator_next_steps
            .iter()
            .any(|step| step.contains("external consent"))
    );
    validate_critical_external_fix_report(&report).expect("critical fix report should validate");
    validate_critical_external_fix_report_for_pending(&report, &lifecycle_plan, &pending_report)
        .expect("critical fix report should validate against source artifacts");
    assert_json_round_trip(&report);
}

#[test]
fn critical_external_fix_report_validation_rejects_stale_source() {
    let (lifecycle_plan, mut pending_report) = sample_external_lifecycle_pending_artifacts();
    let report = critical_external_fix_report_from_pending(
        "critical-fix-1",
        "fix-2026-05-critical",
        "critical",
        &lifecycle_plan,
        &pending_report,
    );

    pending_report.report_id = "other-pending-report".to_string();
    let err = validate_critical_external_fix_report_for_pending(
        &report,
        &lifecycle_plan,
        &pending_report,
    )
    .expect_err("stale source should fail");

    assert_eq!(
        err,
        CriticalExternalFixReportError::SourceMismatch {
            field: "pending_report_id"
        }
    );
}

#[test]
fn critical_external_fix_report_text_reports_passive_boundary() {
    let (lifecycle_plan, pending_report) = sample_external_lifecycle_pending_artifacts();
    let report = critical_external_fix_report_from_pending(
        "critical-fix-1",
        "fix-2026-05-critical",
        "critical",
        &lifecycle_plan,
        &pending_report,
    );

    let text = critical_external_fix_report_text(&report);

    assert!(text.contains("Critical external fix report"));
    assert!(text.contains("mode: passive"));
    assert!(text.contains("execution: none"));
    assert!(text.contains("externally_blocked_roles: 1"));
    assert!(text.contains("residual_exposure"));
    assert!(text.contains("operator_next_steps"));
}
