use super::*;
use crate::fleet_ensure::{
    model::{FleetEnsurePlan, StartupRoleShortfall},
    ops::{EnsureStateError, resolve_desired_artifacts},
    policy::{compile_plan, expected_plan_sha256},
};
use canic_core::{
    cdk::types::Cycles,
    dto::component_provisioning::{
        ComponentGroupPlacementPlan, ComponentGroupPlanEntry, FleetSubnetRootProvisioningBatch,
    },
    ids::{
        ComponentGroupPlacementId, FleetSubnetRootBinding, FleetSubnetRootReleaseSet,
        ReleaseSetDigest,
    },
};
use std::{collections::BTreeMap, fs, path::Path};

const T: u128 = 1_000_000_000_000;

/// Called by the generated-estate fixture so these checks share its exact source/artifact setup.
#[expect(
    clippy::too_many_lines,
    reason = "one generated authority fixture qualifies funding, rejection, tranching and source binding"
)]
pub(in crate::fleet_ensure) fn qualify(
    workspace: &Path,
    desired: &DesiredFleet,
    observed: &FleetObservation,
    mut actions: Vec<EnsureAction>,
) {
    qualify_recovery_threshold(desired);
    let mut artifacts = resolve_desired_artifacts(workspace, desired).unwrap();
    let root_name = &desired.bootstrap.as_ref().unwrap().roots[0].root;
    assert_eq!(
        artifacts.startup_funding_by_root[root_name].minimum_native_cycles,
        10 * T + 1
    );
    let config_path = workspace.join(&desired.protocol.as_ref().unwrap().app_config);
    let original = fs::read_to_string(&config_path).unwrap();
    fs::write(
        &config_path,
        original.replace("initial_cycles = \"5T\"", "initial_cycles = \"6T\""),
    )
    .unwrap();
    let mismatch = resolve_desired_artifacts(workspace, desired);
    fs::write(&config_path, original).unwrap();
    assert!(matches!(
        mismatch,
        Err(EnsureStateError::StartupConfigurationMismatch)
    ));

    bind_batch(desired, &mut actions);
    let operation_id = match &actions[1] {
        EnsureAction::FleetProtocol { action, .. } => action.operation_id().unwrap(),
        _ => panic!("provisioning action"),
    };
    let operation = canic_core::cdk::utils::hash::hex_bytes(operation_id);
    let compile = |observation: &FleetObservation,
                   artifacts: &DesiredFleetArtifacts,
                   actions: &[EnsureAction]| {
        compile_plan(
            desired,
            artifacts,
            actions,
            &"31".repeat(32),
            &desired.fleet,
            observation,
            1_800_000_000_000_000_000,
            &operation,
            None,
        )
    };
    let mut observation = observed.clone();
    for (name, live) in &mut observation.canisters {
        if let Some(live) = live
            && let Some(hash) = artifacts.wasm_sha256_by_canister.get(name)
        {
            live.module_sha256 = Some(hash.clone());
        }
    }
    // The planner input is deliberately 50T here; the separate resolver assertion
    // above checks source binding rather than claiming this fixture needs 50T.
    qualify_reinstall(desired, &artifacts, &observation);
    artifacts
        .startup_funding_by_root
        .get_mut(root_name)
        .unwrap()
        .minimum_native_cycles = 50 * T;
    for (balance, new_fee) in [(30 * T, true), (9 * T, false)] {
        observation
            .canisters
            .get_mut(root_name)
            .unwrap()
            .as_mut()
            .unwrap()
            .cycles = balance;
        let baseline = compile(&observation, &artifacts, &[]).unwrap();
        let planned = compile(&observation, &artifacts, &actions).unwrap();
        assert_eq!(
            planned,
            compile(&observation, &artifacts, &actions).unwrap()
        );
        assert_ne!(planned.plan_sha256, baseline.plan_sha256);
        let funding = root_funding(&planned, root_name).unwrap();
        let EnsureAction::Fund {
            amount,
            expected_post_cycles,
            funding_deficit_cycles,
            funding_margin_cycles,
            ..
        } = funding
        else {
            unreachable!()
        };
        assert_eq!(*funding_deficit_cycles, 50 * T + 1 - balance);
        assert_eq!(*expected_post_cycles, balance + amount);
        assert_eq!(*amount, funding_deficit_cycles + funding_margin_cycles);
        assert_eq!(
            planned.conservation.maximum_unavoidable_fee_cycles
                - baseline.conservation.maximum_unavoidable_fee_cycles,
            if new_fee {
                observation.ledger_fee_cycles
            } else {
                0
            }
        );
        let prior = root_funding(&baseline, root_name).map_or(0, |action| match action {
            EnsureAction::Fund { amount, .. } => *amount,
            _ => unreachable!(),
        });
        assert_eq!(
            planned.conservation.maximum_new_funding_cycles
                - baseline.conservation.maximum_new_funding_cycles,
            amount - prior
        );
        assert_eq!(planned.plan_sha256, expected_plan_sha256(&planned));
        let ordered = crate::fleet_ensure::workflow::ordered_actions(&planned);
        let fund_index = ordered
            .iter()
            .position(|action| *action == funding)
            .unwrap();
        let provision_index = ordered.iter().position(|action| matches!(action, EnsureAction::FleetProtocol { action, .. } if matches!(action.as_ref(), CurrentFleetProtocolAction::ProvisionComponents { .. }))).unwrap();
        assert!(fund_index < provision_index);
    }
    observation
        .canisters
        .get_mut(root_name)
        .unwrap()
        .as_mut()
        .unwrap()
        .cycles = 30 * T;
    let terminal = compile(&observation, &artifacts, &[]).unwrap();
    assert!(root_funding(&terminal, root_name).is_none());
    let mut unrelated = actions.clone();
    if let EnsureAction::FleetProtocol { action, .. } = &mut unrelated[1]
        && let CurrentFleetProtocolAction::ProvisionComponents { request, .. } = action.as_mut()
    {
        request.plan.batches.clear();
    }
    assert!(
        root_funding(
            &compile(&observation, &artifacts, &unrelated).unwrap(),
            root_name
        )
        .is_none()
    );
    let mut invalid = artifacts.clone();
    invalid
        .startup_funding_by_root
        .get_mut(root_name)
        .unwrap()
        .unfunded_role = Some(StartupRoleShortfall {
        role: "app".into(),
        cycles: 1,
    });
    assert!(matches!(
        compile(&observation, &invalid, &actions),
        Err(EnsurePolicyError::StartupRoleFundingUnavailable {
            shortfall_cycles: 1,
            ..
        })
    ));
    invalid.startup_funding_by_root.clear();
    assert!(matches!(
        compile(&observation, &invalid, &actions),
        Err(EnsurePolicyError::MissingArtifactIdentity {
            kind: "startup funding",
            ..
        })
    ));

    // Find a conservation boundary at which only Store preparation fits. The
    // Root's ordinary funding must survive, but its startup increment must not.
    observation
        .canisters
        .get_mut(root_name)
        .unwrap()
        .as_mut()
        .unwrap()
        .cycles = 9 * T;
    artifacts
        .startup_funding_by_root
        .get_mut(root_name)
        .unwrap()
        .minimum_native_cycles = 10 * T + 1;
    let baseline = compile(&observation, &artifacts, &[]).unwrap();
    let deferred = (0..400)
        .find_map(|cost| {
            if let EnsureAction::FleetProtocol {
                maximum_execution_burn_cycles,
                ..
            } = &mut actions[0]
            {
                *maximum_execution_burn_cycles = cost * T;
            }
            let plan = compile(&observation, &artifacts, &actions).ok()?;
            (!plan.protocol_actions.is_empty() && !has_provisioning(&plan.protocol_actions))
                .then_some(plan)
        })
        .expect("fixture spans the provisioning conservation boundary");
    assert_eq!(
        root_funding(&deferred, root_name),
        root_funding(&baseline, root_name)
    );
    assert_eq!(
        deferred.conservation.maximum_new_funding_cycles,
        baseline.conservation.maximum_new_funding_cycles
    );
    assert_eq!(
        deferred.conservation.maximum_unavoidable_fee_cycles,
        baseline.conservation.maximum_unavoidable_fee_cycles
    );
}

