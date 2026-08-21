//! Focused tests for exact live Registry replay classification.

use super::*;
use candid::Principal;
use canic_core::{
    bootstrap::{
        compiled::{FleetServiceMemberPurpose, FleetServicePlacementPolicy},
        parse_config_model,
    },
    cdk::types::Cycles,
    dto::fleet_registry::{
        FleetServiceBinding, FleetServiceComponentBinding, FleetServiceMode, FleetSubnetRootEntry,
        FleetSubnetRootStatus,
    },
    ids::{
        AppId, CanonicalNetworkId, ComponentGroupMemberPath, ComponentGroupPlacementId,
        ComponentInstanceId, ComponentSpecAdmission, CyclesFundingBudget, FleetBinding,
        FleetCoordinatorBinding, FleetId, FleetKey, FleetRegistryAuthority,
        FleetSubnetCanisterPoolConfig, FleetSubnetRootLimits, FleetSubnetRootReleaseSet,
        ReleaseBuildId, ReleaseBuildNonce, ReleaseSetDigest, SubnetId,
    },
};

#[test]
fn verified_activation_accepts_only_the_exact_initial_service_successor() {
    let topology = topology();
    let active = active_registry(&topology);
    let successor = FleetRegistryOps::compile_initial_services(
        &active.authority,
        &topology,
        &active,
        vec![service()],
    )
    .expect("compile service successor");
    let exact_live = live_evidence(&topology, successor.clone());

    require_exact_or_service_successor_registry(&topology, &active, &exact_live)
        .expect("accept exact service successor");

    let mut later = successor;
    later.revision += 1;
    let later_live = live_evidence(&topology, later);
    assert!(
        require_exact_or_service_successor_registry(&topology, &active, &later_live).is_err(),
        "a later canonical Registry must not masquerade as the one service successor"
    );
}

fn topology() -> ComponentTopology {
    parse_config_model(
        r#"
[app]
name = "demo"

[roles.root]
kind = "root"
package = "root"

[roles.worker]
kind = "canister"
package = "worker"

[component_specs.worker]
component_role = "worker"
maximum_instances = 1
"#,
    )
    .expect("valid config")
    .compile_component_topology()
    .expect("Component Topology")
}

fn active_registry(topology: &ComponentTopology) -> FleetRegistry {
    let authority = authority();
    let mut registry =
        FleetRegistryOps::compile_genesis(&AppId::from("demo"), authority.clone(), topology)
            .expect("genesis Registry");
    registry = FleetRegistryOps::compile_joining(&authority, topology, &registry, root(topology))
        .expect("Joining root");
    FleetRegistryOps::compile_active(&authority, topology, &registry).expect("active Registry")
}

fn root(topology: &ComponentTopology) -> FleetSubnetRootEntry {
    let component_spec = "worker".parse().expect("Component Spec ID");
    let spec = topology.get(&component_spec).expect("Component Spec");
    let component_admissions = vec![ComponentSpecAdmission {
        component_spec,
        spec_hash: spec.spec_hash,
        maximum_root_instances: 1,
    }];
    FleetSubnetRootEntry {
        placement_subnet: subnet(5),
        fleet_subnet_root: principal(6),
        component_topology_digest: topology
            .project_for_admissions(&component_admissions)
            .expect("root projection")
            .digest()
            .expect("root topology digest"),
        component_admissions,
        active_release_set: FleetSubnetRootReleaseSet {
            release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes(
                [7; 32],
            )),
            manifest_digest: ReleaseSetDigest::from_bytes([8; 32]),
        },
        limits: root_limits(),
        funding: crate::test_support::fleet_subnet_root_funding_authority(),
        status: FleetSubnetRootStatus::Joining,
    }
}

fn service() -> FleetServiceBinding {
    FleetServiceBinding {
        service: "workers".parse().expect("service ID"),
        role: "worker".parse().expect("role"),
        component_spec: "worker".parse().expect("Component Spec ID"),
        mode: FleetServiceMode::ActivePool,
        placement: FleetServicePlacementPolicy {
            maximum_members_per_root: 1,
            minimum_distinct_roots: 1,
        },
        members: vec![FleetServiceComponentBinding {
            member_purpose: FleetServiceMemberPurpose::PoolMember,
            component: ComponentInstanceId::from_generated_bytes([9; 32]),
            fleet_subnet_root: principal(6),
            canister_id: principal(10),
            group_placement: ComponentGroupPlacementId {
                deployment: "workers".parse().expect("deployment ID"),
                ordinal: 0,
            },
            member_path: ComponentGroupMemberPath::try_from(vec![
                "worker".parse().expect("member ID"),
            ])
            .expect("member path"),
        }],
    }
}

fn live_evidence(topology: &ComponentTopology, registry: FleetRegistry) -> LiveRegistryEvidence {
    LiveRegistryEvidence {
        manifest: FleetRegistryOps::manifest(&registry.authority, topology, &registry)
            .expect("Registry manifest"),
        version: FleetRegistryOps::version(&registry.authority, topology, &registry)
            .expect("Registry version"),
        registry,
    }
}

fn authority() -> FleetRegistryAuthority {
    FleetRegistryAuthority {
        binding: FleetCoordinatorBinding {
            fleet: FleetBinding {
                fleet: FleetKey {
                    canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                    fleet_id: FleetId::from_generated_bytes([1; 32]),
                },
                app: AppId::from("demo"),
            },
            coordinator_subnet: subnet(2),
            coordinator: principal(3),
        },
        epoch: 1,
    }
}

fn root_limits() -> FleetSubnetRootLimits {
    FleetSubnetRootLimits {
        maximum_component_instances: 1,
        maximum_registry_bytes: 2_097_152,
        maximum_wasm_store_bytes: 40_000_000,
        maximum_group_placements: 1,
        canister_pool: FleetSubnetCanisterPoolConfig {
            minimum_size: 1,
            maximum_size: 1,
            canister_cycles: Cycles::new(5_000_000_000_000),
        },
        cycles_funding: CyclesFundingBudget {
            window_secs: 3_600,
            maximum_cycles: Cycles::new(10_000_000_000_000),
        },
    }
}

fn subnet(byte: u8) -> SubnetId {
    SubnetId::from_principal(principal(byte))
}

fn principal(byte: u8) -> Principal {
    Principal::from_slice(&[byte; 29])
}
