use super::*;
use crate::fleet_ensure::model::{
    CanisterRuntimeStatus, EstateFundingDomainObservation, EstatePoolAssetLifecycle,
    EstatePoolAssetObservation, EstatePoolAssetOrigin, EstatePoolInventoryObservation,
    FleetEnsureStateRecord, LiveCanister, RootManagementCanisterObservation,
};

fn assert_root_reinstall_headroom(
    state: &FleetEnsureStateRecord,
    desired: &DesiredFleet,
    artifacts: &DesiredFleetArtifacts,
    management: &RootManagementObservation,
    root: &str,
) {
    let mut desired = desired.clone();
    desired.maximum_observation_burn_cycles = "1000000000000".into();
    desired.maximum_update_burn_cycles = "1000000000000".into();
    let required = 6_000_000_000_000;
    for available in [
        1_339_000_000_000,
        2_000_000_000_000,
        2_339_000_000_000,
        required - 1,
        required,
    ] {
        let mut management = management.clone();
        management.roots.get_mut(root).unwrap().live.cycles = available;
        let result = root_reinstall::compile(
            RootStartPlanInput {
                authority: None,
                state,
                desired: &desired,
                desired_sha256: "headroom-review",
                created_at_time: 1,
                requested_fleet: &desired.fleet,
                observation: &management,
            },
            artifacts,
        );
        if available < 2_000_000_000_000 {
            assert!(
                matches!(result, Err(EnsurePolicyError::RootReinstallHeadroom {
                name, principal, action_count: 1, available: observed, required: bound, shortfall,
            }) if name == root && principal == management.roots[root].live.principal
                && observed == available && bound == 2_000_000_000_000 && shortfall == bound - available)
            );
        } else {
            let plan = result.unwrap().unwrap();
            assert_eq!(plan.scope, FleetEnsurePlanScope::RootReinstallPrerequisite);
            let (burn, funding) = if available < required {
                let [
                    EnsureAction::Stop { .. },
                    EnsureAction::Fund {
                        amount,
                        funding_deficit_cycles,
                        funding_margin_cycles,
                        expected_post_cycles,
                        pool_funding: None,
                        ..
                    },
                    EnsureAction::Install { .. },
                    EnsureAction::Start { .. },
                ] = plan.canisters[0].actions.as_slice()
                else {
                    panic!("stopped Root funding sequence");
                };
                assert_eq!(*amount, 8_000_000_000_000 - available);
                assert_eq!(*funding_deficit_cycles, required - available);
                assert_eq!(*funding_margin_cycles, 2_000_000_000_000);
                assert_eq!(*expected_post_cycles, 8_000_000_000_000);
                (8_000_000_000_000, *amount)
            } else {
                assert_eq!(plan.canisters[0].actions.len(), 3);
                (required, 0)
            };
            assert_eq!(plan.conservation.maximum_execution_burn_cycles, burn);
            assert_eq!(plan.conservation.maximum_new_funding_cycles, funding);
            let fee = if funding == 0 {
                0
            } else {
                desired
                    .ledger_fee_cycles
                    .parse::<Cycles>()
                    .unwrap()
                    .to_u128()
            };
            assert_eq!(
                plan.conservation.maximum_operator_debit_cycles,
                funding + fee
            );
            assert_eq!(plan.conservation.maximum_unavoidable_fee_cycles, fee);
            assert_eq!(plan.conservation.expected_post_operation_cycles, 0);
            let review = plan.recovery_review.as_ref().unwrap();
            assert_eq!(review.maximum_successor_actions, 0);
            assert_eq!(review.whole_continuation_ceiling_cycles, 0);
            assert_eq!(review.per_step_burn_cycles, 4_000_000_000_000);
            for forecast in &review.startup_funding {
                assert_eq!(
                    forecast.continuation_allowance_cycles,
                    u128::from(forecast.maximum_continuation_steps) * review.per_step_burn_cycles
                );
                assert_eq!(
                    forecast.startup_minimum_cycles,
                    artifacts.startup_funding_by_root[&forecast.root].minimum_native_cycles
                );
                assert!(forecast.required_native_cycles > required);
            }
        }
    }
    assert_other_root_cannot_cover_reinstall(state, &desired, artifacts, management, root);
}

