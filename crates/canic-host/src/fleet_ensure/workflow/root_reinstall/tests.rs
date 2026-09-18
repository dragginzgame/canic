//! Management-only funding admission and per-effect authority regressions.

use super::*;
use crate::fleet_ensure::{
    model::{
        CanisterDisposition, CanisterPlan, LiveCanister, ReviewedDesiredFleetRecord,
        RootManagementBinding, RootManagementCanisterObservation, RootManagementObservation,
    },
    tests::{Fixture, protocol_tranche_fixture},
    view::OperatorFundingObservation,
};
use std::collections::BTreeMap;

fn fixture() -> (Fixture, FleetEnsurePlan, FleetEnsureStateRecord) {
    let mut fixture = protocol_tranche_fixture(Vec::new());
    let desired = &fixture.desired;
    let binding = RootManagementBinding {
        name: "root".into(),
        principal: "ryjl3-tyaaa-aaaaa-aaaba-cai".into(),
        controllers: vec![desired.operator.clone()],
        module_sha256: "71".repeat(32),
        subnet: "subnet".into(),
    };
    let mut plan = crate::fleet_ensure::workflow::tests::estate_funding_plan();
    plan.scope = FleetEnsurePlanScope::RootReinstallPrerequisite;
    plan.root_start_authority = None;
    plan.continuation = None;
    plan.protocol_actions.clear();
    plan.reinstall = None;
    plan.planned_at_time = 101;
    plan.reviewed_desired = Some(Box::new(ReviewedDesiredFleetRecord::capture(desired)));
    plan.conservation.maximum_new_funding_cycles = 100;
    let fee = desired
        .ledger_fee_cycles
        .parse::<Cycles>()
        .unwrap()
        .to_u128();
    plan.conservation.maximum_operator_debit_cycles = 100 + fee;
    plan.conservation.maximum_unavoidable_fee_cycles = fee;
    plan.canisters = vec![CanisterPlan {
        name: binding.name.clone(),
        principal: Some(binding.principal.clone()),
        observed_cycles: 500,
        disposition: CanisterDisposition::Reinstall,
        actions: vec![
            EnsureAction::Stop {
                name: binding.name.clone(),
                principal: binding.principal.clone(),
            },
            EnsureAction::Fund {
                name: binding.name.clone(),
                principal: binding.principal.clone(),
                ledger: desired.cycles_ledger.clone(),
                created_at_time: 101,
                amount: 100,
                expected_post_cycles: 600,
                funding_deficit_cycles: 80,
                funding_margin_cycles: 20,
                pool_funding: None,
            },
            EnsureAction::Install {
                name: binding.name.clone(),
                principal: binding.principal.clone(),
                mode: InstallMode::Reinstall,
                canic_init: None,
                reinstall_witness: None,
                init_arg: None,
                init_arg_sha256: None,
                init_candid: None,
                init_candid_sha256: None,
                wasm: "root.wasm".into(),
                wasm_sha256: "81".repeat(32),
            },
            EnsureAction::Start {
                name: binding.name.clone(),
                principal: binding.principal.clone(),
            },
        ],
    }];
    fixture.platform.root_management = Some(RootManagementObservation {
        operator_cycles: 100 + fee,
        roots: BTreeMap::from([(
            binding.name.clone(),
            RootManagementCanisterObservation {
                name: binding.name.clone(),
                subnet: binding.subnet.clone(),
                live: LiveCanister {
                    principal: binding.principal.clone(),
                    controllers: binding.controllers.clone(),
                    module_sha256: Some(binding.module_sha256.clone()),
                    status: CanisterRuntimeStatus::Stopped,
                    cycles: 500,
                    canister_version: Some(1),
                    reinstall_required: true,
                    root_owned_lifecycle: None,
                },
            },
        )]),
    });
    fixture.platform.operator_funding = Some(OperatorFundingObservation {
        cycles_ledger: desired.cycles_ledger.clone(),
        ledger_fee_cycles: fee,
        operator_cycles: 100 + fee,
    });
    plan.root_reinstall_bindings = vec![binding];
    let paths =
        crate::fleet_ensure::ops::EnsurePaths::under(&fixture.root, &plan.environment, &plan.fleet);
    let state = crate::fleet_ensure::ops::read_state(&paths, &plan.fleet).unwrap();
    (fixture, plan, state)
}

