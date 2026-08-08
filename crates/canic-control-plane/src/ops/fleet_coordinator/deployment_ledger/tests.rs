//! Focused proofs for terminal fresh-install deployment-ledger materialization.

use super::*;
use crate::storage::stable::fleet_coordinator::{
    FleetComponentProvisioningStateRecord, FleetComponentRuntimeActivationRecord,
};
use candid::Principal;
use canic_core::{
    bootstrap::parse_config_model,
    cdk::types::Cycles,
    dto::{
        component_provisioning::{
            ComponentGroupPlacementPlan, FleetComponentActivationRootProgress,
            FleetComponentProvisioningOperation, FleetComponentProvisioningPlan,
            FleetSubnetRootProvisioningBatch,
        },
        fleet_registry::FleetRegistryVersion,
    },
    ids::{
        AppId, CanonicalNetworkId, ComponentGroupPlacementId, ComponentTopologyDigest,
        CyclesFundingBudget, FleetBinding, FleetCoordinatorBinding, FleetId, FleetKey,
        FleetRegistryAuthority, FleetSubnetCanisterPoolConfig, FleetSubnetRootBinding,
        FleetSubnetRootLimits, FleetSubnetRootReleaseSet, ReleaseBuildId, ReleaseBuildNonce,
        ReleaseSetDigest, SubnetId,
    },
};

const CONFIG: &str = r#"
[app]
name = "deployment_ledger"

[roles.root]
kind = "root"
package = "root"

[roles.worker]
kind = "canister"
package = "worker"

[component_specs.workers]
component_role = "worker"
maximum_instances = 4

[component_groups.cell.components.worker]
component_spec = "workers"

[component_group_deployments.cells]
component_group = "cell"
initial_placements = 2
maximum_placements = 4
placement.maximum_per_root = 2
placement.minimum_distinct_roots = 1
"#;