fn assert_other_root_cannot_cover_reinstall(
    state: &FleetEnsureStateRecord,
    desired: &DesiredFleet,
    artifacts: &DesiredFleetArtifacts,
    management: &RootManagementObservation,
    root: &str,
) {
    let mut desired = desired.clone();
    let mut artifacts = artifacts.clone();
    let mut management = management.clone();
    let mut other = desired
        .canisters
        .iter()
        .find(|c| c.name == root)
        .unwrap()
        .clone();
    other.name = "other-funded-root".into();
    other.principal = Some(candid::Principal::from_slice(&[98; 29]).to_text());
    other.canic_init = Some(crate::fleet_ensure::model::DesiredCanisterInit::Root {
        root: other.name.clone(),
    });
    let mut bootstrap_root = desired
        .bootstrap
        .as_ref()
        .unwrap()
        .roots
        .iter()
        .find(|entry| entry.root == root)
        .unwrap()
        .clone();
    let mut store = desired
        .canisters
        .iter()
        .find(|canister| canister.name == bootstrap_root.store)
        .unwrap()
        .clone();
    store.name = "other-store".into();
    store.principal = Some(candid::Principal::from_slice(&[99; 29]).to_text());
    store.parent = Some(other.name.clone());
    for controller in &mut store.controller_canisters {
        if controller == root {
            controller.clone_from(&other.name);
        }
    }
    store.canic_init = Some(crate::fleet_ensure::model::DesiredCanisterInit::Store {
        root: other.name.clone(),
    });
    bootstrap_root.root.clone_from(&other.name);
    bootstrap_root.store.clone_from(&store.name);
    bootstrap_root.canister_pool_imports.clear();
    desired
        .bootstrap
        .as_mut()
        .unwrap()
        .roots
        .push(bootstrap_root);
    let mut live = management.roots[root].clone();
    live.name.clone_from(&other.name);
    live.live.principal = other.principal.clone().unwrap();
    live.live.cycles = 100_000_000_000_000;
    management.roots.insert(other.name.clone(), live);
    management.roots.get_mut(root).unwrap().live.cycles = 1_339_000_000_000;
    for hashes in [
        &mut artifacts.wasm_sha256_by_canister,
        &mut artifacts.init_arg_sha256_by_canister,
        &mut artifacts.init_candid_sha256_by_canister,
    ] {
        if let Some(hash) = hashes.get(root).cloned() {
            hashes.insert(other.name.clone(), hash);
        }
    }
    desired.canisters.push(other);
    desired.canisters.push(store);
    let result = root_reinstall::compile(
        RootStartPlanInput {
            authority: None,
            state,
            desired: &desired,
            desired_sha256: "headroom-review",
            created_at_time: 1,
            requested_fleet: &desired.fleet,
            observation: &management,
        },
        &artifacts,
    );
    assert!(
        matches!(&result, Err(EnsurePolicyError::RootReinstallHeadroom {
        name, action_count: 1, available: 1_339_000_000_000, required: 2_000_000_000_000,
        shortfall: 661_000_000_000, ..
    }) if name == root),
        "unexpected multi-Root result: {result:?}"
    );
}

