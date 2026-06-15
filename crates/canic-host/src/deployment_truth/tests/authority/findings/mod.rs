use super::super::*;

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