#[test]
fn terminal_fresh_plan_compiles_one_canonical_protected_deployment_ledger() {
    let (configuration, provisioning) = fixture();
    let deployments = compile_initial(&configuration, &provisioning).expect("deployment ledger");

    assert_eq!(deployments.len(), 1);
    let deployment = &deployments[0];
    assert_eq!(deployment.deployment.as_ref(), "cells");
    assert_eq!(deployment.component_group.as_ref(), "cell");
    assert_eq!(
        deployment.configuration_digest,
        provisioning.plan.configuration_digest
    );
    assert_eq!(deployment.initial_placements, 2);
    assert_eq!(deployment.maximum_placements, 4);
    assert_eq!(deployment.placement_policy.maximum_per_root, 2);
    assert_eq!(deployment.placement_policy.minimum_distinct_roots, 1);
    assert_eq!(deployment.next_placement_ordinal, 2);
    assert_eq!(
        deployment
            .placements
            .iter()
            .map(|placement| placement.placement.ordinal)
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
    assert_eq!(deployment.placements[0].fleet_subnet_root, principal(10));
    assert_eq!(deployment.placements[1].fleet_subnet_root, principal(11));
    assert_eq!(deployment.placements[0].root_receipt_content_hash, [40; 32]);
    assert_eq!(deployment.placements[1].root_receipt_content_hash, [41; 32]);
    validate(&configuration, Some(&provisioning), &deployments).expect("exact ledger validates");
}

#[test]
fn deployment_ledger_rejects_premature_corrupt_and_unbound_receipt_state() {
    let (configuration, mut provisioning) = fixture();
    let deployments = compile_initial(&configuration, &provisioning).expect("deployment ledger");

    let terminal = provisioning.state.clone();
    provisioning.state = FleetComponentProvisioningStateRecord::Planned { planned_at_ns: 1 };
    assert!(validate(&configuration, Some(&provisioning), &deployments).is_err());
    provisioning.state = terminal;

    let mut corrupted = deployments;
    corrupted[0].next_placement_ordinal = 1;
    assert!(validate(&configuration, Some(&provisioning), &corrupted).is_err());

    let FleetComponentProvisioningStateRecord::RuntimesActivated { activations, .. } =
        &mut provisioning.state
    else {
        panic!("terminal fixture state")
    };
    activations[0].receipt_content_hash = [0; 32];
    assert!(compile_initial(&configuration, &provisioning).is_err());
}

fn fixture() -> (
    ComponentDeploymentConfiguration,
    FleetComponentProvisioningRecord,
) {
    let config = parse_config_model(CONFIG).expect("valid config");
    let configuration = config
        .compile_component_deployment_configuration()
        .expect("compiled deployment configuration");
    let configured = &configuration
        .deployment_topology
        .component_group_deployments[0];
    let configuration_digest = configuration.digest().expect("configuration digest");
    let authority = authority();
    let batches = [10_u8, 11]
        .into_iter()
        .enumerate()
        .map(|(ordinal, root)| FleetSubnetRootProvisioningBatch {
            root: root_binding(authority.clone(), root),
            active_release_set: release_set(),
            placements: vec![ComponentGroupPlacementPlan {
                group_placement: ComponentGroupPlacementId {
                    deployment: configured.deployment.clone(),
                    ordinal: u32::try_from(ordinal).expect("bounded ordinal"),
                },
                component_group: configured.component_group.clone(),
                entries: vec![],
            }],
        })
        .collect::<Vec<_>>();
    let fleet_registry = FleetRegistryVersion {
        authority: authority.clone(),
        revision: 2,
        content_hash: [20; 32],
    };
    let plan = FleetComponentProvisioningPlan {
        fleet: authority.binding.fleet,
        fleet_registry: fleet_registry.clone(),
        configuration_digest,
        operation: FleetComponentProvisioningOperation::FreshInstall,
        directory_confirmation_roots: vec![principal(10), principal(11)],
        batches,
    };
    let activations = [10_u8, 11]
        .into_iter()
        .enumerate()
        .map(|(index, root)| FleetComponentRuntimeActivationRecord {
            started_at_ns: 20,
            progress: FleetComponentActivationRootProgress {
                fleet_subnet_root: principal(root),
                component_count: 0,
                activated_component_count: 0,
                root_runtime_active: true,
            },
            activation: None,
            activation_started_at_ns: Some(21),
            runtimes_activated_at_ns: Some(22),
            receipt_content_hash: [40 + u8::try_from(index).expect("bounded index"); 32],
            recorded_at_ns: 23,
        })
        .collect();
    let provisioning = FleetComponentProvisioningRecord {
        operation_id: [30; 32],
        plan_hash: [31; 32],
        plan,
        state: FleetComponentProvisioningStateRecord::RuntimesActivated {
            planned_at_ns: 1,
            acceptances: vec![],
            roots_accepted_at_ns: 2,
            provisions: vec![],
            components_provisioned_at_ns: 3,
            published_fleet_registry: fleet_registry,
            service_topology_published_at_ns: 4,
            confirmations: vec![],
            directories_confirmed_at_ns: 5,
            activations,
            runtimes_activated_at_ns: 6,
        },
    };
    (configuration, provisioning)
}

fn authority() -> FleetRegistryAuthority {
    FleetRegistryAuthority {
        binding: FleetCoordinatorBinding {
            fleet: FleetBinding {
                fleet: FleetKey {
                    canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                    fleet_id: FleetId::from_generated_bytes([1; 32]),
                },
                app: AppId::from("deployment_ledger"),
            },
            coordinator_subnet: subnet(2),
            coordinator: principal(3),
        },
        epoch: 1,
    }
}

fn root_binding(authority: FleetRegistryAuthority, root: u8) -> FleetSubnetRootBinding {
    FleetSubnetRootBinding {
        authority,
        placement_subnet: subnet(root),
        fleet_subnet_root: principal(root),
        component_admissions: vec![],
        component_topology_digest: ComponentTopologyDigest::from_bytes([root; 32]),
        limits: FleetSubnetRootLimits {
            maximum_component_instances: 4,
            maximum_registry_bytes: 1_024,
            maximum_wasm_store_bytes: 1_024,
            canister_pool: FleetSubnetCanisterPoolConfig {
                minimum_size: 2,
                maximum_size: 4,
                canister_cycles: Cycles::new(1),
            },
            cycles_funding: CyclesFundingBudget {
                window_secs: 1,
                maximum_cycles: Cycles::new(1),
            },
            maximum_group_placements: 4,
        },
    }
}

fn release_set() -> FleetSubnetRootReleaseSet {
    FleetSubnetRootReleaseSet {
        release_build_id: ReleaseBuildId::from_nonce(ReleaseBuildNonce::from_random_bytes([1; 32])),
        manifest_digest: ReleaseSetDigest::from_bytes([2; 32]),
    }
}

fn principal(byte: u8) -> Principal {
    Principal::from_slice(&[byte; 29])
}

fn subnet(byte: u8) -> SubnetId {
    SubnetId::from_principal(principal(byte))
}
