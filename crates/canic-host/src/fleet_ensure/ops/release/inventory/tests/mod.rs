//! Ownership collection qualification is distinct from real role quiescence qualification.

mod pocketic;

use super::*;
use crate::fleet_ensure::policy::release::tests::fixture as release_fixture;
use canic_contracts::{
    cycles::Cycles,
    dto::fleet_registry::{FleetSubnetRootEntry, FleetSubnetRootStatus},
    ids::{
        ComponentTopologyDigest, CyclesFundingBudget, FleetAdmissionPolicy,
        FleetCoordinatorBinding, FleetRegistryAuthority, FleetSubnetCanisterPoolConfig,
        FleetSubnetRootLimits, FleetSubnetRootReleaseSet, ReleaseBuildId, ReleaseBuildNonce,
        ReleaseSetDigest,
    },
};

pub(in crate::fleet_ensure::ops::release) fn fixture() -> (FleetReleaseReviewRecord, FleetRegistry)
{
    let (review, _) = release_fixture();
    let registry = FleetRegistry {
        authority: FleetRegistryAuthority {
            binding: FleetCoordinatorBinding {
                fleet: review.authority.fleet.clone(),
                coordinator_subnet: review.sources[0].binding.subnet,
                coordinator: review.authority.coordinator,
                recovery_controllers: vec![review.authority.operator],
            },
            epoch: 1,
        },
        revision: 1,
        admission: FleetAdmissionPolicy {
            schema_version: 1,
            fleet: review.authority.fleet.clone(),
            generation: 1,
            fleet_principals: vec![review.authority.operator],
            rules: vec![],
            policy_digest: [11; 32],
        },
        component_specs: vec![],
        services: vec![],
        fleet_subnet_roots: vec![FleetSubnetRootEntry {
            placement_subnet: review.sources[1].binding.subnet,
            fleet_subnet_root: review.sources[1].binding.canister_id,
            component_admissions: vec![],
            component_topology_digest: ComponentTopologyDigest::from_bytes([8; 32]),
            active_release_set: FleetSubnetRootReleaseSet {
                release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                    [9; 32],
                )),
                manifest_digest: ReleaseSetDigest::from_bytes([10; 32]),
            },
            limits: FleetSubnetRootLimits {
                maximum_component_instances: 10,
                maximum_registry_bytes: 100_000,
                maximum_wasm_store_bytes: 100_000,
                canister_pool: FleetSubnetCanisterPoolConfig {
                    minimum_size: 0,
                    maximum_size: 10,
                    canister_cycles: Cycles::new(1_000),
                    creation_execution_margin: Cycles::new(100),
                },
                cycles_funding: CyclesFundingBudget {
                    window_secs: 60,
                    maximum_cycles: Cycles::new(1_000),
                },
                maximum_group_placements: 10,
            },
            funding: crate::test_support::fleet_subnet_root_funding_authority(),
            status: FleetSubnetRootStatus::Active,
        }],
    };
    (review, registry)
}

#[test]
fn registry_selection_requires_every_exact_owner_and_subnet() {
    let (review, registry) = fixture();
    let observed = validate_registry_selection(&review, &registry).unwrap();
    assert_eq!(observed.children, expected_ownership(&review).unwrap());
    assert_ne!(
        review.sources[0].binding.subnet,
        review.sources[1].binding.subnet
    );
    for change in [
        |registry: &mut FleetRegistry| {
            registry.fleet_subnet_roots.clear();
        },
        |registry: &mut FleetRegistry| {
            registry
                .fleet_subnet_roots
                .push(registry.fleet_subnet_roots[0].clone());
        },
    ] {
        let mut wrong = registry.clone();
        change(&mut wrong);
        assert_eq!(
            validate_registry_selection(&review, &wrong),
            Err(FleetReleaseError::Inventory)
        );
    }
    let mut wrong = registry.clone();
    wrong.authority.binding.coordinator = review.authority.operator;
    assert_eq!(
        validate_registry_selection(&review, &wrong),
        Err(FleetReleaseError::Authority)
    );
    wrong = registry.clone();
    wrong.fleet_subnet_roots[0].placement_subnet = review.sources[0].binding.subnet;
    assert!(matches!(
        validate_registry_selection(&review, &wrong),
        Err(FleetReleaseError::Custody { .. })
    ));
    let mut missing_store = review.clone();
    missing_store.sources.remove(2);
    assert_eq!(
        validate_registry_selection(&missing_store, &registry),
        Err(FleetReleaseError::Inventory)
    );
    let mut duplicate = review;
    duplicate.sources.push(duplicate.sources[3].clone());
    assert_eq!(
        validate_registry_selection(&duplicate, &registry),
        Err(FleetReleaseError::Inventory)
    );
}

#[test]
fn complete_inventory_does_not_establish_quiescence() {
    let (review, registry) = fixture();
    let inventory = validate_registry_selection(&review, &registry).unwrap();
    let (_, mut observed) = release_fixture();
    for owner in &mut observed.owners {
        owner.children = inventory.children[&owner.owner].iter().copied().collect();
        owner.producers_quiescent = false;
    }
    assert!(matches!(
        crate::fleet_ensure::policy::release::validate_review(&review, &observed),
        Err(FleetReleaseError::Unfinished { .. })
    ));
}