fn qualify_recovery_threshold(desired: &DesiredFleet) {
    use crate::fleet_ensure::policy::startup_funding::recovery_minimum_cycles;
    let mut desired = desired.clone();
    let root = desired.bootstrap.as_ref().unwrap().roots[0].root.clone();
    assert_eq!(
        recovery_minimum_cycles(&desired, &root, 0).unwrap(),
        10 * T + 1
    );
    assert_eq!(
        recovery_minimum_cycles(&desired, &root, 20 * T).unwrap(),
        20 * T
    );

    let mut unrelated = desired.bootstrap.as_ref().unwrap().roots[0].clone();
    unrelated.root = "unrelated-root".into();
    unrelated.funding.root_funding.request_threshold = Cycles::new(100 * T);
    desired.bootstrap.as_mut().unwrap().roots.push(unrelated);
    assert_eq!(
        recovery_minimum_cycles(&desired, &root, 0).unwrap(),
        10 * T + 1
    );
    assert!(matches!(
        recovery_minimum_cycles(&desired, "missing-root", 0),
        Err(EnsurePolicyError::EstateFundingTopology { .. })
    ));
    let duplicate = desired.bootstrap.as_ref().unwrap().roots[0].clone();
    desired.bootstrap.as_mut().unwrap().roots.push(duplicate);
    assert!(matches!(
        recovery_minimum_cycles(&desired, &root, 0),
        Err(EnsurePolicyError::EstateFundingTopology { .. })
    ));
    desired.bootstrap.as_mut().unwrap().roots.pop();
    desired.bootstrap.as_mut().unwrap().roots[0]
        .funding
        .root_funding
        .request_threshold = Cycles::new(u128::MAX);
    assert!(matches!(
        recovery_minimum_cycles(&desired, &root, 0),
        Err(EnsurePolicyError::ArithmeticOverflow { .. })
    ));
    desired.bootstrap = None;
    assert_eq!(recovery_minimum_cycles(&desired, &root, 0).unwrap(), T);
}

