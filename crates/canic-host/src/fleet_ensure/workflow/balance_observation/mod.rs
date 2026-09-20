//! Native balance observations do not authenticate deposits or measure gross burn.

use crate::fleet_ensure::{
    model::{FleetEnsureCompletion, FleetEnsurePlanScope, FleetObservation},
    workflow::{
        EnsureWorkflowError,
        tests::{estate_funding_plan, retained_evidence},
        verify_terminal_conservation,
    },
};
use std::{collections::BTreeMap, io};

#[test]
fn donations_are_net_surplus_in_every_scope_without_rewriting_the_baseline() {
    for scope in [
        FleetEnsurePlanScope::Full,
        FleetEnsurePlanScope::ReinstallPreparation,
        FleetEnsurePlanScope::RootReinstallPrerequisite,
        FleetEnsurePlanScope::RootStartPrerequisite,
    ] {
        let mut plan = estate_funding_plan();
        plan.scope = scope;
        plan.canisters.clear();
        plan.protocol_actions.clear();
        plan.conservation.estate_funding_domains.clear();
        plan.conservation.maximum_operator_debit_cycles = 0;
        plan.conservation.maximum_new_funding_cycles = 0;
        plan.conservation.maximum_unavoidable_fee_cycles = 0;
        plan.conservation.maximum_execution_burn_cycles = 2;
        let (state, mut journal) = retained_evidence();
        journal.effects.clear();
        journal.initial_controlled_cycles = 100;
        journal.initial_operator_cycles = 1_000;
        journal.initial_estate_funding_cycles_by_root.clear();
        let bytes = serde_json::to_vec(&journal).unwrap();
        for (balance, deficit, surplus) in
            [(98, 2, 0), (100, 0, 0), (108, 0, 8), (10_000, 0, 9_900)]
        {
            let observation = observation(balance);
            for _ in 0..2 {
                let actual = verify_terminal_conservation::<io::Error>(
                    &plan,
                    &journal,
                    &state,
                    &observation,
                )
                .unwrap();
                assert_eq!(actual.observed_net_cycle_debit_cycles, deficit);
                assert_eq!(actual.observed_net_cycle_credit_cycles, surplus);
                assert_eq!(actual.received_new_funding_cycles, 0);
                assert_eq!(actual.operator_debit_cycles, 0);
                assert_eq!(actual.observed_starting_cycles, 100);
                assert_eq!(
                    actual.observed_starting_cycles + surplus,
                    actual.final_controlled_cycles + deficit
                );
            }
        }
        assert_eq!(serde_json::to_vec(&journal).unwrap(), bytes);
        assert!(matches!(
            verify_terminal_conservation::<io::Error>(&plan, &journal, &state, &observation(97)),
            Err(EnsureWorkflowError::Conservation(_))
        ));
        // Even an enormous native donation cannot cover an unauthorized operator debit.
        let mut changed = observation(10_000);
        changed.operator_cycles = 999;
        assert!(matches!(
            verify_terminal_conservation::<io::Error>(&plan, &journal, &state, &changed),
            Err(EnsureWorkflowError::Conservation(_))
        ));
    }
}

fn observation(cycles: u128) -> FleetObservation {
    FleetObservation {
        additional_controlled_cycles: BTreeMap::from([("root".into(), cycles)]),
        canisters: BTreeMap::new(),
        estate_funding_domains: BTreeMap::new(),
        ledger_fee_cycles: 0,
        operator_cycles: 1_000,
        protocol_ready: BTreeMap::new(),
    }
}

#[test]
fn terminal_account_diagnostic_preserves_in_progress_conservation_and_receipts() {
    let mut plan = estate_funding_plan();
    plan.conservation.maximum_operator_debit_cycles = 100;
    let (state, mut journal) = retained_evidence();
    journal.initial_operator_cycles = 1_000;
    for completion in [
        FleetEnsureCompletion::InProgress,
        FleetEnsureCompletion::Prepared,
        FleetEnsureCompletion::Converged,
    ] {
        journal.completion = completion;
        let retained = serde_json::to_vec(&journal).unwrap();
        for balance in [899, 1_001] {
            let mut terminal = observation(100);
            terminal.operator_cycles = balance;
            let error =
                verify_terminal_conservation::<io::Error>(&plan, &journal, &state, &terminal)
                    .unwrap_err();
            if completion == FleetEnsureCompletion::Converged {
                assert!(
                    matches!(error, EnsureWorkflowError::TerminalReplayBalanceChanged {
                    current_operator_cycles,
                    minimum_operator_cycles: 900,
                    operator_source_cycles: 1_000,
                    ..
                } if current_operator_cycles == balance)
                );
            } else {
                assert!(matches!(error, EnsureWorkflowError::Conservation(_)));
            }
            assert_eq!(serde_json::to_vec(&journal).unwrap(), retained);
        }
    }
}
