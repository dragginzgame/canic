//! Exact receipt credit and terminal conservation regressions for activation recovery.

use super::*;
use crate::fleet_ensure::{
    model::*,
    workflow::{
        tests::{estate_funding_observation, estate_funding_plan, retained_evidence},
        verify_terminal_conservation,
    },
};

#[test]
fn terminal_credit_preserves_start_and_burn_bounds_on_repeat_verification() {
    for scope in [
        FleetEnsurePlanScope::ReinstallPreparation,
        FleetEnsurePlanScope::RootReinstallPrerequisite,
    ] {
        let (mut plan, mut journal) = fixture(scope);
        let (state, _) = retained_evidence();
        let mut terminal = estate_funding_observation(None);
        terminal.estate_funding_domains.clear();
        terminal
            .additional_controlled_cycles
            .insert("root".into(), 108);
        let before = serde_json::to_vec(&journal).unwrap();
        for _ in 0..2 {
            let actual =
                verify_terminal_conservation::<std::io::Error>(&plan, &journal, &state, &terminal)
                    .unwrap();
            assert_eq!(actual.observed_starting_cycles, 100);
            assert_eq!(actual.observed_settlement_credit_cycles, 10);
            assert_eq!(actual.measured_execution_burn_cycles, 2);
            assert_eq!(actual.received_new_funding_cycles, 0);
        }
        assert_eq!(serde_json::to_vec(&journal).unwrap(), before);
        terminal
            .additional_controlled_cycles
            .insert("root".into(), 111);
        assert!(matches!(
            verify_terminal_conservation::<std::io::Error>(&plan, &journal, &state, &terminal),
            Err(EnsureWorkflowError::Conservation(_))
        ));
        terminal
            .additional_controlled_cycles
            .insert("root".into(), 107);
        assert!(matches!(
            verify_terminal_conservation::<std::io::Error>(&plan, &journal, &state, &terminal),
            Err(EnsureWorkflowError::Conservation(_))
        ));
        terminal
            .additional_controlled_cycles
            .insert("root".into(), 108);
        plan.scope = FleetEnsurePlanScope::Full;
        assert!(matches!(
            verify_terminal_conservation::<std::io::Error>(&plan, &journal, &state, &terminal),
            Err(EnsureWorkflowError::Conservation(_))
        ));
        plan.scope = scope;
        journal.initial_controlled_cycles = u128::MAX;
        assert!(matches!(
            verify_terminal_conservation::<std::io::Error>(&plan, &journal, &state, &terminal),
            Err(EnsureWorkflowError::Conservation(_))
        ));
    }
}

#[test]
fn terminal_credit_rejects_incomplete_or_extra_receipts() {
    let (plan, mut journal) = fixture(FleetEnsurePlanScope::ReinstallPreparation);
    journal.effects[0].post_cycles = None;
    assert!(matches!(
        observed_credit::<std::io::Error>(&plan, &journal),
        Err(EnsureWorkflowError::JournalIntegrity)
    ));
    journal.effects[0].post_cycles = Some(110);
    journal.effects.push(journal.effects[0].clone());
    assert!(matches!(
        observed_credit::<std::io::Error>(&plan, &journal),
        Err(EnsureWorkflowError::JournalIntegrity)
    ));
    journal.effects.clear();
    assert!(matches!(
        observed_credit::<std::io::Error>(&plan, &journal),
        Err(EnsureWorkflowError::JournalIntegrity)
    ));
}

fn fixture(scope: FleetEnsurePlanScope) -> (FleetEnsurePlan, FleetEnsureJournalRecord) {
    let action = EnsureAction::Stop {
        name: "root".into(),
        principal: "rrkah-fqaaa-aaaaa-aaaaq-cai".into(),
    };
    let desired: DesiredFleet = serde_json::from_value(serde_json::json!({
        "canisters": [], "cycles_ledger": "um5iw-rqaaa-aaaaq-qaaba-cai", "environment": "local", "fleet": "fleet",
        "ledger_fee_cycles": "0B", "management_creation_fee_cycles": "0B",
        "material_cycle_threshold": "1B", "maximum_observation_burn_cycles": "0.000000010B",
        "maximum_stalled_observations": 2, "maximum_update_burn_cycles": "2B",
        "operator": "rrkah-fqaaa-aaaaa-aaaaq-cai", "schema_version": 1, "treasury": "root"
    })).unwrap();
    let mut plan = estate_funding_plan();
    plan.scope = scope;
    plan.protocol_actions = vec![action.clone()];
    plan.conservation.estate_funding_domains.clear();
    plan.conservation.maximum_execution_burn_cycles = 2;
    plan.reviewed_desired = Some(Box::new(ReviewedDesiredFleetRecord::capture(&desired)));
    plan.reinstall = Some(Box::new(FleetReinstallRecord {
        target_artifacts_sha256: None,
        source: None,
        operation_id: "operation".into(),
        source_operation_id: "source".into(),
        authorities: vec![],
        assets: vec![],
        activation_reset: Some(Box::new(FleetActivationResetRecord {
            preparation: None,
            roots: vec![],
            source: FleetActivationSourceRecord {
                infrastructure: vec![],
                operator: desired.operator,
                cycles_ledger: desired.cycles_ledger,
                initial_controlled_cycles: 100,
                maximum_execution_burn_cycles: 2,
                initial_estate_funding_cycles_by_root: std::collections::BTreeMap::new(),
                operation_id: "source".into(),
                plan_sha256: "a".repeat(64),
                plan_document_sha256: "b".repeat(64),
                journal_document_sha256: "c".repeat(64),
                state_document_sha256: "d".repeat(64),
                provisioning: action.clone(),
                registry_preparations: vec![],
                stores: vec![],
            },
        })),
    }));
    let (_, mut journal) = retained_evidence();
    journal.initial_controlled_cycles = 100;
    journal.effects = vec![receipt(&action, 100, 110)];
    (plan, journal)
}

