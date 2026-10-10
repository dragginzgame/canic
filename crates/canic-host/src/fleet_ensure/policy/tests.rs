//! Native convergence-policy and capacity qualification.

use super::{
    CurrentFleetProtocolAction, CycleBounds, DesiredFleet, EnsurePolicyError,
    FLEET_ENSURE_SCHEMA_VERSION, FleetObservation, PlanAccumulator, append_estate_funding_actions,
    append_pool_reconciliation_funding, component_provisioning_observation_bound_from_counts,
    forecast_observed_root_pool, initial_role_tree_size, paced_protocol_stall_limit,
    required_estate_creation_count, terminal_initial_component_observation_count,
    validate_bootstrap_root_pool_import_capacity,
};
use crate::fleet_ensure::model::{
    CanisterDisposition, CanisterPlan, DesiredFleetBootstrapRoot, EnsureAction,
    EstateFundingDomainPlan, EstatePoolAssetLifecycle, EstatePoolAssetObservation,
    EstatePoolAssetOrigin, EstatePoolInventoryObservation, EstatePoolPendingCreationObservation,
    PoolFundingAuthority,
};
use candid::Principal;
use canic_contracts::{
    cycles::Cycles,
    ids::{
        CanisterRole, ComponentSpecId, ComponentTopologyDigest, CyclesFundingBudget,
        FleetSubnetCanisterPoolConfig, FleetSubnetRootLimits, SubnetId,
    },
};
use canic_core::control_plane_support::config::{
    ComponentLimits, ComponentSpawnGrant, ComponentSpec,
};
use std::collections::BTreeMap;

#[test]
fn terminal_pool_capacity_keeps_workloads_and_ready_reserve_independent() {
    for (workloads, ready_floor) in [(2, 1), (19, 5), (32, 5)] {
        let required = workloads + ready_floor;
        for maximum_size in [required, required + 1] {
            assert_eq!(
                super::terminal_pool_supply("root", workloads, ready_floor, maximum_size),
                Ok(required)
            );
        }
        for maximum_size in [workloads, required - 1] {
            assert_eq!(
                super::terminal_pool_supply("root", workloads, ready_floor, maximum_size),
                Err(EnsurePolicyError::TerminalPoolCapacity {
                    root: "root".into(),
                    workloads,
                    ready_floor,
                    maximum_size,
                })
            );
        }
    }
    assert_eq!(
        super::terminal_pool_supply("root", u32::MAX, 1, u32::MAX),
        Err(EnsurePolicyError::TerminalPoolCapacity {
            root: "root".into(),
            workloads: u32::MAX,
            ready_floor: 1,
            maximum_size: u32::MAX,
        })
    );
}

#[test]
fn continuation_review_reserves_only_available_headroom_and_exposes_pending_discovery() {
    let observation = FleetObservation {
        additional_controlled_cycles: BTreeMap::new(),
        canisters: BTreeMap::new(),
        estate_funding_domains: BTreeMap::new(),
        ledger_fee_cycles: 0,
        operator_cycles: 0,
        protocol_ready: BTreeMap::new(),
    };
    let bounds = CycleBounds {
        observation_burn: 3,
        update_burn: 5,
        ledger_fee: 1,
        management_creation_fee: 0,
        material_threshold: 1,
    };
    let review = super::recovery::review(&observation, bounds, 32, 0, 41, 369).unwrap();
    assert_eq!(review.base_execution_burn_cycles, 41);
    assert_eq!(review.whole_continuation_ceiling_cycles, 448);
    assert_eq!(review.continuation_reserve_cycles, 328);
    assert_eq!(review.maximum_successor_actions, 32);
    assert_eq!(review.fixture_publication_retry_attempts, 0);
    assert_eq!(review.per_step_burn_cycles, 14);
    assert!(review.known_pool_funding.is_empty());
    assert_eq!(
        review.discovery,
        crate::fleet_ensure::model::RecoveryDiscovery::PendingCurrentProtocol
    );
    let no_headroom = super::recovery::review(&observation, bounds, 32, 0, 41, 40).unwrap();
    assert_eq!(no_headroom.continuation_reserve_cycles, 0);
    let fixtures = super::recovery::review(&observation, bounds, 32, 6, 41, 1_000).unwrap();
    assert_eq!(fixtures.whole_continuation_ceiling_cycles, 532);
    assert_eq!(fixtures.continuation_reserve_cycles, 532);
    assert_eq!(fixtures.fixture_publication_retry_attempts, 6);
    assert_eq!(fixtures.per_step_burn_cycles, 14);
    assert_eq!(
        fixtures.base_execution_burn_cycles,
        review.base_execution_burn_cycles
    );
}