#[test]
fn initial_funding_requires_exact_ledger_fee_and_complete_operator_debit() {
    let (mut fixture, plan, _) = fixture();
    let initial = plan.conservation.maximum_operator_debit_cycles;
    verify_initial_funding(&plan, initial, &mut fixture.platform).unwrap();
    fixture
        .platform
        .operator_funding
        .as_mut()
        .unwrap()
        .operator_cycles = initial - 1;
    assert!(matches!(
        verify_initial_funding(&plan, initial - 1, &mut fixture.platform),
        Err(EnsureWorkflowError::InsufficientOperatorCycles { .. })
    ));
    assert!(matches!(
        verify_initial_funding(&plan, initial, &mut fixture.platform),
        Err(EnsureWorkflowError::DriftedBeforeApply)
    ));
    fixture
        .platform
        .operator_funding
        .as_mut()
        .unwrap()
        .operator_cycles = initial;
    fixture
        .platform
        .operator_funding
        .as_mut()
        .unwrap()
        .ledger_fee_cycles += 1;
    assert!(matches!(
        verify_initial_funding(&plan, initial, &mut fixture.platform),
        Err(EnsureWorkflowError::DriftedBeforeApply)
    ));
    fixture
        .platform
        .operator_funding
        .as_mut()
        .unwrap()
        .ledger_fee_cycles -= 1;
    fixture
        .platform
        .operator_funding
        .as_mut()
        .unwrap()
        .cycles_ledger = "different-ledger".into();
    assert!(matches!(
        verify_initial_funding(&plan, initial, &mut fixture.platform),
        Err(EnsureWorkflowError::DriftedBeforeApply)
    ));
}

#[test]
fn payment_and_install_require_stopped_exact_source_authority() {
    for index in [1, 2] {
        for drift in [
            "running",
            "stopping",
            "controller",
            "module",
            "principal",
            "subnet",
        ] {
            let (mut fixture, plan, state) = fixture();
            let action = &plan.canisters[0].actions[index];
            verify_effect_authority(&plan, action, Some(500), &state, &mut fixture.platform)
                .unwrap();
            let observed = fixture
                .platform
                .root_management
                .as_mut()
                .unwrap()
                .roots
                .get_mut("root")
                .unwrap();
            match drift {
                "running" => observed.live.status = CanisterRuntimeStatus::Running,
                "stopping" => observed.live.status = CanisterRuntimeStatus::Stopping,
                "controller" => observed.live.controllers.push("foreign".into()),
                "module" => observed.live.module_sha256 = Some("91".repeat(32)),
                "principal" => observed.live.principal = "foreign".into(),
                "subnet" => observed.subnet = "foreign".into(),
                _ => unreachable!(),
            }
            assert!(
                matches!(
                    verify_effect_authority(
                        &plan,
                        action,
                        Some(500),
                        &state,
                        &mut fixture.platform
                    ),
                    Err(EnsureWorkflowError::DriftedBeforeApply)
                ),
                "{drift}"
            );
            assert!(fixture.platform.seal_reads.is_empty());
        }
    }
}

#[test]
fn payment_retry_checks_fee_without_requiring_already_spent_operator_balance() {
    let (mut fixture, plan, state) = fixture();
    fixture
        .platform
        .operator_funding
        .as_mut()
        .unwrap()
        .operator_cycles = 0;
    let action = &plan.canisters[0].actions[1];
    verify_effect_authority(&plan, action, Some(500), &state, &mut fixture.platform).unwrap();
    fixture
        .platform
        .operator_funding
        .as_mut()
        .unwrap()
        .ledger_fee_cycles += 1;
    assert!(matches!(
        verify_effect_authority(&plan, action, Some(500), &state, &mut fixture.platform),
        Err(EnsureWorkflowError::DriftedBeforeApply)
    ));
}