#[test]
fn settlement_credit_is_scoped_to_source_bound_activation_recovery() {
    for scope in [
        FleetEnsurePlanScope::ReinstallPreparation,
        FleetEnsurePlanScope::RootReinstallPrerequisite,
    ] {
        assert!(eligible_scope(scope, true));
        assert!(!eligible_scope(scope, false));
    }
    assert!(!eligible_scope(FleetEnsurePlanScope::Full, true));
    assert!(!eligible_scope(
        FleetEnsurePlanScope::RootStartPrerequisite,
        true
    ));
}

#[test]
fn settlement_install_movement_grants_no_credit() {
    let install = EnsureAction::Install {
        canic_init: None,
        reinstall_witness: None,
        init_arg: None,
        init_arg_sha256: None,
        init_candid: None,
        init_candid_sha256: None,
        mode: crate::fleet_ensure::model::InstallMode::Reinstall,
        name: "root".to_string(),
        principal: "rrkah-fqaaa-aaaaa-aaaaq-cai".to_string(),
        wasm: "root.wasm".to_string(),
        wasm_sha256: "a".repeat(64),
    };
    assert_eq!(
        receipt_credit::<std::io::Error>(&install, &receipt(&install, 100, 110), 10).unwrap(),
        0
    );
}

#[test]
fn preparation_stop_credit_requires_exact_applied_receipt_and_bound() {
    let action = EnsureAction::Stop {
        name: "root".to_string(),
        principal: "rrkah-fqaaa-aaaaa-aaaaq-cai".to_string(),
    };
    let mut effect = receipt(&action, 100, 110);
    assert_eq!(
        receipt_credit::<std::io::Error>(&action, &effect, 10).unwrap(),
        10
    );
    assert!(matches!(
        receipt_credit::<std::io::Error>(&action, &effect, 9),
        Err(EnsureWorkflowError::Conservation(_))
    ));
    effect.state = EffectState::Issued;
    assert!(matches!(
        receipt_credit::<std::io::Error>(&action, &effect, 10),
        Err(EnsureWorkflowError::JournalIntegrity)
    ));
    effect.state = EffectState::Applied;
    effect.pre_cycles = None;
    assert!(matches!(
        receipt_credit::<std::io::Error>(&action, &effect, 10),
        Err(EnsureWorkflowError::JournalIntegrity)
    ));
    effect.pre_cycles = Some(100);
    effect.action_sha256 = "0".repeat(64);
    assert!(matches!(
        receipt_credit::<std::io::Error>(&action, &effect, 10),
        Err(EnsureWorkflowError::JournalIntegrity)
    ));
}

#[test]
fn preparation_stop_debits_and_start_movements_grant_no_credit() {
    let stop = EnsureAction::Stop {
        name: "root".to_string(),
        principal: "rrkah-fqaaa-aaaaa-aaaaq-cai".to_string(),
    };
    assert_eq!(
        receipt_credit::<std::io::Error>(&stop, &receipt(&stop, 100, 90), 10).unwrap(),
        0
    );
    let start = EnsureAction::Start {
        name: "root".to_string(),
        principal: "rrkah-fqaaa-aaaaa-aaaaq-cai".to_string(),
    };
    assert_eq!(
        receipt_credit::<std::io::Error>(&start, &receipt(&start, 100, 110), 10).unwrap(),
        0
    );
}

fn receipt(action: &EnsureAction, before: u128, after: u128) -> EffectRecord {
    EffectRecord {
        maintenance_attempts: 0,
        publication_attempts: 0,
        action_sha256: action_sha256(action),
        created_principal: None,
        destination_post_cycles: None,
        destination_pre_cycles: None,
        post_cycles: Some(after),
        pre_cycles: Some(before),
        pre_canister_version: None,
        progress_identity: None,
        receipt: None,
        state: EffectState::Applied,
    }
}

#[test]
fn native_funding_is_not_counted_again_as_stop_settlement_credit() {
    let action = EnsureAction::Fund {
        name: "root".into(),
        principal: "rrkah-fqaaa-aaaaa-aaaaq-cai".into(),
        ledger: "um5iw-rqaaa-aaaaq-qaaba-cai".into(),
        created_at_time: 1,
        amount: 100,
        expected_post_cycles: 200,
        funding_deficit_cycles: 90,
        funding_margin_cycles: 10,
        pool_funding: None,
    };
    assert_eq!(
        receipt_credit::<std::io::Error>(&action, &receipt(&action, 100, 200), 10).unwrap(),
        0
    );
}