#[test]
fn estate_workload_forecast_includes_recursive_initial_children() {
    let mut spec = ComponentSpec {
        application_init_required: false,
        component_spec: ComponentSpecId::try_from(String::from("hub"))
            .expect("hub Component Spec ID"),
        spec_hash: [1; 32],
        component_role: CanisterRole::from("hub"),
        maximum_fleet_instances: 1,
        limits: ComponentLimits {
            maximum_descendants: 8,
            maximum_registry_bytes: 1,
            cycles_funding: CyclesFundingBudget {
                window_secs: 1,
                maximum_cycles: Cycles::new(1),
            },
        },
        children: Vec::new(),
        spawn_grants: vec![
            ComponentSpawnGrant {
                parent_role: CanisterRole::from("hub"),
                child_role: CanisterRole::from("shard"),
                initial_instances_per_parent: 2,
                maximum_instances_per_parent: 2,
            },
            ComponentSpawnGrant {
                parent_role: CanisterRole::from("shard"),
                child_role: CanisterRole::from("leaf"),
                initial_instances_per_parent: 3,
                maximum_instances_per_parent: 3,
            },
        ],
    };

    assert_eq!(initial_role_tree_size(&spec), Ok(9));
    for capacity in [8, 10_000, u32::MAX] {
        spec.limits.maximum_descendants = capacity;
        let children = super::initial_child_observation_bound(
            std::slice::from_ref(&spec),
            [(&spec.component_spec, &spec.spec_hash); 2],
        )
        .unwrap();
        assert_eq!(children, 48);
        let base = component_provisioning_observation_bound_from_counts(1, 2).unwrap();
        assert_eq!(paced_protocol_stall_limit(8, base + children), 60);
    }
    assert!(matches!(
        super::initial_child_observation_bound(
            std::slice::from_ref(&spec),
            [(&spec.component_spec, &[2; 32])]
        ),
        Err(EnsurePolicyError::EstateFundingTopology { .. })
    ));
}

#[test]
fn estate_creation_forecast_retains_ready_floor_without_repeating_workloads() {
    assert_eq!(required_estate_creation_count(8, 10, 10, 0), Ok(8));
    assert_eq!(required_estate_creation_count(8, 10, 10, 8), Ok(0));
    assert_eq!(required_estate_creation_count(8, 2, 2, 0), Ok(8));
    assert_eq!(required_estate_creation_count(8, 2, 10, 0), Ok(0));
}

#[test]
fn estate_shortfall_compiles_one_exact_plan_owned_ledger_transfer() {
    let mut accumulator = PlanAccumulator::new();
    accumulator.canisters.push(CanisterPlan {
        actions: Vec::new(),
        disposition: CanisterDisposition::Reuse,
        name: "root-0".to_string(),
        observed_cycles: 30,
        principal: Some("rrkah-fqaaa-aaaaa-aaaaq-cai".to_string()),
    });
    let domain = EstateFundingDomainPlan {
        allocated_workloads: 1,
        available_cycles: Some(40),
        available_pool_slots: 1,
        creation_amount_cycles: 45,
        creation_execution_margin_cycles: 5,
        readiness_floor_cycles: 35,
        cycles_ledger: "aaaaa-aa".to_string(),
        eligible_ready_pool_assets: 1,
        initial_pool_assets: vec!["existing-pool".to_string()],
        ledger_fee_cycles: 5,
        management_creation_fee_cycles: 5,
        maximum_creation_debit_cycles: 100,
        maximum_creation_fee_cycles: 20,
        maximum_funding_cycles: 60,
        occupied_pool_assets: 1,
        pending_creation_count: 0,
        pending_creation: None,
        planned_initial_workloads: 2,
        pool_maximum_size: 2,
        required_creation_count: 2,
        root: "root-0".to_string(),
        root_principal: Some("rrkah-fqaaa-aaaaa-aaaaq-cai".to_string()),
        shortfall_cycles: 60,
    };

    append_estate_funding_actions(&[domain], 100, 4, &mut accumulator)
        .expect("compile exact estate funding action");

    assert_eq!(accumulator.new_funding, 60);
    assert_eq!(accumulator.fees, 5);
    assert!(matches!(
        accumulator.canisters[0].actions.as_slice(),
        [EnsureAction::FundEstate {
            amount: 60,
            created_at_time: 104,
            expected_post_cycles: 100,
            ledger,
            ledger_fee_cycles: 5,
            name,
            principal,
        }] if ledger == "aaaaa-aa"
            && name == "root-0"
            && principal == "rrkah-fqaaa-aaaaa-aaaaq-cai"
    ));
}