#[test]
fn reset_rejects_payment_outside_stop_install_and_start_requires_new_module() {
    let (mut fixture, mut plan, state) = fixture();
    let start = plan.canisters[0].actions[3].clone();
    assert!(matches!(
        verify_effect_authority(&plan, &start, Some(500), &state, &mut fixture.platform),
        Err(EnsureWorkflowError::DriftedBeforeApply)
    ));
    fixture
        .platform
        .root_management
        .as_mut()
        .unwrap()
        .roots
        .get_mut("root")
        .unwrap()
        .live
        .module_sha256 = Some("81".repeat(32));
    verify_effect_authority(&plan, &start, Some(500), &state, &mut fixture.platform).unwrap();
    for (left, right) in [(0, 1), (1, 2), (2, 3)] {
        plan.canisters[0].actions.swap(left, right);
        assert!(matches!(
            targets::<std::io::Error>(&plan),
            Err(EnsureWorkflowError::PlanIntegrity)
        ));
        plan.canisters[0].actions.swap(left, right);
    }
    if let EnsureAction::Fund { principal, .. } = &mut plan.canisters[0].actions[1] {
        *principal = "foreign".into();
    }
    assert!(matches!(
        targets::<std::io::Error>(&plan),
        Err(EnsureWorkflowError::PlanIntegrity)
    ));
}

#[test]
fn payment_rejects_new_burn_beyond_the_reviewed_margin() {
    let (mut fixture, plan, state) = fixture();
    fixture
        .platform
        .root_management
        .as_mut()
        .unwrap()
        .roots
        .get_mut("root")
        .unwrap()
        .live
        .cycles = 479;
    assert!(matches!(
        verify_effect_authority(
            &plan,
            &plan.canisters[0].actions[1],
            Some(500),
            &state,
            &mut fixture.platform
        ),
        Err(EnsureWorkflowError::DriftedBeforeApply)
    ));
}

#[test]
fn payment_rejects_inconsistent_budget_before_stop() {
    let (mut fixture, mut plan, _) = fixture();
    let initial = plan.conservation.maximum_operator_debit_cycles;
    for field in ["funding", "fee", "debit"] {
        let original = plan.conservation.clone();
        match field {
            "funding" => plan.conservation.maximum_new_funding_cycles = 0,
            "fee" => plan.conservation.maximum_unavoidable_fee_cycles += 1,
            "debit" => plan.conservation.maximum_operator_debit_cycles += 1,
            _ => unreachable!(),
        }
        assert!(matches!(
            verify_initial_funding(&plan, initial, &mut fixture.platform),
            Err(EnsureWorkflowError::PlanIntegrity)
        ));
        plan.conservation = original;
    }
}

#[test]
fn payment_requires_retained_prebalance_that_can_reconcile() {
    let (mut fixture, plan, state) = fixture();
    let action = &plan.canisters[0].actions[1];
    assert!(matches!(
        verify_effect_authority(&plan, action, None, &state, &mut fixture.platform),
        Err(EnsureWorkflowError::JournalIntegrity)
    ));
    verify_effect_authority(&plan, action, Some(501), &state, &mut fixture.platform)
        .expect("donation before intent preserves the reviewed payment authority");
    assert!(matches!(
        verify_effect_authority(&plan, action, Some(479), &state, &mut fixture.platform),
        Err(EnsureWorkflowError::DriftedBeforeApply)
    ));
    fixture
        .platform
        .root_management
        .as_mut()
        .unwrap()
        .roots
        .get_mut("root")
        .unwrap()
        .live
        .cycles = 600;
    verify_effect_authority(&plan, action, Some(500), &state, &mut fixture.platform).unwrap();
}

#[test]
fn root_review_accepts_native_surplus_only_with_the_same_action_authority() {
    let (fixture, plan, _) = fixture();
    let mut donated = plan.clone();
    donated.canisters[0].observed_cycles = 10_000;
    assert!(compatible_root_start_prerequisite(
        &plan,
        &donated,
        &fixture.desired
    ));
    donated.canisters[0].actions.remove(1);
    assert!(!compatible_root_start_prerequisite(
        &plan,
        &donated,
        &fixture.desired
    ));
}
