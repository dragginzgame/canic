//! Qualify exact Fleet/registration prerequisites independently of subnet co-location.

use super::*;
use crate::fleet_ensure::{
    ops::capacity_import::prepare_review,
    policy::capacity_import::tests::{plan, principal},
};
use canic_core::{
    cdk::types::Cycles,
    dto::fleet_registry::FleetSubnetRootEntry,
    ids::{
        CyclesFundingBudget, FleetAdmissionPolicy, FleetCoordinatorBinding, FleetFundingProfile,
        FleetRegistryAuthority, FleetSubnetCanisterPoolConfig, FleetSubnetRootBinding,
        FleetSubnetRootFundingPolicy, FleetSubnetRootReleaseSet, ReleaseBuildId, ReleaseBuildNonce,
        ReleaseSetDigest,
    },
};

pub(in crate::fleet_ensure::ops::capacity_import) fn fixture()
-> (CapacityImportPlanRecord, PoolImportContext, FleetRegistry) {
    let initial = plan();
    let authority = FleetRegistryAuthority {
        binding: FleetCoordinatorBinding {
            fleet: initial.authority.fleet.clone(),
            coordinator_subnet: SubnetId::from_principal(principal(70)),
            coordinator: initial.authority.coordinator,
            recovery_controllers: initial.authority.recovery_controllers.clone(),
        },
        epoch: 1,
    };
    let budget = CyclesFundingBudget {
        window_secs: 60,
        maximum_cycles: Cycles::new(1_000),
    };
    let binding = FleetSubnetRootBinding {
        authority: authority.clone(),
        placement_subnet: initial.authority.subnet,
        fleet_subnet_root: initial.authority.root,
        component_admissions: vec![],
        component_topology_digest: ComponentTopologyDigest::from_bytes([8; 32]),
        limits: FleetSubnetRootLimits {
            maximum_component_instances: 10,
            maximum_registry_bytes: 10_000,
            maximum_wasm_store_bytes: 10_000,
            canister_pool: FleetSubnetCanisterPoolConfig {
                minimum_size: 1,
                maximum_size: 10,
                canister_cycles: Cycles::new(800),
                creation_execution_margin: Cycles::new(10),
            },
            cycles_funding: budget.clone(),
            maximum_group_placements: 10,
        },
        funding: FleetSubnetRootFundingAuthority {
            root_funding: FleetSubnetRootFundingPolicy {
                funding_profile: FleetFundingProfile::MultiSubnet,
                request_threshold: Cycles::new(900),
                target_balance: Cycles::new(1_000),
                cooldown_secs: 60,
                budget,
                maximum_automatic_grants: 1,
                maximum_automatic_cycles: Cycles::new(100),
            },
            icp_refill: None,
        },
    };
    let entry = FleetSubnetRootEntry {
        placement_subnet: binding.placement_subnet,
        fleet_subnet_root: binding.fleet_subnet_root,
        component_admissions: binding.component_admissions.clone(),
        component_topology_digest: binding.component_topology_digest,
        active_release_set: FleetSubnetRootReleaseSet {
            release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                [9; 32],
            )),
            manifest_digest: ReleaseSetDigest::from_bytes([10; 32]),
        },
        limits: binding.limits.clone(),
        funding: binding.funding.clone(),
        status: FleetSubnetRootStatus::Active,
    };
    let context = PoolImportContext {
        maximum_call_debit_cycles: 1,
        bootstrap: None,
        root_authority_sha256: CanisterPoolApi::import_authority_hash(&binding).unwrap(),
        binding,
        next_sequence: 0,
        active_import: None,
    };
    let mut reviewed = initial.authority;
    reviewed.root_authority_sha256 = context.root_authority_sha256;
    let plan = prepare_review(reviewed, initial.sources, initial.root_budget).unwrap();
    let registry = FleetRegistry {
        authority,
        revision: 1,
        admission: FleetAdmissionPolicy {
            schema_version: 1,
            fleet: plan.authority.fleet.clone(),
            generation: 1,
            fleet_principals: vec![plan.authority.operator],
            rules: vec![],
            policy_digest: [11; 32],
        },
        component_specs: vec![],
        fleet_subnet_roots: vec![entry],
        services: vec![],
    };
    (plan, context, registry)
}

#[test]
fn capacity_import_coordinator_can_register_a_root_on_a_different_subnet() {
    let (plan, context, registry) = fixture();
    assert_ne!(
        registry.authority.binding.coordinator_subnet,
        plan.authority.subnet
    );
    assert_eq!(
        validate_destination_authority(&plan, &context, &registry),
        Ok(())
    );
}