#[test]
fn complete_pool_inventory_owns_creation_capacity_forecast() {
    let policy = pool_policy(4, 8);
    let exhausted = observed_pool([
        EstatePoolAssetLifecycle::Workload,
        EstatePoolAssetLifecycle::Workload,
        EstatePoolAssetLifecycle::Workload,
        EstatePoolAssetLifecycle::Workload,
        EstatePoolAssetLifecycle::Failed,
        EstatePoolAssetLifecycle::Failed,
        EstatePoolAssetLifecycle::Failed,
        EstatePoolAssetLifecycle::Failed,
    ]);
    assert!(matches!(
        forecast_observed_root_pool("root", &exhausted, &policy, 4, 4, 1_900, 500),
        Err(EnsurePolicyError::EstatePoolCapacity {
            allocated_workloads: 4,
            available_slots: 0,
            capacity_shortfall: 4,
            eligible_ready_assets: 0,
            maximum_size: 8,
            occupied_assets: 8,
            pending_creations: 0,
            required_creation_count: 4,
            ..
        })
    ));

    let recoverable = observed_pool([
        EstatePoolAssetLifecycle::Workload,
        EstatePoolAssetLifecycle::Workload,
        EstatePoolAssetLifecycle::Workload,
        EstatePoolAssetLifecycle::Workload,
        EstatePoolAssetLifecycle::Ready,
        EstatePoolAssetLifecycle::Ready,
    ]);
    let forecast = forecast_observed_root_pool("root", &recoverable, &policy, 4, 4, 1_900, 500)
        .expect("two free slots can restore the four-Ready floor");
    assert_eq!(forecast.occupied_assets, 6);
    assert_eq!(forecast.eligible_ready_assets, 2);
    assert_eq!(forecast.required_creation_count, 2);
    assert_eq!(forecast.available_slots, 2);
}

#[test]
fn every_retained_pool_lifecycle_and_pending_creation_consumes_capacity_once() {
    let policy = pool_policy(2, 10);
    let mut observed = observed_pool([
        EstatePoolAssetLifecycle::Claimed,
        EstatePoolAssetLifecycle::Failed,
        EstatePoolAssetLifecycle::HandingOff,
        EstatePoolAssetLifecycle::PendingReset,
        EstatePoolAssetLifecycle::Ready,
        EstatePoolAssetLifecycle::Recycling,
        EstatePoolAssetLifecycle::Workload,
    ]);
    observed.minimum_size = 2;
    observed.maximum_size = 10;
    observed.pending_creation = Some(EstatePoolPendingCreationObservation {
        attempt_count: 1,
        available_cycles: None,
        creation_amount_cycles: 2_500,
        created_principal: None,
        diagnostic: None,
        last_attempt_at_ns: Some(1),
        operation_id: "11".repeat(32),
        required_cycles: None,
        retry_at_ns: Some(2),
        shortfall_cycles: Some(2_500),
        uncertain_result: false,
    });
    let forecast = forecast_observed_root_pool("root", &observed, &policy, 2, 2, 1_900, 500)
        .expect("all bounded lifecycles fit with one free slot");
    assert_eq!(forecast.occupied_assets, 8);
    assert_eq!(forecast.allocated_workloads, 2);
    assert_eq!(forecast.eligible_ready_assets, 1);
    assert_eq!(forecast.pending_creations, 1);
    assert_eq!(forecast.required_creation_count, 1);

    observed.maximum_size = 9;
    let drifted = forecast_observed_root_pool("root", &observed, &policy, 2, 2, 1_900, 500)
        .expect("observed policy drift retains exact assets under desired limits");
    assert_eq!(drifted, forecast);
}

