use super::FleetSubnetRootDrainingReservationOps;
use crate::{
    dto::fleet_registry::{
        FleetRegistryVersion, FleetSubnetRootDrainingReservationRequest,
        FleetSubnetRootDrainingReservationResponse, FleetSubnetRootEntry, FleetSubnetRootStatus,
    },
    ids::{
        AppId, CanonicalNetworkId, ComponentTopologyDigest, CyclesFundingBudget, FleetBinding,
        FleetCoordinatorBinding, FleetId, FleetKey, FleetRegistryAuthority,
        FleetSubnetCanisterPoolConfig, FleetSubnetRootLimits, FleetSubnetRootReleaseSet,
        ReleaseBuildId, ReleaseBuildNonce, ReleaseSetDigest, SubnetId,
    },
};
use candid::Principal;

#[test]
fn reservation_hash_is_domain_separated_and_excludes_only_its_hash_field() {
    let mut response = fixture();
    let hash = FleetSubnetRootDrainingReservationOps::content_hash(&response)
        .expect("hash draining reservation");
    assert_eq!(
        crate::cdk::utils::hash::hex_bytes(hash),
        "2c78d37f15fef3b56a2b17f06f5fd7bf4559009dfc0bb9d96c13f5a93cd6cf74"
    );
    response.reservation_hash = [99; 32];
    assert_eq!(
        FleetSubnetRootDrainingReservationOps::content_hash(&response)
            .expect("rehash draining reservation"),
        hash
    );

    response.prepared_at_ns += 1;
    assert_ne!(
        FleetSubnetRootDrainingReservationOps::content_hash(&response)
            .expect("hash changed draining reservation"),
        hash
    );
}

fn fixture() -> FleetSubnetRootDrainingReservationResponse {
    let coordinator = Principal::from_slice(&[3; 29]);
    let authority = FleetRegistryAuthority {
        binding: FleetCoordinatorBinding {
            fleet: FleetBinding {
                fleet: FleetKey {
                    canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                    fleet_id: FleetId::from_generated_bytes([1; 32]),
                },
                app: AppId::from("test"),
            },
            coordinator_subnet: SubnetId::from_principal(Principal::from_slice(&[2; 29])),
            coordinator,
        },
        epoch: 1,
    };
    let expected_registry = FleetRegistryVersion {
        authority,
        revision: 7,
        content_hash: [4; 32],
    };
    FleetSubnetRootDrainingReservationResponse {
        request: FleetSubnetRootDrainingReservationRequest {
            operation_id: [5; 32],
            expected_registry,
            expected_root: FleetSubnetRootEntry {
                fleet_subnet_root: Principal::from_slice(&[6; 29]),
                placement_subnet: SubnetId::from_principal(Principal::from_slice(&[7; 29])),
                status: FleetSubnetRootStatus::Active,
                component_admissions: Vec::new(),
                component_topology_digest: ComponentTopologyDigest::from_bytes([8; 32]),
                active_release_set: FleetSubnetRootReleaseSet {
                    release_build_id: ReleaseBuildId::from_nonce(
                        ReleaseBuildNonce::from_random_bytes([9; 32]),
                    ),
                    manifest_digest: ReleaseSetDigest::from_bytes([10; 32]),
                },
                limits: FleetSubnetRootLimits {
                    maximum_component_instances: 11,
                    maximum_registry_bytes: 12,
                    maximum_wasm_store_bytes: 13,
                    canister_pool: FleetSubnetCanisterPoolConfig {
                        minimum_size: 1,
                        maximum_size: 2,
                        canister_cycles: crate::cdk::types::Cycles::new(1_000_000_000_000),
                        creation_execution_margin: crate::cdk::types::Cycles::new(
                            1_000_000_000_000,
                        ),
                    },
                    cycles_funding: CyclesFundingBudget {
                        window_secs: 3_600,
                        maximum_cycles: crate::cdk::types::Cycles::new(2_000_000_000_000),
                    },
                    maximum_group_placements: 14,
                },
                funding: crate::test::support::fleet_subnet_root_funding_authority(),
            },
        },
        coordinator,
        prepared_at_ns: 15,
        reservation_hash: [0; 32],
    }
}
