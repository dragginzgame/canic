use super::*;
use crate::ids::{
    AppId, CanonicalNetworkId, ComponentInstanceId, FleetBinding, FleetCoordinatorBinding, FleetId,
    FleetKey, FleetRegistryAuthority, SubnetId,
};
use candid::Principal;

fn principal(byte: u8) -> Principal {
    Principal::from_slice(&[byte; 29])
}

fn component_binding() -> ComponentBinding {
    ComponentBinding {
        authority: FleetRegistryAuthority {
            binding: FleetCoordinatorBinding {
                fleet: FleetBinding {
                    fleet: FleetKey {
                        canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                        fleet_id: FleetId::from_generated_bytes([1; 32]),
                    },
                    app: AppId::from("env-test"),
                },
                coordinator_subnet: SubnetId::from_principal(principal(2)),
                coordinator: principal(3),
            },
            epoch: 1,
        },
        component: ComponentInstanceId::from_generated_bytes([4; 32]),
        component_spec: "worker".parse().expect("Component Spec ID"),
        spec_hash: [5; 32],
        role: CanisterRole::from("worker"),
        placement_subnet: SubnetId::from_principal(principal(6)),
        fleet_subnet_root: principal(7),
        canister_id: principal(8),
    }
}

#[test]
fn top_level_component_env_keeps_physical_subnet_distinct_from_root_canister() {
    let component = component_binding();
    let root = component.fleet_subnet_root;
    let physical_subnet = component.placement_subnet.into_principal();

    let env = validated_managed_env(root, ManagedCanisterBinding::Component(component));

    assert_eq!(env.subnet_pid, physical_subnet);
    assert_eq!(env.fleet_subnet_root_pid, root);
    assert_eq!(env.root_pid, root);
    assert_ne!(env.subnet_pid, env.root_pid);
}

#[test]
fn component_child_env_inherits_physical_subnet_and_keeps_immediate_parent() {
    let component = component_binding();
    let root = component.fleet_subnet_root;
    let physical_subnet = component.placement_subnet.into_principal();
    let parent = principal(9);
    let child = ComponentChildBinding {
        component,
        parent_canister_id: parent,
        role: CanisterRole::from("worker_child"),
        canister_id: principal(10),
    };

    let env = validated_managed_env(root, ManagedCanisterBinding::ComponentChild(child));

    assert_eq!(env.subnet_pid, physical_subnet);
    assert_eq!(env.fleet_subnet_root_pid, root);
    assert_eq!(env.root_pid, root);
    assert_eq!(env.parent_pid, parent);
}
