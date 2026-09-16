use super::*;
use crate::fleet_ensure::{
    model::{
        DesiredCanisterInit, DesiredCanisterKind, LiveCanister, ReviewedDesiredFleetRecord,
        RootManagementCanisterObservation,
    },
    tests::{Fixture, protocol_tranche_fixture},
};
use std::{collections::BTreeMap, fs};

fn fixture(
    kind: DesiredCanisterKind,
) -> (
    Fixture,
    FleetEnsurePlan,
    FleetEnsureStateRecord,
    EnsureAction,
) {
    let mut fixture = protocol_tranche_fixture(Vec::new());
    let mut root = fixture.desired.canisters[0].clone();
    root.kind = DesiredCanisterKind::Root;
    root.name = "root".into();
    root.principal = Some("ryjl3-tyaaa-aaaaa-aaaba-cai".into());
    root.parent = Some(fixture.desired.canisters[0].name.clone());
    root.canic_init = Some(DesiredCanisterInit::Root {
        root: root.name.clone(),
    });
    fixture.desired.canisters.push(root);
    let hash = canic_core::cdk::utils::hash::sha256_hex(b"current-wasm");
    let authorities = fixture
        .desired
        .canisters
        .iter()
        .map(|configured| {
            (
                configured.name.clone(),
                RootManagementCanisterObservation {
                    name: configured.name.clone(),
                    subnet: configured.subnet.clone(),
                    live: LiveCanister {
                        canister_version: Some(1),
                        controllers: configured.controllers.clone(),
                        cycles: 500,
                        module_sha256: Some(hash.clone()),
                        principal: configured.principal.clone().unwrap(),
                        reinstall_required: true,
                        root_owned_lifecycle: None,
                        status: CanisterRuntimeStatus::Running,
                    },
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    let source = FleetReinstallSourceRecord {
        reviewed_desired: ReviewedDesiredFleetRecord::capture(&fixture.desired),
        wasm_sha256_by_canister: authorities
            .keys()
            .map(|name| (name.clone(), hash.clone()))
            .collect(),
        candid_sha256_by_path: BTreeMap::from([
            ("root.did".into(), "31".repeat(32)),
            ("coordinator.did".into(), "32".repeat(32)),
        ]),
    };
    // This unit fixture targets the per-effect guard, not full plan admission.
    let mut plan = crate::fleet_ensure::workflow::tests::estate_funding_plan();
    plan.operation_id = "71".repeat(32);
    let bindings = authorities
        .values()
        .map(|observed| capture::authority_binding(observed).unwrap())
        .collect::<Vec<_>>();
    let target = fixture
        .desired
        .canisters
        .iter()
        .find(|canister| canister.kind == kind)
        .unwrap();
    let binding = bindings
        .iter()
        .find(|binding| binding.name == target.name)
        .unwrap();
    let seal = capture::funding_seal(&source, binding, kind).unwrap();
    plan.canisters = vec![crate::fleet_ensure::model::CanisterPlan {
        actions: vec![EnsureAction::Install {
            canic_init: None,
            reinstall_witness: None,
            init_arg: None,
            init_arg_sha256: None,
            init_candid: None,
            init_candid_sha256: None,
            mode: crate::fleet_ensure::model::InstallMode::Reinstall,
            name: binding.name.clone(),
            principal: binding.principal.clone(),
            wasm: "selected.wasm".into(),
            wasm_sha256: "81".repeat(32),
        }],
        disposition: crate::fleet_ensure::model::CanisterDisposition::Reinstall,
        name: binding.name.clone(),
        observed_cycles: 500,
        principal: Some(binding.principal.clone()),
    }];
    plan.reinstall = Some(Box::new(FleetReinstallRecord {
        target_artifacts_sha256: Some("41".repeat(32)),
        source: Some(Box::new(source)),
        activation_reset: None,
        operation_id: plan.operation_id.clone(),
        source_operation_id: "51".repeat(32),
        authorities: bindings,
        assets: Vec::new(),
    }));
    fixture.platform.reinstall_authority = Some(authorities);
    fixture.platform.seal_identity = Some((plan.operation_id.clone(), action_sha256(&seal)));
    let paths = EnsurePaths::under(&fixture.root, &plan.environment, &plan.fleet);
    let state = read_state(&paths, &plan.fleet).unwrap();
    (fixture, plan, state, seal)
}

fn credits(seal: &EnsureAction) -> [EnsureAction; 2] {
    let EnsureAction::SealAuthority {
        name, principal, ..
    } = seal
    else {
        panic!("seal fixture")
    };
    [
        EnsureAction::Fund {
            amount: 100,
            created_at_time: 101,
            expected_post_cycles: 600,
            funding_deficit_cycles: 100,
            funding_margin_cycles: 0,
            ledger: "ledger".into(),
            name: name.clone(),
            pool_funding: None,
            principal: principal.clone(),
        },
        EnsureAction::FundEstate {
            amount: 100,
            created_at_time: 102,
            expected_post_cycles: 600,
            ledger: "ledger".into(),
            ledger_fee_cycles: 1,
            name: name.clone(),
            principal: principal.clone(),
        },
    ]
}

#[test]
fn funding_rechecks_exact_seal_on_every_attempt() {
    for kind in [DesiredCanisterKind::Root, DesiredCanisterKind::Coordinator] {
        let (mut fixture, plan, state, seal) = fixture(kind);
        for credit in credits(&seal) {
            verify_effect_authority(&plan, &credit, &state, &mut fixture.platform).unwrap();
            fixture.platform.seal_identity = None;
            assert!(matches!(
                verify_effect_authority(&plan, &credit, &state, &mut fixture.platform),
                Err(EnsureWorkflowError::DriftedBeforeApply)
            ));
            fixture.platform.seal_identity =
                Some(("foreign-operation".into(), action_sha256(&seal)));
            assert!(matches!(
                verify_effect_authority(&plan, &credit, &state, &mut fixture.platform),
                Err(EnsureWorkflowError::DriftedBeforeApply)
            ));
            fixture.platform.seal_identity =
                Some((plan.operation_id.clone(), action_sha256(&seal)));
            verify_effect_authority(&plan, &credit, &state, &mut fixture.platform).unwrap();
        }
        assert!(
            fixture
                .platform
                .seal_reads
                .iter()
                .all(|read| read == &(plan.operation_id.clone(), seal.clone()))
        );
        fs::remove_dir_all(fixture.root).unwrap();
    }
}

#[test]
fn funding_rejects_management_drift_before_using_source_protocol() {
    let (mut fixture, plan, state, seal) = fixture(DesiredCanisterKind::Root);
    let original = fixture.platform.reinstall_authority.clone().unwrap();
    for change in 0..6 {
        let mut authorities = original.clone();
        let observed = authorities.values_mut().next().unwrap();
        match change {
            0 => observed.live.controllers.clear(),
            1 => observed.live.module_sha256 = Some("foreign-module".into()),
            2 => observed.live.principal = "foreign-principal".into(),
            3 => observed.subnet = "foreign-subnet".into(),
            4 => observed.name = "foreign-name".into(),
            _ => observed.live.status = CanisterRuntimeStatus::Stopped,
        }
        fixture.platform.reinstall_authority = Some(authorities);
        for credit in credits(&seal) {
            assert!(matches!(
                verify_effect_authority(&plan, &credit, &state, &mut fixture.platform),
                Err(EnsureWorkflowError::ConvergenceDrift)
            ));
        }
    }
    assert!(fixture.platform.seal_reads.is_empty());
    fs::remove_dir_all(fixture.root).unwrap();
}

#[test]
fn funding_requires_bound_source_interface_and_exact_recipient() {
    let (mut fixture, mut plan, state, seal) = fixture(DesiredCanisterKind::Root);
    for mut credit in credits(&seal) {
        match &mut credit {
            EnsureAction::Fund { principal, .. } | EnsureAction::FundEstate { principal, .. } => {
                *principal = "foreign-recipient".into();
            }
            _ => unreachable!(),
        }
        assert!(matches!(
            verify_effect_authority(&plan, &credit, &state, &mut fixture.platform),
            Err(EnsureWorkflowError::ConvergenceDrift)
        ));
    }
    plan.reinstall
        .as_mut()
        .unwrap()
        .source
        .as_mut()
        .unwrap()
        .candid_sha256_by_path
        .clear();
    assert!(matches!(
        verify_effect_authority(&plan, &credits(&seal)[0], &state, &mut fixture.platform),
        Err(EnsureWorkflowError::PlanIntegrity)
    ));
    assert!(fixture.platform.seal_reads.is_empty());
    fs::remove_dir_all(fixture.root).unwrap();
}

#[test]
fn funding_protection_budget_and_scope_are_explicit() {
    let (mut fixture, mut plan, state, seal) = fixture(DesiredCanisterKind::Root);
    plan.canisters
        .iter_mut()
        .find(|canister| canister.name == seal.name())
        .unwrap()
        .actions
        .extend(credits(&seal));
    assert_eq!(
        policy::funding_observation_count(
            &fixture.desired,
            plan.reinstall.as_deref(),
            &plan.canisters
        )
        .unwrap(),
        12
    );
    assert_eq!(
        policy::funding_observation_count(&fixture.desired, None, &plan.canisters).unwrap(),
        0
    );
    // A management-only prerequisite has no retained source protocol authority.
    plan.reinstall.as_mut().unwrap().source = None;
    for credit in credits(&seal) {
        verify_effect_authority(&plan, &credit, &state, &mut fixture.platform).unwrap();
    }
    assert!(fixture.platform.seal_reads.is_empty());
    assert_eq!(
        policy::funding_observation_count(
            &fixture.desired,
            plan.reinstall.as_deref(),
            &plan.canisters
        )
        .unwrap(),
        0
    );
    fs::remove_dir_all(fixture.root).unwrap();
}

#[test]
fn successor_funding_does_not_query_the_replaced_source_seal() {
    let (mut fixture, mut plan, state, seal) = fixture(DesiredCanisterKind::Root);
    plan.canisters[0].actions = credits(&seal).to_vec();
    fixture.platform.reinstall_authority = None;
    fixture.platform.seal_identity = None;
    for credit in credits(&seal) {
        verify_effect_authority(&plan, &credit, &state, &mut fixture.platform).unwrap();
    }
    assert!(fixture.platform.seal_reads.is_empty());
    assert_eq!(
        policy::funding_observation_count(
            &fixture.desired,
            plan.reinstall.as_deref(),
            &plan.canisters
        )
        .unwrap(),
        0
    );
    fs::remove_dir_all(fixture.root).unwrap();
}