fn root_funding<'a>(plan: &'a FleetEnsurePlan, root: &str) -> Option<&'a EnsureAction> {
    let actions = &plan
        .canisters
        .iter()
        .find(|canister| canister.name == root)
        .unwrap()
        .actions;
    let funds = actions
        .iter()
        .filter(|action| matches!(action, EnsureAction::Fund { .. }))
        .collect::<Vec<_>>();
    assert!(funds.len() <= 1);
    funds.first().copied()
}

fn bind_batch(desired: &DesiredFleet, actions: &mut [EnsureAction]) {
    let bootstrap = desired.bootstrap.as_ref().unwrap();
    let root = &bootstrap.roots[0];
    let deployment = &bootstrap
        .component_deployment_configuration
        .deployment_topology
        .component_group_deployments[0];
    let EnsureAction::FleetProtocol { action, .. } = &mut actions[1] else {
        panic!("provisioning");
    };
    let CurrentFleetProtocolAction::ProvisionComponents { request, .. } = action.as_mut() else {
        panic!("provisioning");
    };
    let principal = desired
        .canisters
        .iter()
        .find(|canister| canister.name == root.root)
        .unwrap()
        .principal
        .as_ref()
        .unwrap()
        .parse()
        .unwrap();
    request.plan.batches.push(FleetSubnetRootProvisioningBatch {
        root: FleetSubnetRootBinding {
            authority: request.plan.fleet_registry.authority.clone(),
            placement_subnet: root.placement_subnet,
            fleet_subnet_root: principal,
            component_admissions: root.component_admissions.clone(),
            component_topology_digest: root.component_topology_digest,
            limits: root.limits.clone(),
            funding: root.funding.clone(),
        },
        active_release_set: FleetSubnetRootReleaseSet {
            release_build_id: bootstrap.release_build_id,
            manifest_digest: ReleaseSetDigest::from_bytes([5; 32]),
        },
        placements: vec![ComponentGroupPlacementPlan {
            group_placement: ComponentGroupPlacementId {
                deployment: deployment.deployment.clone(),
                ordinal: 0,
            },
            component_group: deployment.component_group.clone(),
            entries: deployment
                .members
                .iter()
                .map(|member| ComponentGroupPlanEntry {
                    member_path: member.member_path.clone(),
                    component_spec: member.component_spec.clone(),
                    spec_hash: member.component_spec_hash,
                    purpose: member.purpose.clone(),
                    labels: member.labels.clone(),
                    limits: member.limits.clone(),
                })
                .collect(),
        }],
    });
}

/// Qualify the initial Create amount on the existing generated fresh-estate authority.
pub(in crate::fleet_ensure) fn qualify_creation(
    workspace: &Path,
    desired: &DesiredFleet,
    placements: &[crate::fleet_ensure::model::DesiredComponentGroupPlacement],
) {
    let observation = empty_observation(desired);
    let artifacts = resolve_desired_artifacts(workspace, desired).unwrap();
    let baseline = continuation_plan(desired, &artifacts, &observation);
    let mut selected = desired.clone();
    selected
        .protocol
        .as_mut()
        .unwrap()
        .component_group_placements = placements.to_vec();
    let artifacts = resolve_desired_artifacts(workspace, &selected).unwrap();
    let planned = continuation_plan(&selected, &artifacts, &observation);
    assert!(planned.continuation.is_some());
    assert!(planned.protocol_actions.is_empty());
    let root = &selected.bootstrap.as_ref().unwrap().roots[0].root;
    let initial = |plan: &FleetEnsurePlan| {
        plan.canisters
            .iter()
            .find(|canister| &canister.name == root)
            .unwrap()
            .actions
            .iter()
            .find_map(|action| match action {
                EnsureAction::Create {
                    requested_initial_cycles,
                    ..
                } => Some(*requested_initial_cycles),
                _ => None,
            })
            .unwrap()
    };
    let before = initial(&baseline);
    let after = initial(&planned);
    assert!(after > before);
    assert!(after > artifacts.startup_funding_by_root[root].minimum_native_cycles);
    assert_eq!(
        planned.conservation.maximum_new_funding_cycles
            - baseline.conservation.maximum_new_funding_cycles,
        after - before
    );
    assert_eq!(
        planned.conservation.maximum_unavoidable_fee_cycles,
        baseline.conservation.maximum_unavoidable_fee_cycles
    );
    assert_eq!(
        planned,
        continuation_plan(&selected, &artifacts, &observation)
    );
    assert_eq!(planned.plan_sha256, expected_plan_sha256(&planned));
    assert!(root_funding(&planned, root).is_none());

    let mut missing = artifacts.clone();
    missing
        .startup_funding_by_root
        .get_mut(root)
        .unwrap()
        .maximum_continuation_steps = 0;
    assert!(matches!(
        compile_plan(
            &selected,
            &missing,
            &[],
            &"31".repeat(32),
            &selected.fleet,
            &observation,
            1_800_000_000_000_000_000,
            &"32".repeat(32),
            None
        ),
        Err(EnsurePolicyError::MissingArtifactIdentity {
            kind: "startup continuation steps",
            ..
        })
    ));
    selected
        .canisters
        .iter_mut()
        .find(|canister| &canister.name == root)
        .unwrap()
        .initial_cycles = format!("{}T", after / T + 1);
    let prefunded = continuation_plan(&selected, &artifacts, &observation);
    assert_eq!(initial(&prefunded), (after / T + 1) * T);
}

