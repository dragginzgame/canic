//! Supplementary budgets preserve paid evidence and reject altered amounts or replenished attempts.

use super::*;

pub(in crate::fleet_ensure::ops) fn qualify(
    plan: &FleetEnsurePlan,
    journal: &FleetEnsureJournalRecord,
    phase: &FleetEnsurePlan,
    state: &FleetEnsureStateRecord,
    observed: &InfrastructureBootstrapObservation,
) {
    let root = crate::test_support::temp_dir("bootstrap-registration-recovery");
    let paths = EnsurePaths::under(&root, &plan.environment, &plan.fleet);
    let before_plan = serde_json::to_vec(plan).unwrap();
    let before_effects = journal.effects.clone();
    let mut journal = journal.clone();
    let mut observed = observed.clone();
    observed.operator_cycles =
        journal.initial_operator_cycles - plan.conservation.maximum_operator_debit_cycles;
    reserve(&paths, &mut journal, false).unwrap();
    let review = candidate(
        plan,
        &journal,
        phase,
        &observed,
        0,
        plan.planned_at_time + 100,
    )
    .unwrap();
    retain(&paths, plan, state, &mut journal, review.clone()).unwrap();
    let stored = std::fs::read(&paths.journal).unwrap();
    let projection: serde_json::Value = serde_json::from_slice(&stored).unwrap();
    for action in projection["bootstrap_registration_recovery"]["review"]["protocol_actions"]
        .as_array()
        .unwrap()
    {
        assert!(action.pointer("/action/request/bytes").is_none());
    }
    assert_eq!(
        crate::fleet_ensure::ops::read_journal(&paths)
            .unwrap()
            .unwrap(),
        journal
    );
    assert!(
        stored.len() < 8 * 1024 * 1024,
        "bootstrap import's bounded journal reader must remain usable"
    );
    verify(plan, &journal, state).unwrap();
    assert!(funding_actions(&journal).is_empty());
    assert_eq!(conservation(plan, &journal).unwrap(), plan.conservation);
    reserve(&paths, &mut journal, true).unwrap();
    approve(&paths, &mut journal).unwrap();
    verify(plan, &journal, state).unwrap();
    assert_eq!(journal.effects, before_effects);
    assert_eq!(serde_json::to_vec(plan).unwrap(), before_plan);
    assert_eq!(
        journal.initial_controlled_cycles,
        plan.conservation.observed_controlled_cycles
    );
    let bounds = conservation(plan, &journal).unwrap();
    assert!(
        bounds.maximum_execution_burn_cycles
            >= review.successor_burn_cycles + review.recovery_burn_cycles
    );
    assert_eq!(
        bounds.maximum_new_funding_cycles,
        plan.conservation.maximum_new_funding_cycles + review.additional_funding_cycles
    );
    let bytes = serde_json::to_vec(&journal).unwrap();
    let restored = serde_json::from_slice(&bytes).unwrap();
    verify(plan, &restored, state).unwrap();
    assert_eq!(journal, restored);
    qualify_funding_targets(&review, &observed);
    let mut changed = journal.clone();
    let body = changed
        .bootstrap_registration_recovery
        .as_mut()
        .unwrap()
        .review
        .as_mut()
        .unwrap();
    body.maximum_execution_burn_cycles += 1;
    body.review_sha256 = digest(body).unwrap();
    assert!(matches!(
        verify(plan, &changed, state),
        Err(InfrastructureBootstrapError::Integrity)
    ));
    let mut changed = journal.clone();
    changed.effects[0].pre_canister_version = Some(999);
    assert!(matches!(
        verify(plan, &changed, state),
        Err(InfrastructureBootstrapError::Integrity)
    ));
    reserve(&paths, &mut journal, true).unwrap();
    assert!(matches!(
        reserve(&paths, &mut journal, true),
        Err(InfrastructureBootstrapError::InspectionBudget)
    ));
    assert_eq!(
        journal
            .bootstrap_registration_recovery
            .as_ref()
            .unwrap()
            .approval_attempts,
        RECOVERY_INSPECTION_ROUNDS
    );

    qualify_budget_shortfalls(phase);
}

fn qualify_funding_targets(
    review: &BootstrapRegistrationReviewRecord,
    observed: &InfrastructureBootstrapObservation,
) {
    for action in &review.funding_actions {
        let EnsureAction::Fund {
            amount,
            expected_post_cycles,
            funding_deficit_cycles,
            funding_margin_cycles,
            name,
            ..
        } = action
        else {
            unreachable!()
        };
        let sample = &review.canisters[name];
        assert!(*funding_margin_cycles > 0);
        let proof = crate::fleet_ensure::ops::NativeFundingObservation {
            amount: *amount,
            expected_post_cycles: *expected_post_cycles,
            funding_deficit_cycles: *funding_deficit_cycles,
            funding_margin_cycles: *funding_margin_cycles,
            live_cycles: Some(expected_post_cycles - funding_margin_cycles),
            pre_cycles: Some(sample.cycles - 1),
        };
        assert!(crate::fleet_ensure::ops::native_funding_applied(proof));
        let mut live = super::super::fleet_observation(observed)
            .canisters
            .remove(name)
            .unwrap()
            .unwrap();
        verify_funding_target(review, action, Some(&live)).unwrap();
        live.controllers.clear();
        assert!(matches!(
            verify_funding_target(review, action, Some(&live)),
            Err(InfrastructureBootstrapError::Integrity)
        ));
    }
}

fn qualify_budget_shortfalls(phase: &FleetEnsurePlan) {
    let mut underfunded = phase.clone();
    underfunded.conservation.observed_controlled_cycles = 193_920_000_000_000;
    underfunded.conservation.maximum_execution_burn_cycles = 237_000_000_000_000;
    assert!(matches!(
        registration::require_budget(&underfunded, 154_000_000_000_000),
        Err(InfrastructureBootstrapError::RegistrationBudget {
            funding_shortfall_cycles: 43_080_000_000_000,
            execution_shortfall_cycles: 83_000_000_000_000,
            ..
        })
    ));
    underfunded.conservation.observed_controlled_cycles = 300_000_000_000_000;
    assert!(matches!(
        registration::require_budget(&underfunded, 154_000_000_000_000),
        Err(InfrastructureBootstrapError::RegistrationBudget {
            funding_shortfall_cycles: 0,
            execution_shortfall_cycles: 83_000_000_000_000,
            ..
        })
    ));
    registration::require_budget(&underfunded, 237_000_000_000_000).unwrap();
}