#[test]
fn capacity_import_handoff_requires_its_current_root_reservation() {
    let (plan, mut context, _) = fixture();
    assert_eq!(
        require_active_reservation(&plan, &context),
        Err(CapacityImportPrerequisiteError::RootReservationMissing)
    );
    let identity = PoolImportIdentity {
        sequence: plan.authority.import_sequence,
        plan_sha256: plan.plan_sha256,
    };
    context.active_import = Some(identity);
    assert_eq!(require_active_reservation(&plan, &context), Ok(()));
    context.active_import.as_mut().unwrap().sequence += 1;
    assert_eq!(
        require_active_reservation(&plan, &context),
        Err(CapacityImportPrerequisiteError::RootReservationMissing)
    );
    context.active_import = Some(identity);
    context.active_import.as_mut().unwrap().plan_sha256[0] ^= 1;
    assert_eq!(
        require_active_reservation(&plan, &context),
        Err(CapacityImportPrerequisiteError::RootReservationMissing)
    );
}

#[test]
fn capacity_import_requires_one_active_coordinator_registration() {
    let (plan, context, registry) = fixture();
    for status in [
        FleetSubnetRootStatus::Joining,
        FleetSubnetRootStatus::Draining,
        FleetSubnetRootStatus::Removed,
    ] {
        let mut changed = registry.clone();
        changed.fleet_subnet_roots[0].status = status;
        assert_eq!(
            validate_destination_authority(&plan, &context, &changed),
            Err(CapacityImportPrerequisiteError::RootNotActive)
        );
    }
    let mut changed = registry.clone();
    changed.fleet_subnet_roots.clear();
    assert_eq!(
        validate_destination_authority(&plan, &context, &changed),
        Err(CapacityImportPrerequisiteError::RootNotRegistered)
    );
    changed = registry.clone();
    changed
        .fleet_subnet_roots
        .push(registry.fleet_subnet_roots[0].clone());
    assert_eq!(
        validate_destination_authority(&plan, &context, &changed),
        Err(CapacityImportPrerequisiteError::RootNotRegistered)
    );
}

#[test]
fn capacity_import_rejects_substituted_coordinator_and_root_policy() {
    let (plan, context, registry) = fixture();
    let mut changed = registry.clone();
    changed.authority.epoch += 1;
    assert_eq!(
        validate_destination_authority(&plan, &context, &changed),
        Err(CapacityImportPrerequisiteError::CoordinatorAuthorityMismatch)
    );
    changed = registry.clone();
    changed.authority.binding.coordinator = principal(80);
    assert_eq!(
        validate_destination_authority(&plan, &context, &changed),
        Err(CapacityImportPrerequisiteError::CoordinatorAuthorityMismatch)
    );
    let mutations: [fn(&mut FleetSubnetRootEntry); 3] = [
        |e| e.placement_subnet = SubnetId::from_principal(principal(81)),
        |e| e.limits.canister_pool.maximum_size += 1,
        |e| e.funding.root_funding.maximum_automatic_grants += 1,
    ];
    for mutate in mutations {
        changed = registry.clone();
        mutate(&mut changed.fleet_subnet_roots[0]);
        assert_eq!(
            validate_destination_authority(&plan, &context, &changed),
            Err(CapacityImportPrerequisiteError::RootRegistrationMismatch)
        );
    }
    let mut changed_context = context;
    changed_context.binding.limits.canister_pool.maximum_size += 1;
    assert_eq!(
        validate_destination_authority(&plan, &changed_context, &registry),
        Err(CapacityImportPrerequisiteError::RootAuthorityMismatch)
    );
}

#[test]
fn capacity_import_bootstrap_requires_exact_source_set_and_operator() {
    let (plan, mut context, registry) = fixture();
    context.bootstrap = Some(canic_core::dto::pool_import::PoolImportBootstrap {
        review_sha256: [43; 32],
        operator: plan.authority.operator,
        sources: plan
            .sources
            .iter()
            .map(|source| source.binding.canister_id)
            .collect(),
    });
    validate_destination_authority(&plan, &context, &registry).unwrap();
    context.bootstrap.as_mut().unwrap().sources.clear();
    assert_eq!(
        validate_destination_authority(&plan, &context, &registry),
        Err(CapacityImportPrerequisiteError::BootstrapReservationMismatch)
    );
    context.bootstrap.as_mut().unwrap().sources = plan
        .sources
        .iter()
        .map(|source| source.binding.canister_id)
        .collect();
    context.bootstrap.as_mut().unwrap().operator = plan.authority.root;
    assert_eq!(
        validate_destination_authority(&plan, &context, &registry),
        Err(CapacityImportPrerequisiteError::BootstrapReservationMismatch)
    );
}