fn qualify_reinstall(
    desired: &DesiredFleet,
    artifacts: &DesiredFleetArtifacts,
    observation: &FleetObservation,
) {
    let root = &desired.bootstrap.as_ref().unwrap().roots[0].root;
    let mut observed = observation.clone();
    observed
        .canisters
        .get_mut(root)
        .unwrap()
        .as_mut()
        .unwrap()
        .module_sha256 = Some("ff".repeat(32));
    let mut ordinary = desired.clone();
    ordinary
        .protocol
        .as_mut()
        .unwrap()
        .component_group_placements
        .clear();
    let baseline = continuation_plan(&ordinary, artifacts, &observed);
    let planned = continuation_plan(desired, artifacts, &observed);
    assert!(planned.continuation.is_some());
    assert!(planned.protocol_actions.is_empty());
    let Some(EnsureAction::Fund {
        amount,
        expected_post_cycles,
        ..
    }) = root_funding(&planned, root)
    else {
        panic!("initial reinstall review funds startup reserve");
    };
    let prior = root_funding(&baseline, root).map_or(0, |action| match action {
        EnsureAction::Fund { amount, .. } => *amount,
        _ => unreachable!(),
    });
    assert!(*amount > prior);
    assert_eq!(
        planned.conservation.maximum_new_funding_cycles
            - baseline.conservation.maximum_new_funding_cycles,
        amount - prior
    );
    let fee = if prior == 0 {
        observed.ledger_fee_cycles
    } else {
        0
    };
    assert_eq!(
        planned.conservation.maximum_unavoidable_fee_cycles
            - baseline.conservation.maximum_unavoidable_fee_cycles,
        fee
    );
    assert!(expected_post_cycles > &artifacts.startup_funding_by_root[root].minimum_native_cycles);
    let ordered = crate::fleet_ensure::workflow::ordered_actions(&planned);
    let funding = ordered
        .iter()
        .position(|action| matches!(action, EnsureAction::Fund { name, .. } if name == root))
        .unwrap();
    let install = ordered
        .iter()
        .position(|action| matches!(action, EnsureAction::Install { name, .. } if name == root))
        .unwrap();
    assert!(funding < install);
}

fn continuation_plan(
    desired: &DesiredFleet,
    artifacts: &DesiredFleetArtifacts,
    observation: &FleetObservation,
) -> FleetEnsurePlan {
    compile_plan(
        desired,
        artifacts,
        &[],
        &"31".repeat(32),
        &desired.fleet,
        observation,
        1_800_000_000_000_000_000,
        &"32".repeat(32),
        None,
    )
    .unwrap()
}

fn empty_observation(desired: &DesiredFleet) -> FleetObservation {
    FleetObservation {
        canisters: desired
            .canisters
            .iter()
            .map(|canister| (canister.name.clone(), None))
            .collect(),
        estate_funding_domains: desired
            .bootstrap
            .as_ref()
            .unwrap()
            .roots
            .iter()
            .map(|root| {
                (
                    root.root.clone(),
                    crate::fleet_ensure::model::EstateFundingDomainObservation {
                        balance_cycles: None,
                        cycles_ledger: desired.cycles_ledger.clone(),
                        pool: None,
                        root_principal: None,
                    },
                )
            })
            .collect(),
        additional_controlled_cycles: BTreeMap::default(),
        ledger_fee_cycles: 100_000_000,
        operator_cycles: u128::MAX,
        protocol_ready: BTreeMap::default(),
    }
}
