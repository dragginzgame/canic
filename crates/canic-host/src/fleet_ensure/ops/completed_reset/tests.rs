//! Completed-reset accounting and local terminal replay. No IC admission or installation is faked.

use super::*;
use crate::fleet_ensure::{
    model::{
        ActualCycleConservation, CanisterRuntimeStatus,
        completed_handoff::{
            CompletedCanisterCustodyRecord, CompletedEstateCustodyRecord,
            CompletedPhysicalBindingRecord,
            preparation::{
                CompletedPreparationBalanceRecord, CompletedPreparationInspectionRecord,
                CompletedSourceAccountingRecord,
            },
        },
    },
    policy::reinstall::completed::conserved,
    view::completed_reset::CompletedResetBalancesView,
};
use candid::Principal;

fn balance(native_cycles: u128, reserved_cycles: u128) -> CompletedPreparationBalanceRecord {
    CompletedPreparationBalanceRecord {
        status: CanisterRuntimeStatus::Running,
        native_cycles,
        reserved_cycles,
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "one local accounting fixture keeps the original and current cycle domains visible together"
)]
fn accounting() -> (
    CompletedEstateResetRecord,
    CompletedResetBalancesView,
    ActualCycleConservation,
) {
    let root = Principal::from_slice(&[1]);
    let pool = Principal::from_slice(&[2]);
    let operator = Principal::from_slice(&[3]);
    let custody = CompletedEstateCustodyRecord {
        operator,
        network: "11".repeat(32).parse().unwrap(),
        canisters: [("root", root), ("pool", pool)]
            .into_iter()
            .map(|(name, principal)| {
                (
                    name.into(),
                    CompletedCanisterCustodyRecord {
                        binding: CompletedPhysicalBindingRecord {
                            principal,
                            subnet: root.into(),
                            controllers: vec![operator],
                            module_sha256: None,
                        },
                        certificate_tree_sha256: [0; 32],
                    },
                )
            })
            .collect(),
    };
    let review = CompletedPreparationReviewRecord {
        schema_version: 1,
        cli_release: env!("CARGO_PKG_VERSION").into(),
        environment: "local".into(),
        fleet: "test-fleet".into(),
        operation_id: "11".repeat(32),
        source: crate::fleet_ensure::model::FleetTerminalSourceRecord {
            operation_id: "22".repeat(32),
            plan_sha256: "33".repeat(32),
            plan_document_sha256: "44".repeat(32),
            journal_document_sha256: "55".repeat(32),
            state_document_sha256: "66".repeat(32),
            phase_document_sha256: BTreeMap::new(),
        },
        custody,
        source_artifacts: BTreeMap::new(),
        actions: Vec::new(),
        inspection_roots: BTreeMap::from([
            ("root".into(), None),
            ("pool".into(), Some("root".into())),
        ]),
        source_accounting: CompletedSourceAccountingRecord {
            initial_native_cycles: 2000,
            recorded_funding_cycles: 0,
            maximum_source_burn_cycles: 200,
        },
        maximum_attempts_per_action: 2,
        maximum_observations_per_action: 4,
        maximum_execution_burn_cycles: 100,
        review_sha256: "77".repeat(32),
    };
    let prepared = CompletedPreparationJournalRecord {
        schema_version: 1,
        review_sha256: review.review_sha256.clone(),
        effects: Vec::new(),
        prepared: true,
        inspections: ["root", "pool"]
            .into_iter()
            .map(|name| {
                (
                    name.into(),
                    CompletedPreparationInspectionRecord {
                        attempts: 1,
                        balance: Some(balance(1000, 75)),
                    },
                )
            })
            .collect(),
    };
    let source = CompletedEstateResetRecord {
        maximum_terminal_observations: 2,
        preparation: review,
        prepared,
        source_names: BTreeMap::from([
            ("new-root".into(), "root".into()),
            ("new-pool".into(), "pool".into()),
        ]),
        operator_cycles: 1000,
        root_ledger_cycles: BTreeMap::from([("new-root".into(), 100)]),
        other_ledger_cycles: BTreeMap::from([(pool.to_text(), 50)]),
    };
    let balances = CompletedResetBalancesView {
        canisters: BTreeMap::from([
            (root.to_text(), balance(950, 75)),
            (pool.to_text(), balance(1000, 75)),
        ]),
        ledger: BTreeMap::from([(root.to_text(), 100), (pool.to_text(), 50)]),
        operator_cycles: 1000,
    };
    let actual = ActualCycleConservation {
        observed_starting_cycles: 2100,
        final_controlled_cycles: 2050,
        observed_net_cycle_debit_cycles: 50,
        observed_net_cycle_credit_cycles: 0,
        operator_debit_cycles: 0,
        received_new_funding_cycles: 0,
        exact_unavoidable_fee_cycles: 0,
        exact_estate_creation_fee_cycles: 0,
        estate_funding_cycles: 0,
    };
    (source, balances, actual)
}

