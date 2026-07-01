use super::super::*;
use crate::deployment_truth::authority::AUTHORITY_PROFILE_OVERLAP_CODE;

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
            .all(|finding| finding.code == AUTHORITY_PROFILE_OVERLAP_CODE
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
