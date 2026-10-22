use super::placement;
use crate::registry::RegistryEntry;
use canic_contracts::{
    cycles::Cycles,
    dto::fleet_registry::{FleetRegistry, FleetSubnetRootEntry, FleetSubnetRootStatus},
    ids::{
        AppId, CanonicalNetworkId, ComponentTopologyDigest, CyclesFundingBudget,
        FleetAdmissionPolicy, FleetBinding, FleetCoordinatorBinding, FleetId, FleetKey,
        FleetRegistryAuthority, FleetSubnetCanisterPoolConfig, FleetSubnetRootLimits,
        FleetSubnetRootReleaseSet, ReleaseBuildId, ReleaseBuildNonce, ReleaseSetDigest, SubnetId,
    },
};
use ic_platform::principal::ExternalPrincipal as Principal;

fn entry(pid: Principal, parent: Option<Principal>, role: &str) -> RegistryEntry {
    RegistryEntry {
        pid: pid.to_text(),
        role: Some(role.into()),
        parent_pid: parent.map(|pid| pid.to_text()),
        module_hash: None,
        protocol_binding: None,
    }
}

#[test]
fn coordinator_placement_uses_its_exact_binding_and_children_keep_root_placement() {
    let coordinator = Principal::from_slice(&[1; 29]);
    let root = Principal::from_slice(&[2; 29]);
    let store = Principal::from_slice(&[3; 29]);
    let coordinator_subnet = SubnetId::from_principal(Principal::from_slice(&[4; 29]));
    let root_subnet = SubnetId::from_principal(Principal::from_slice(&[5; 29]));
    let fleet = FleetBinding {
        app: AppId::from("observatory_test"),
        fleet: FleetKey {
            canonical_network_id: CanonicalNetworkId::ic_mainnet(),
            fleet_id: FleetId::from_generated_bytes([6; 32]),
        },
    };
    let registry = FleetRegistry {
        authority: FleetRegistryAuthority {
            binding: FleetCoordinatorBinding {
                fleet: fleet.clone(),
                coordinator,
                coordinator_subnet,
                recovery_controllers: Vec::new(),
            },
            epoch: 1,
        },
        revision: 1,
        admission: FleetAdmissionPolicy {
            schema_version: 1,
            fleet,
            generation: 1,
            fleet_principals: Vec::new(),
            rules: Vec::new(),
            policy_digest: [0; 32],
        },
        component_specs: Vec::new(),
        services: Vec::new(),
        fleet_subnet_roots: vec![FleetSubnetRootEntry {
            fleet_subnet_root: root,
            placement_subnet: root_subnet,
            component_admissions: Vec::new(),
            component_topology_digest: ComponentTopologyDigest::from_bytes([7; 32]),
            active_release_set: FleetSubnetRootReleaseSet {
                release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                    [8; 32],
                )),
                manifest_digest: ReleaseSetDigest::from_bytes([9; 32]),
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
    let entries = [
        entry(coordinator, None, "fleet_coordinator"),
        entry(root, Some(coordinator), "root"),
        entry(store, Some(root), "wasm_store"),
    ];
    assert_ne!(coordinator_subnet, root_subnet);
    assert_eq!(
        placement(&registry, &entries, &entries[0]),
        Some(coordinator_subnet.to_string()),
    );
    for child in &entries[1..] {
        assert_eq!(
            placement(&registry, &entries, child),
            Some(root_subnet.to_string()),
        );
    }
    let unrelated = Principal::from_slice(&[10; 29]);
    for unplaced in [
        entry(unrelated, None, "fleet_coordinator"),
        entry(unrelated, Some(coordinator), "app"),
        entry(unrelated, Some(Principal::from_slice(&[11; 29])), "app"),
        entry(unrelated, Some(unrelated), "app"),
    ] {
        let mut inventory = entries.to_vec();
        inventory.push(unplaced.clone());
        assert_eq!(placement(&registry, &inventory, &unplaced), None);
    }
}