#[test]
fn completed_reset_accounts_for_exact_native_reserved_and_ledger_domains() {
    let (source, balances, actual) = accounting();
    assert!(conserved(&source, &balances, &actual, 100, 1000));
    let mut missing = balances.clone();
    missing.ledger.pop_first();
    assert!(!conserved(&source, &missing, &actual, 100, 1000));
    let mut lost_reserve = balances.clone();
    for balance in lost_reserve.canisters.values_mut() {
        balance.reserved_cycles = 0;
    }
    assert!(!conserved(&source, &lost_reserve, &actual, 100, 1000));
    let mut debit = balances.clone();
    debit.operator_cycles -= 1;
    assert!(!conserved(&source, &debit, &actual, 100, 1000));
    let mut unreviewed_account = balances;
    *unreviewed_account
        .ledger
        .get_mut(&Principal::from_slice(&[2]).to_text())
        .unwrap() -= 1;
    assert!(!conserved(&source, &unreviewed_account, &actual, 100, 1000));
}

#[test]
fn completed_reset_cannot_rebase_source_loss_or_overflow() {
    let (mut source, balances, actual) = accounting();
    source.preparation.source_accounting.initial_native_cycles = 2400;
    assert!(!conserved(&source, &balances, &actual, 100, 1000));
    source.preparation.source_accounting.initial_native_cycles = u128::MAX;
    assert!(!conserved(&source, &balances, &actual, u128::MAX, 1000));
    let (source, balances, mut actual) = accounting();
    actual.final_controlled_cycles += 1;
    assert!(!conserved(&source, &balances, &actual, 100, 1000));
}

#[test]
fn completed_reset_terminal_receipt_replays_locally_and_rejects_changed_evidence() {
    let (fixture, paths, mut plan) = crate::fleet_ensure::tests::terminal_retirement_fixture();
    let (source, balances, actual) = accounting();
    plan.reinstall = Some(Box::new(FleetReinstallRecord {
        completed_reset: Some(Box::new(source)),
        source: None,
        activation_reset: None,
        target_artifacts_sha256: None,
        operation_id: plan.operation_id.clone(),
        source_operation_id: "22".repeat(32),
        authorities: Vec::new(),
        assets: Vec::new(),
    }));
    plan.protocol_actions.clear();
    plan.plan_sha256 = policy::expected_plan_sha256(&plan);
    let mut journal: FleetEnsureJournalRecord = ops::read_current(&paths.journal).unwrap().unwrap();
    journal.initial_controlled_cycles = actual.observed_starting_cycles;
    journal.plan_sha256 = plan.plan_sha256.clone();
    journal.effects.clear();
    journal.successor_phases.clear();
    journal.completion = FleetEnsureCompletion::Converged;
    ops::write_plan(&paths, &plan).unwrap();
    ops::write_journal(&paths, &journal).unwrap();
    // Interruption after journal publication has no terminal receipt yet.
    assert_eq!(terminal::read(&paths, &plan, &journal).unwrap(), None);
    terminal::consume(&paths, &plan).unwrap();
    // Reopening consumes the remaining attempt, rather than recreating the budget.
    terminal::consume(&paths, &plan).unwrap();
    let attempts =
        std::fs::read(paths.plan.with_file_name("completed-reset-accounting.json")).unwrap();
    assert!(matches!(
        terminal::consume(&paths, &plan),
        Err(EnsureStateError::CompletedResetAccountingBudget)
    ));
    assert_eq!(
        std::fs::read(paths.plan.with_file_name("completed-reset-accounting.json")).unwrap(),
        attempts
    );
    terminal::retain(&paths, &plan, &actual, balances).unwrap();
    let receipt =
        std::fs::read(paths.plan.with_file_name("completed-reset-terminal.json")).unwrap();
    for _ in 0..2 {
        let mut platform = ops::IcpEnsurePlatform::new(
            fixture.desired.clone(),
            "/no-icp-terminal-replay",
            &fixture.root,
        );
        let result = crate::fleet_ensure::workflow::apply(
            &fixture.root,
            &fixture.desired,
            &plan.desired_sha256,
            &plan.fleet,
            &plan.plan_sha256,
            &mut platform,
        )
        .unwrap();
        assert_eq!(result.effects_applied, 0);
        assert_eq!(result.actual_conservation, Some(actual.clone()));
        assert_eq!(
            std::fs::read(paths.plan.with_file_name("completed-reset-terminal.json")).unwrap(),
            receipt
        );
    }
    let mut state: FleetEnsureStateRecord = ops::read_current(&paths.state).unwrap().unwrap();
    state
        .principals
        .insert("unreviewed".into(), Principal::from_slice(&[9]).to_text());
    ops::write_state(&paths, &state).unwrap();
    assert!(matches!(
        terminal::read(&paths, &plan, &journal),
        Err(EnsureStateError::CompletedHandoffConflict)
    ));
}