/// Exercise pure recovery admission with the maintained generator's complete desired contract.
#[expect(
    clippy::too_many_lines,
    reason = "one generated fixture covers preparation, reset review and nearest authority failures"
)]
pub(in crate::fleet_ensure) fn assert_activation_reset_reviews(
    generated: &DesiredFleet,
    artifacts: &DesiredFleetArtifacts,
) {
    let mut desired = generated.clone();
    let mut additional = desired
        .canisters
        .iter()
        .find(|c| c.kind == DesiredCanisterKind::Pool)
        .unwrap()
        .clone();
    additional.name = "extra-retained-pool".into();
    additional.principal = Some(candid::Principal::from_slice(&[97; 29]).to_text());
    let root = &mut desired.bootstrap.as_mut().unwrap().roots[0];
    root.canister_pool_imports.push(additional.name.clone());
    root.limits.canister_pool.maximum_size = 3;
    root.limits.canister_pool.minimum_size = 1;
    let root_name = root.root.clone();
    let pool_policy = root.limits.canister_pool.clone();
    desired.canisters.push(additional);
    let principals = desired
        .canisters
        .iter()
        .map(|c| (c.name.clone(), c.principal.clone().unwrap()))
        .collect::<BTreeMap<_, _>>();
    let mut observation = FleetObservation {
        additional_controlled_cycles: BTreeMap::new(),
        canisters: BTreeMap::new(),
        estate_funding_domains: BTreeMap::new(),
        ledger_fee_cycles: 100_000_000,
        operator_cycles: 0,
        protocol_ready: BTreeMap::new(),
    };
    let mut authorities = BTreeMap::new();
    let mut assets = Vec::new();
    let mut pool_assets = Vec::new();
    for canister in &desired.canisters {
        let principal = canister.principal.clone().unwrap();
        let mut controllers = canister.controllers.clone();
        controllers.extend(
            canister
                .controller_canisters
                .iter()
                .map(|name| principals[name].clone()),
        );
        controllers.sort();
        let live = LiveCanister {
            canister_version: Some(10),
            controllers: controllers.clone(),
            cycles: 100_000_000_000_000,
            module_sha256: Some("ee".repeat(32)),
            principal: principal.clone(),
            reinstall_required: false,
            root_owned_lifecycle: None,
            status: CanisterRuntimeStatus::Running,
        };
        if canister.kind == DesiredCanisterKind::Pool {
            assets.push(FleetReinstallAssetRecord {
                controllers,
                module_sha256: live.module_sha256.clone(),
                principal: principal.clone(),
                root: root_name.clone(),
                subnet: canister.subnet.clone(),
            });
            pool_assets.push(EstatePoolAssetObservation {
                creation_receipt: None,
                cycles: live.cycles,
                origin: EstatePoolAssetOrigin::Imported,
                principal,
                lifecycle: if pool_assets.len() < 2 {
                    EstatePoolAssetLifecycle::Workload
                } else {
                    EstatePoolAssetLifecycle::Ready
                },
            });
        } else {
            authorities.insert(
                canister.name.clone(),
                RootManagementCanisterObservation {
                    live: live.clone(),
                    name: canister.name.clone(),
                    subnet: canister.subnet.clone(),
                },
            );
        }
        observation
            .canisters
            .insert(canister.name.clone(), Some(live));
    }
    observation.estate_funding_domains.insert(
        root_name.clone(),
        EstateFundingDomainObservation {
            balance_cycles: Some(0),
            cycles_ledger: desired.cycles_ledger.clone(),
            root_principal: Some(principals[&root_name].clone()),
            pool: Some(EstatePoolInventoryObservation {
                assets: pool_assets,
                maximum_size: 3,
                minimum_size: 1,
                pending_creation: None,
                readiness_floor_cycles: pool_policy.canister_cycles.to_u128(),
                creation_execution_margin_cycles: pool_policy.creation_execution_margin.to_u128(),
            }),
        },
    );
    let source = FleetActivationSourceRecord {
        infrastructure: authorities
            .values()
            .map(|a| crate::fleet_ensure::ops::reinstall::authority_binding(a).unwrap())
            .collect(),
        operator: desired.operator.clone(),
        cycles_ledger: desired.cycles_ledger.clone(),
        initial_controlled_cycles: 600_000_000_000_000,
        maximum_execution_burn_cycles: 1_000_000_000_000,
        initial_estate_funding_cycles_by_root: BTreeMap::from([(root_name.clone(), 0)]),
        operation_id: "aa".repeat(32),
        plan_sha256: "ab".repeat(32),
        plan_document_sha256: "ac".repeat(32),
        journal_document_sha256: "ad".repeat(32),
        state_document_sha256: "ae".repeat(32),
        registry_preparations: Vec::new(),
        stores: Vec::new(),
        provisioning: EnsureAction::Stop {
            name: "opaque source evidence".into(),
            principal: principals[&root_name].clone(),
        },
    };
    let roots = vec![RootActivationResetRecord {
        root: root_name.clone(),
        activation_operation_id: [1; 32],
        inventory_hash: [2; 32],
        provisioning_receipt_hash: [3; 32],
        component_count: 2,
        managed_descendants: 0,
    }];
    let state = FleetEnsureStateRecord {
        active_registry: None,
        completed_reinstall_action_sha256: BTreeMap::new(),
        completed_reinstall_operation_id: None,
        completed_reinstalls: BTreeMap::new(),
        fleet: desired.fleet.clone(),
        pending_principals: BTreeMap::new(),
        principals,
        retained_cycles_by_principal: BTreeMap::new(),
        schema_version: 1,
        topology: BTreeMap::new(),
    };
    let management = RootManagementObservation {
        roots: authorities,
        operator_cycles: 0,
    };
    assert_root_reinstall_headroom(&state, &desired, artifacts, &management, &root_name);
    let compile = |source: &FleetActivationSourceRecord,
                   assets: &[FleetReinstallAssetRecord],
                   observation: &FleetObservation| {
        preparation(ActivationPreparationInput {
            root: RootStartPlanInput {
                authority: None,
                state: &state,
                desired: &desired,
                desired_sha256: &"bb".repeat(32),
                created_at_time: 1,
                requested_fleet: &desired.fleet,
                observation: &management,
            },
            artifacts,
            source,
            roots: &roots,
            assets,
            observation,
        })
    };
    let prepared = compile(&source, &assets, &observation).expect("closed source preparation");
    let actions = crate::fleet_ensure::workflow::ordered_actions(&prepared);
    assert!(matches!(
        actions.as_slice(),
        [
            EnsureAction::Stop { .. },
            EnsureAction::Stop { .. },
            EnsureAction::Start { .. }
        ]
    ));
    assert_eq!(prepared.conservation.maximum_operator_debit_cycles, 0);
    assert_eq!(
        prepared.conservation.observed_controlled_cycles,
        source.initial_controlled_cycles
    );
    assert!(matches!(
        compile(&source, &assets[..2], &observation),
        Err(EnsurePolicyError::RootManagementAuthorityMismatch { .. })
    ));
    let mut changed = source.clone();
    changed.infrastructure[0].module_sha256 = "ff".repeat(32);
    assert!(compile(&changed, &assets, &observation).is_err());
    changed = source.clone();
    changed.initial_controlled_cycles += source.maximum_execution_burn_cycles + 1;
    assert!(compile(&changed, &assets, &observation).is_err());
    changed = source.clone();
    changed.initial_controlled_cycles -= 1;
    assert!(compile(&changed, &assets, &observation).is_err());
    changed = source.clone();
    changed
        .initial_estate_funding_cycles_by_root
        .insert(root_name.clone(), 1);
    assert!(compile(&changed, &assets, &observation).is_err());
    let mut drifted = observation.clone();
    drifted
        .estate_funding_domains
        .get_mut(&root_name)
        .unwrap()
        .pool
        .as_mut()
        .unwrap()
        .maximum_size += 1;
    assert!(compile(&source, &assets, &drifted).is_err());

    let evidence = crate::fleet_ensure::model::ActivationPreparationEvidenceRecord {
        plan_document_sha256: "cc".repeat(32),
        journal_document_sha256: "cd".repeat(32),
    };
    let bounds = cycle_bounds(&desired).unwrap();
    let per_effect = bounds.observation_burn + bounds.update_burn;
    let mut depleted_management = management.clone();
    depleted_management
        .roots
        .get_mut(&root_name)
        .unwrap()
        .live
        .cycles = 2 * per_effect;
    let depleted_input = || RootStartPlanInput {
        authority: None,
        state: &state,
        desired: &desired,
        desired_sha256: &prepared.desired_sha256,
        created_at_time: 1,
        requested_fleet: &desired.fleet,
        observation: &depleted_management,
    };
    assert!(matches!(
        preparation(ActivationPreparationInput {
            root: depleted_input(),
            artifacts,
            source: &source,
            roots: &roots,
            assets: &assets,
            observation: &observation,
        }),
        Err(EnsurePolicyError::RootReinstallHeadroom {
            action_count: 3,
            ..
        })
    ));
    let mut depleted_observation = observation.clone();
    depleted_observation
        .canisters
        .get_mut(&root_name)
        .unwrap()
        .as_mut()
        .unwrap()
        .cycles = 2 * per_effect;
    let funded_reset = reset(
        depleted_input(),
        artifacts,
        &prepared,
        evidence.clone(),
        &depleted_observation,
    )
    .expect("prepared reset preserves its separately reviewed native funding");
    let amount: u128 = funded_reset
        .canisters
        .iter()
        .flat_map(|canister| &canister.actions)
        .filter_map(|action| match action {
            EnsureAction::Fund { amount, .. } => Some(*amount),
            _ => None,
        })
        .sum();
    assert_eq!(amount, 2 * per_effect);
    assert_eq!(funded_reset.conservation.maximum_new_funding_cycles, amount);
    assert_eq!(
        funded_reset.conservation.maximum_unavoidable_fee_cycles,
        bounds.ledger_fee
    );
    assert_eq!(
        funded_reset.conservation.maximum_operator_debit_cycles,
        amount + bounds.ledger_fee
    );
    assert_eq!(
        funded_reset.conservation.maximum_execution_burn_cycles,
        prepared.conservation.maximum_execution_burn_cycles + bounds.update_burn + per_effect
    );
    assert_eq!(
        funded_reset.conservation.expected_post_operation_cycles,
        funded_reset.conservation.observed_controlled_cycles + amount
            - funded_reset.conservation.maximum_execution_burn_cycles
    );
    let reset = reset(
        RootStartPlanInput {
            authority: None,
            state: &state,
            desired: &desired,
            desired_sha256: &prepared.desired_sha256,
            created_at_time: 1,
            requested_fleet: &desired.fleet,
            observation: &management,
        },
        artifacts,
        &prepared,
        evidence.clone(),
        &observation,
    )
    .expect("separate corrected Root review");
    assert_eq!(reset.scope, FleetEnsurePlanScope::RootReinstallPrerequisite);
    assert_eq!(reset.operation_id, prepared.operation_id);
    assert!(matches!(
        reset.canisters[0].actions.as_slice(),
        [
            EnsureAction::Stop { .. },
            EnsureAction::Install {
                mode: InstallMode::Reinstall,
                ..
            },
            EnsureAction::Start { .. }
        ]
    ));
    assert_eq!(
        reset
            .reinstall
            .as_ref()
            .unwrap()
            .activation_reset
            .as_ref()
            .unwrap()
            .preparation,
        Some(evidence)
    );
    let mut intent = *reset.reinstall.unwrap();
    let mut after = observation;
    for binding in &mut intent.authorities {
        let live = after
            .canisters
            .get_mut(&binding.name)
            .unwrap()
            .as_mut()
            .unwrap();
        if binding.name == root_name {
            binding.module_sha256 = artifacts.wasm_sha256_by_canister[&root_name].clone();
            live.module_sha256 = Some(binding.module_sha256.clone());
        } else {
            live.status = CanisterRuntimeStatus::Stopped;
        }
    }
    let targets =
        super::super::validate_reset(&desired, artifacts, &after, &intent, &prepared.operation_id)
            .expect("remaining infrastructure reset admission");
    assert!(!targets.contains(&root_name));
    assert_eq!(targets.len(), 2);
}