#[test]
fn unresolved_pool_creation_rejects_before_funding_authority() {
    let policy = pool_policy(1, 2);
    let mut observed = observed_pool([]);
    observed.pending_creation = Some(EstatePoolPendingCreationObservation {
        attempt_count: 1,
        available_cycles: None,
        creation_amount_cycles: 2_500,
        created_principal: None,
        diagnostic: None,
        last_attempt_at_ns: Some(1),
        operation_id: "11".repeat(32),
        required_cycles: None,
        retry_at_ns: None,
        shortfall_cycles: None,
        uncertain_result: true,
    });
    assert!(matches!(
        forecast_observed_root_pool("root", &observed, &policy, 1, 1, 1_900, 500),
        Err(EnsurePolicyError::EstateFundingTopology { reason })
            if reason.contains("unresolved creation response")
    ));
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one complete inventory proof binds funding, retained identities and the no-creation outcome"
)]
fn full_failed_inventory_funds_exact_retained_assets_without_creating_replacements() {
    let root = Principal::from_slice(&[10]).to_text();
    let desired = DesiredFleet {
        bootstrap: None,
        canisters: Vec::new(),
        cycles_ledger: Principal::from_slice(&[11]).to_text(),
        environment: "local".into(),
        fleet: "repair".into(),
        ledger_fee_cycles: "5".into(),
        management_creation_fee_cycles: "500".into(),
        material_cycle_threshold: "1".into(),
        maximum_observation_burn_cycles: "10".into(),
        maximum_stalled_observations: 8,
        maximum_update_burn_cycles: "20".into(),
        operator: Principal::from_slice(&[12]).to_text(),
        protocol: None,
        protocol_steps: Vec::new(),
        schema_version: FLEET_ENSURE_SCHEMA_VERSION,
        treasury: "coordinator".into(),
    };
    let mut pool = observed_pool([
        EstatePoolAssetLifecycle::Workload,
        EstatePoolAssetLifecycle::Workload,
        EstatePoolAssetLifecycle::Workload,
        EstatePoolAssetLifecycle::Workload,
        EstatePoolAssetLifecycle::Failed,
        EstatePoolAssetLifecycle::Failed,
        EstatePoolAssetLifecycle::Failed,
        EstatePoolAssetLifecycle::Failed,
    ]);
    let mut actions = Vec::new();
    for (index, asset) in pool.assets.iter_mut().enumerate() {
        let canister_id =
            Principal::from_slice(&[u8::try_from(index + 20).expect("bounded asset")]);
        asset.principal = canister_id.to_text();
        if asset.lifecycle == EstatePoolAssetLifecycle::Failed {
            asset.cycles -= 1;
            actions.push(EnsureAction::FleetProtocol {
                action: Box::new(CurrentFleetProtocolAction::ReconcilePoolAsset {
                    request: canic_contracts::dto::pool::PoolCanisterRequest { canister_id },
                    minimum_cycles: Cycles::new(pool.readiness_floor_cycles),
                }),
                candid: "root.did".into(),
                candid_sha256: "42".repeat(32),
                maximum_execution_burn_cycles: 20,
                name: format!("pool-reconcile-{canister_id}"),
                principal: root.clone(),
            });
        }
    }
    assert!(matches!(
        forecast_observed_root_pool("root", &pool, &pool_policy(4, 8), 4, 4, 1_900, 500),
        Err(EnsurePolicyError::EstatePoolCapacity {
            capacity_shortfall: 4,
            ..
        })
    ));
    let observation = FleetObservation {
        additional_controlled_cycles: pool
            .assets
            .iter()
            .map(|asset| (asset.principal.clone(), asset.cycles))
            .collect(),
        canisters: BTreeMap::new(),
        estate_funding_domains: BTreeMap::from([(
            "root".into(),
            crate::fleet_ensure::model::EstateFundingDomainObservation {
                balance_cycles: Some(0),
                cycles_ledger: desired.cycles_ledger.clone(),
                pool: Some(pool.clone()),
                root_principal: Some(root.clone()),
            },
        )]),
        ledger_fee_cycles: 5,
        operator_cycles: 10_000,
        protocol_ready: BTreeMap::new(),
    };
    let mut accumulator = PlanAccumulator::new();
    append_pool_reconciliation_funding(
        &desired,
        &observation,
        &actions,
        CycleBounds {
            ledger_fee: 5,
            management_creation_fee: 500,
            material_threshold: 1,
            observation_burn: 10,
            update_burn: 20,
        },
        100,
        &mut accumulator,
    )
    .expect("review recovery funding");
    let preview = super::recovery::review(
        &observation,
        CycleBounds {
            ledger_fee: 5,
            management_creation_fee: 500,
            material_threshold: 1,
            observation_burn: 10,
            update_burn: 20,
        },
        32,
        0,
        40,
        10_000,
    )
    .unwrap();
    assert_eq!(preview.known_pool_funding.len(), 4);
    assert_eq!(
        preview
            .known_pool_funding
            .iter()
            .map(|funding| funding.amount_cycles)
            .sum::<u128>(),
        accumulator.new_funding
    );
    assert_eq!(
        preview
            .known_pool_funding
            .iter()
            .map(|funding| funding.ledger_fee_cycles)
            .sum::<u128>(),
        accumulator.fees
    );
    assert_eq!(accumulator.new_funding, 4 * 131);
    assert_eq!(accumulator.fees, 4 * 5);
    for plan in &accumulator.canisters {
        assert!(matches!(plan.actions.as_slice(), [EnsureAction::Fund {
            pool_funding: Some(PoolFundingAuthority { root: bound_root, lifecycle: EstatePoolAssetLifecycle::Failed }), principal, amount: 131, expected_post_cycles: 2_030,
            funding_deficit_cycles: 101, funding_margin_cycles: 30, ..
        }] if bound_root == &root && Some(principal) == plan.principal.as_ref()));
    }
    for asset in &mut pool.assets {
        if asset.lifecycle == EstatePoolAssetLifecycle::Failed {
            asset.cycles = 2_000;
            asset.lifecycle = EstatePoolAssetLifecycle::Ready;
        }
    }
    let recovered =
        forecast_observed_root_pool("root", &pool, &pool_policy(4, 8), 4, 4, 1_900, 500)
            .expect("recovered full pool satisfies the same demand");
    assert_eq!(recovered.eligible_ready_assets, 4);
    assert_eq!(recovered.required_creation_count, 0);
}

