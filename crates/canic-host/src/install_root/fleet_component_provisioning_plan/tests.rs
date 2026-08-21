//! Focused tests for explicit initial Component placement compilation.

use super::*;
use crate::fleet_install_plan::{
    FleetInstallPlan, PlannedCanisterCreationFunding, PlannedComponentGroupPlacementAssignment,
    PlannedFleetCoordinator, PlannedFleetSubnetRoot,
};
use candid::Principal;
use canic_core::{
    bootstrap::parse_config_model,
    cdk::types::Cycles,
    control_plane_support::ops::fleet_registry::FleetRegistryOps,
    dto::fleet_registry::{FleetRegistry, FleetSubnetRootEntry, FleetSubnetRootStatus},
    ids::{
        AppId, CanonicalNetworkId, ComponentSpecAdmission, CyclesFundingBudget, FleetBinding,
        FleetCoordinatorBinding, FleetId, FleetKey, FleetRegistryAuthority,
        FleetSubnetCanisterPoolConfig, FleetSubnetRootLimits, FleetSubnetRootReleaseSet,
        ReleaseBuildId, ReleaseBuildNonce, ReleaseSetDigest, SubnetId,
    },
};

const CONFIG: &str = r#"
[app]
name = "plan_test"

[roles.root]
kind = "root"
package = "root"

[roles.alpha]
kind = "canister"
package = "alpha"

[roles.beta]
kind = "canister"
package = "beta"

[component_specs.alpha]
component_role = "alpha"
maximum_instances = 4

[component_specs.beta]
component_role = "beta"
maximum_instances = 4

[component_groups.cell.components.alpha]
component_spec = "alpha"
labels = { tier = "api" }

[component_groups.cell.components.beta]
component_spec = "beta"

[component_group_deployments.cells]
component_group = "cell"
initial_placements = 2
maximum_placements = 4
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 2
"#;

