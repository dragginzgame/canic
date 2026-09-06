use super::*;
use crate::fleet_ensure::{
    model::{
        CanisterPlan, CurrentFleetProtocolAction, CycleConservation, FLEET_ENSURE_SCHEMA_VERSION,
        FleetEnsureCompletion, FleetEnsureContinuationAuthority, ReviewedDesiredFleetRecord,
    },
    ops::{read_journal, read_state},
    tests::{MockError, MockPlatform},
};
use canic_core::cdk::types::Cycles;
use std::collections::BTreeMap;

const ROOT: &str = "rrkah-fqaaa-aaaaa-aaaaq-cai";

fn fixture() -> (
    DesiredFleet,
    FleetEnsurePlan,
    FleetEnsurePlan,
    FleetEnsureJournalRecord,
) {
    let desired: DesiredFleet = serde_json::from_value(serde_json::json!({
        "canisters": [], "cycles_ledger": "um5iw-rqaaa-aaaaq-qaaba-cai",
        "environment": "local", "fleet": "fleet", "ledger_fee_cycles": "0",
        "management_creation_fee_cycles": "0", "material_cycle_threshold": "1",
        "maximum_observation_burn_cycles": "1", "maximum_stalled_observations": 2,
        "maximum_update_burn_cycles": "2", "operator": ROOT,
        "schema_version": FLEET_ENSURE_SCHEMA_VERSION, "treasury": "root"
    }))
    .expect("bounded continuation policy");
    let conservation = CycleConservation {
        estate_funding_domains: Vec::new(),
        expected_post_operation_cycles: 900,
        maximum_execution_burn_cycles: 100,
        maximum_new_funding_cycles: 0,
        maximum_operator_debit_cycles: 0,
        maximum_unavoidable_fee_cycles: 0,
        observed_controlled_cycles: 1000,
        retained_in_reused_canisters_cycles: 1000,
        scheduled_transfer_cycles: 0,
    };
    let mut original = FleetEnsurePlan {
        canisters: vec![CanisterPlan {
            actions: Vec::new(),
            disposition: CanisterDisposition::Reuse,
            name: "root".into(),
            observed_cycles: 1000,
            principal: Some(ROOT.into()),
        }],
        continuation: Some(FleetEnsureContinuationAuthority {
            app_config_sha256: "11".repeat(32),
            application_artifact_union_sha256: "22".repeat(32),
            coordinator_candid_sha256: "33".repeat(32),
            maximum_successor_actions: 2,
            root_candid_sha256: "44".repeat(32),
            store_candid_sha256: "55".repeat(32),
        }),
        conservation,
        desired_sha256: "66".repeat(32),
        environment: "local".into(),
        fleet: "fleet".into(),
        operation_id: "77".repeat(32),
        plan_sha256: String::new(),
        planned_at_time: 1,
        protocol_actions: Vec::new(),
        root_reinstall_bindings: Vec::new(),
        root_start_authority: None,
        reviewed_desired: Some(Box::new(ReviewedDesiredFleetRecord::capture(&desired))),
        schema_version: FLEET_ENSURE_SCHEMA_VERSION,
        scope: FleetEnsurePlanScope::Full,
        terminal_inventory_operation_id: None,
    };
    original.plan_sha256 = expected_plan_sha256(&original);
    let mut phase = original.clone();
    phase.continuation = None;
    phase.protocol_actions.push(EnsureAction::FleetProtocol {
        action: Box::new(CurrentFleetProtocolAction::ObservePoolReadiness {
            minimum_ready: 1,
            readiness_floor: Cycles::new(5),
        }),
        candid: "root.did".into(),
        candid_sha256: "44".repeat(32),
        maximum_execution_burn_cycles: 0,
        name: "readiness".into(),
        principal: ROOT.into(),
    });
    phase.plan_sha256 = expected_plan_sha256(&phase);
    let journal = FleetEnsureJournalRecord {
        successor_phases: Vec::new(),
        completion: FleetEnsureCompletion::InProgress,
        estate_funding_required: None,
        effects: Vec::new(),
        fleet: original.fleet.clone(),
        initial_controlled_cycles: 1000,
        initial_estate_funding_cycles_by_root: BTreeMap::new(),
        initial_operator_cycles: 2000,
        operation_id: original.operation_id.clone(),
        plan_sha256: original.plan_sha256.clone(),
        schema_version: FLEET_ENSURE_SCHEMA_VERSION,
        stalled_observations: 0,
    };
    (desired, original, phase, journal)
}