fn pool_policy(minimum_size: u32, maximum_size: u32) -> FleetSubnetCanisterPoolConfig {
    FleetSubnetCanisterPoolConfig {
        minimum_size,
        maximum_size,
        canister_cycles: Cycles::new(1_900),
        creation_execution_margin: Cycles::new(100),
    }
}

fn observed_pool(
    lifecycles: impl IntoIterator<Item = EstatePoolAssetLifecycle>,
) -> EstatePoolInventoryObservation {
    EstatePoolInventoryObservation {
        assets: lifecycles
            .into_iter()
            .enumerate()
            .map(|(index, lifecycle)| EstatePoolAssetObservation {
                creation_receipt: None,
                cycles: 1_900,
                lifecycle,
                origin: EstatePoolAssetOrigin::Imported,
                principal: format!("asset-{index}"),
            })
            .collect(),
        maximum_size: 8,
        minimum_size: 4,
        pending_creation: None,
        readiness_floor_cycles: 1_900,
        creation_execution_margin_cycles: 100,
    }
}

#[test]
fn fresh_fleet_descendant_capacity_does_not_multiply_terminal_proof() {
    for maximum_descendants in [0, 1, 10_000, u32::MAX] {
        assert_eq!(
            terminal_initial_component_observation_count(maximum_descendants),
            3,
        );
    }
}

#[test]
fn component_provisioning_stall_floor_scales_and_caps() {
    let topology_bound = component_provisioning_observation_bound_from_counts(1, 11)
        .expect("bounded one-Root eleven-Component topology");
    assert_eq!(paced_protocol_stall_limit(8, topology_bound), 39);
    assert_eq!(paced_protocol_stall_limit(8, 10_000), 64);
    assert_eq!(paced_protocol_stall_limit(80, topology_bound), 80);
}

#[test]
fn apply_policy_rejects_bootstrap_imports_above_the_root_maximum() {
    let root = DesiredFleetBootstrapRoot {
        capacity_import_bootstrap: None,
        canister_pool_imports: vec![
            "pool-0".to_string(),
            "pool-1".to_string(),
            "pool-2".to_string(),
        ],
        component_admissions: Vec::new(),
        component_topology_digest: ComponentTopologyDigest::from_bytes([1; 32]),
        funding: crate::test_support::fleet_subnet_root_funding_authority(),
        limits: FleetSubnetRootLimits {
            maximum_component_instances: 1,
            maximum_registry_bytes: 1,
            maximum_wasm_store_bytes: 1,
            canister_pool: FleetSubnetCanisterPoolConfig {
                minimum_size: 2,
                maximum_size: 2,
                canister_cycles: Cycles::new(1),
                creation_execution_margin: Cycles::new(1),
            },
            cycles_funding: CyclesFundingBudget {
                window_secs: 1,
                maximum_cycles: Cycles::new(1),
            },
            maximum_group_placements: 1,
        },
        placement_subnet: SubnetId::from_principal(Principal::from_slice(&[2])),
        root: "root-0".to_string(),
        store: "store-0".to_string(),
    };

    assert!(matches!(
        validate_bootstrap_root_pool_import_capacity(&root),
        Err(EnsurePolicyError::PoolImportCapacity(error))
            if error.import_count == 3
                && error.maximum_size == 2
                && error.root == "root-0"
    ));
}