#[test]
fn explicit_subnet_assignments_compile_to_exact_principal_ordered_batches() {
    let config = parse_config_model(CONFIG).expect("valid deployment config");
    let (plan, registry) = authorities(&config);
    let compiled =
        compile_fleet_component_provisioning_plan(CompileFleetComponentProvisioningPlanRequest {
            config: &config,
            fleet_install_plan: &plan,
            registry: &registry,
            operation_id: [42; 32],
        })
        .expect("compile explicit placement plan");

    let batches = &compiled.prepare_request.plan.batches;
    assert_eq!(batches.len(), 3);
    assert!(batches.is_sorted_by_key(|batch| batch.root.fleet_subnet_root));
    let placements = batches
        .iter()
        .filter_map(|batch| {
            batch.placements.first().map(|placement| {
                (
                    batch.root.placement_subnet,
                    placement.group_placement.ordinal,
                )
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(placements, vec![(subnet(7), 1), (subnet(6), 0)]);
    assert_eq!(batches[0].placements[0].entries.len(), 2);
    assert!(batches[1].placements.is_empty());
    assert_eq!(
        compiled.prepare_request.plan.directory_confirmation_roots,
        vec![principal(10), principal(15), principal(20)]
    );
    assert_ne!(compiled.plan_hash, [0; 32]);
}

#[test]
fn live_registry_cannot_substitute_another_fleet_authority() {
    let config = parse_config_model(CONFIG).expect("valid deployment config");
    let (plan, mut registry) = authorities(&config);
    registry.authority.binding.fleet.fleet.fleet_id = FleetId::from_generated_bytes([99; 32]);

    assert!(matches!(
        compile_fleet_component_provisioning_plan(CompileFleetComponentProvisioningPlanRequest {
            config: &config,
            fleet_install_plan: &plan,
            registry: &registry,
            operation_id: [42; 32],
        }),
        Err(FleetComponentProvisioningPlanError::FleetAuthorityMismatch)
    ));
}

fn authorities(
    config: &canic_core::bootstrap::compiled::ConfigModel,
) -> (FleetInstallPlan, FleetRegistry) {
    let topology = config
        .compile_component_topology()
        .expect("Component topology");
    let fleet = FleetBinding {
        fleet: FleetKey {
            canonical_network_id: CanonicalNetworkId::ic_mainnet(),
            fleet_id: FleetId::from_generated_bytes([7; 32]),
        },
        app: AppId::from("plan_test"),
    };
    let authority = FleetRegistryAuthority {
        binding: FleetCoordinatorBinding {
            fleet: fleet.clone(),
            coordinator_subnet: subnet(1),
            coordinator: principal(30),
        },
        epoch: 1,
    };
    let release_set = FleetSubnetRootReleaseSet {
        release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([8; 32])),
        manifest_digest: ReleaseSetDigest::from_bytes([9; 32]),
    };
    let admissions = [admission(&topology, "alpha"), admission(&topology, "beta")];
    let roots = vec![
        planned_root(&topology, subnet(6), Some(0), &admissions, release_set),
        planned_root(&topology, subnet(7), Some(1), &admissions, release_set),
        planned_root(&topology, subnet(8), None, &admissions, release_set),
    ];
    let mut registry = FleetRegistryOps::compile_genesis(&fleet.app, authority.clone(), &topology)
        .expect("genesis Registry");
    // Principal order deliberately opposes Subnet order.
    for (planned, canister) in roots
        .iter()
        .zip([principal(20), principal(10), principal(15)])
    {
        registry = FleetRegistryOps::compile_joining(
            &authority,
            &topology,
            &registry,
            FleetSubnetRootEntry {
                placement_subnet: planned.placement_subnet,
                fleet_subnet_root: canister,
                component_admissions: planned.component_admissions.clone(),
                component_topology_digest: planned.component_topology_digest,
                active_release_set: planned.initial_release_set,
                limits: planned.limits.clone(),
                funding: planned.funding.clone(),
                status: FleetSubnetRootStatus::Joining,
            },
        )
        .expect("Joining root");
    }
    let registry = FleetRegistryOps::compile_active(&authority, &topology, &registry)
        .expect("active Registry");
    (
        FleetInstallPlan {
            fleet,
            fresh_fleet_plan_digest: "ab".repeat(32),
            release_build_id: release_set.release_build_id,
            application_artifact_union_digest: [6; 32],
            coordinator: PlannedFleetCoordinator {
                coordinator_subnet: subnet(1),
                creation_funding: funding(),
                root_funding: Some(crate::test_support::coordinator_root_funding_policy()),
            },
            fleet_subnet_roots: roots,
        },
        registry,
    )
}

fn planned_root(
    topology: &canic_core::bootstrap::compiled::ComponentTopology,
    placement_subnet: SubnetId,
    ordinal: Option<u32>,
    admissions: &[ComponentSpecAdmission; 2],
    release_set: FleetSubnetRootReleaseSet,
) -> PlannedFleetSubnetRoot {
    PlannedFleetSubnetRoot {
        placement_subnet,
        component_group_placements: ordinal
            .map(|ordinal| PlannedComponentGroupPlacementAssignment {
                deployment: "cells".parse().expect("deployment ID"),
                ordinal,
            })
            .into_iter()
            .collect(),
        component_admissions: admissions.to_vec(),
        component_topology_digest: topology
            .project_for_admissions(admissions)
            .expect("root topology")
            .digest()
            .expect("root topology digest"),
        initial_release_set: release_set,
        limits: limits(),
        funding: crate::test_support::fleet_subnet_root_funding_authority(),
        canister_pool_imports: Vec::new(),
        root_creation_funding: funding(),
        wasm_store_creation_funding: funding(),
    }
}

fn admission(
    topology: &canic_core::bootstrap::compiled::ComponentTopology,
    component: &str,
) -> ComponentSpecAdmission {
    let component_spec = component.parse().expect("Component Spec ID");
    ComponentSpecAdmission {
        spec_hash: topology
            .get(&component_spec)
            .expect("Component Spec")
            .spec_hash,
        component_spec,
        maximum_root_instances: 1,
    }
}

fn limits() -> FleetSubnetRootLimits {
    FleetSubnetRootLimits {
        maximum_component_instances: 4,
        maximum_registry_bytes: 16_777_216,
        maximum_wasm_store_bytes: 40_000_000,
        maximum_group_placements: 2,
        canister_pool: FleetSubnetCanisterPoolConfig {
            minimum_size: 1,
            maximum_size: 2,
            canister_cycles: Cycles::new(5_000_000_000_000),
        },
        cycles_funding: CyclesFundingBudget {
            window_secs: 3_600,
            maximum_cycles: Cycles::new(10_000_000_000_000),
        },
    }
}

fn funding() -> PlannedCanisterCreationFunding {
    PlannedCanisterCreationFunding::Cycles {
        cycles: 2_000_000_000_000,
    }
}

fn principal(byte: u8) -> Principal {
    Principal::from_slice(&[byte; 29])
}

fn subnet(byte: u8) -> SubnetId {
    SubnetId::from_principal(principal(byte))
}