fn observation() -> FleetObservation {
    FleetObservation {
        additional_controlled_cycles: BTreeMap::from([(ROOT.into(), 950)]),
        canisters: BTreeMap::new(),
        estate_funding_domains: BTreeMap::new(),
        ledger_fee_cycles: 0,
        operator_cycles: 2000,
        protocol_ready: BTreeMap::new(),
    }
}

#[test]
fn phase_is_durable_before_intent_and_hydrates_without_inline_actions_in_journal() {
    let (desired, original, phase, mut journal) = fixture();
    let root = crate::test_support::temp_dir("continuation-durable-phase");
    std::fs::create_dir_all(&root).expect("create fixture directory");
    let paths = EnsurePaths::under(&root, "local", "fleet");
    let mut state = read_state(&paths, "fleet").expect("empty current state");
    state.principals.insert("root".into(), ROOT.into());
    let mut platform = MockPlatform::new(desired.clone(), []);
    platform.set_fresh_protocol_actions(phase.protocol_actions.clone());
    append(
        &paths,
        &desired,
        &original,
        &mut journal,
        &state,
        &observation(),
        phase.clone(),
        &mut platform,
    )
    .expect("bounded canonical successor");
    assert!(journal.effects.is_empty());
    assert_eq!(journal.plan_sha256, original.plan_sha256);
    assert_eq!(journal.initial_controlled_cycles, 1000);
    let restored = read_journal(&paths)
        .expect("read persisted phase")
        .expect("journal");
    assert_eq!(restored, journal);
    assert_eq!(restored.successor_phases[0].execution_burn_before_phase, 50);
    assert_eq!(restored.successor_phases[0].plan.as_deref(), Some(&phase));
    verify_records::<MockError>(&original, &restored).expect("exact restored authority");
    let json: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&paths.journal).expect("journal bytes"))
            .expect("journal JSON");
    assert!(json["successor_phases"][0].get("plan").is_none());
    std::fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn insufficient_remaining_budget_rejects_before_phase_or_intent_persistence() {
    let (desired, mut original, phase, mut journal) = fixture();
    original.conservation.maximum_execution_burn_cycles = 52;
    original.plan_sha256 = expected_plan_sha256(&original);
    journal.plan_sha256 = original.plan_sha256.clone();
    let root = crate::test_support::temp_dir("continuation-budget");
    std::fs::create_dir_all(&root).expect("create fixture directory");
    let paths = EnsurePaths::under(&root, "local", "fleet");
    let mut state = read_state(&paths, "fleet").expect("empty current state");
    state.principals.insert("root".into(), ROOT.into());
    let mut platform = MockPlatform::new(desired.clone(), []);
    platform.set_fresh_protocol_actions(phase.protocol_actions.clone());
    let error = append(
        &paths,
        &desired,
        &original,
        &mut journal,
        &state,
        &observation(),
        phase,
        &mut platform,
    )
    .expect_err("past burn consumes the original shared budget");
    assert!(matches!(
        error,
        EnsureWorkflowError::SuccessorReviewRequired {
            reason: FleetEnsureSuccessorReviewReason::BudgetExceeded
        }
    ));
    assert!(journal.successor_phases.is_empty());
    assert!(journal.effects.is_empty());
    assert!(!paths.journal.exists());
    std::fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn a_rehashed_successor_cannot_add_a_canister_effect_or_change_protocol_authority() {
    for additional_effect in [false, true] {
        let (desired, original, mut phase, mut journal) = fixture();
        let root = crate::test_support::temp_dir("continuation-altered-authority");
        std::fs::create_dir_all(&root).expect("create fixture directory");
        let paths = EnsurePaths::under(&root, "local", "fleet");
        let mut state = read_state(&paths, "fleet").expect("empty current state");
        state.principals.insert("root".into(), ROOT.into());
        let mut platform = MockPlatform::new(desired.clone(), []);
        platform.set_fresh_protocol_actions(phase.protocol_actions.clone());
        if additional_effect {
            phase.canisters[0].actions.push(EnsureAction::Start {
                name: "root".into(),
                principal: ROOT.into(),
            });
        } else if let EnsureAction::FleetProtocol { action, .. } = &mut phase.protocol_actions[0] {
            **action = CurrentFleetProtocolAction::ObservePoolReadiness {
                minimum_ready: 2,
                readiness_floor: Cycles::new(5),
            };
        }
        phase.plan_sha256 = expected_plan_sha256(&phase);
        let error = append(
            &paths,
            &desired,
            &original,
            &mut journal,
            &state,
            &observation(),
            phase,
            &mut platform,
        )
        .expect_err("a new digest grants no new authority");
        let expected = if additional_effect {
            FleetEnsureSuccessorReviewReason::AdditionalEffect
        } else {
            FleetEnsureSuccessorReviewReason::ProtocolAuthority
        };
        assert!(
            matches!(error, EnsureWorkflowError::SuccessorReviewRequired { reason } if reason == expected)
        );
        assert!(journal.successor_phases.is_empty());
        assert!(!paths.journal.exists());
        std::fs::remove_dir_all(root).expect("remove fixture");
    }
}

#[test]
fn successor_rejects_changed_principal_and_exhausted_phase_authority() {
    for exceeded in [false, true] {
        let (desired, mut original, mut phase, mut journal) = fixture();
        let root = crate::test_support::temp_dir("continuation-exact-principals");
        std::fs::create_dir_all(&root).expect("create fixture directory");
        let paths = EnsurePaths::under(&root, "local", "fleet");
        let mut state = read_state(&paths, "fleet").expect("empty current state");
        state.principals.insert("root".into(), ROOT.into());
        let mut platform = MockPlatform::new(desired.clone(), []);
        platform.set_fresh_protocol_actions(phase.protocol_actions.clone());
        if exceeded {
            original
                .continuation
                .as_mut()
                .expect("authority")
                .maximum_successor_actions = 0;
            original.plan_sha256 = expected_plan_sha256(&original);
            journal.plan_sha256 = original.plan_sha256.clone();
        } else {
            phase.canisters[0].principal = Some("aaaaa-aa".into());
            phase.plan_sha256 = expected_plan_sha256(&phase);
        }
        let error = append(
            &paths,
            &desired,
            &original,
            &mut journal,
            &state,
            &observation(),
            phase,
            &mut platform,
        )
        .expect_err("reviewed authority cannot widen");
        let expected = if exceeded {
            FleetEnsureSuccessorReviewReason::PhaseBound
        } else {
            FleetEnsureSuccessorReviewReason::ProtocolAuthority
        };
        assert!(
            matches!(error, EnsureWorkflowError::SuccessorReviewRequired { reason } if reason == expected)
        );
        assert!(journal.successor_phases.is_empty());
        assert!(!paths.journal.exists());
        std::fs::remove_dir_all(root).expect("remove fixture");
    }
}

#[test]
fn new_review_does_not_revalidate_the_previous_inactive_plans_phases() {
    let (desired, original, phase, journal) = fixture();
    let mut journal = candidate_journal(&journal, &phase, 50);
    let mut successor = original.clone();
    successor.operation_id = "88".repeat(32);
    successor.continuation = None;
    successor.plan_sha256 = expected_plan_sha256(&successor);
    let root = crate::test_support::temp_dir("continuation-new-review");
    let paths = EnsurePaths::under(&root, "local", "fleet");
    let state = read_state(&paths, "fleet").unwrap();
    let mut platform = MockPlatform::new(desired, []);
    for completion in [
        FleetEnsureCompletion::ReplanRequired,
        FleetEnsureCompletion::Converged,
    ] {
        journal.completion = completion;
        verify_canonical(&successor, &journal, &state, &mut platform)
            .expect("a separately reviewed plan does not inherit prior phases");
        assert!(matches!(
            verify_canonical(&original, &journal, &state, &mut platform),
            Err(EnsureWorkflowError::SuccessorReviewRequired {
                reason: FleetEnsureSuccessorReviewReason::ProtocolAuthority
            })
        ));
    }
    journal.completion = FleetEnsureCompletion::InProgress;
    assert!(matches!(
        verify_canonical(&successor, &journal, &state, &mut platform),
        Err(EnsureWorkflowError::JournalIntegrity)
    ));
}
