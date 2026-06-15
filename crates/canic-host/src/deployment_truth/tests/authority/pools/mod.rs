use super::super::*;

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
