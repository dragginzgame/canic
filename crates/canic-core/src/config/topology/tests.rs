//! Module: config::topology::tests
//!
//! Responsibility: verify canonical Component Topology compilation, hashing, and admissions.
//! Does not own: host root placement, Registry persistence, or runtime lifecycle.
//! Boundary: exercises the complete validated-config to protected-topology transition.

use crate::{
    cdk::{candid::Principal, types::Cycles, utils::hash::hex_bytes},
    config::Config,
    ids::{
        AppId, CanonicalNetworkId, ComponentBinding, ComponentChildBinding, ComponentInstanceId,
        ComponentSpecAdmission, ComponentSpecId, CyclesFundingBudget, FleetBinding,
        FleetCoordinatorBinding, FleetId, FleetKey, FleetRegistryAuthority, FleetSubnetRootBinding,
        FleetSubnetRootLimits, SubnetId,
    },
};

use super::*;

const CONFIG: &str = r#"
[app]
name = "toko"

[roles.root]
kind = "root"
package = "root"

[roles.user_hub]
kind = "canister"
package = "user_hub"

[roles.user_shard]
kind = "canister"
package = "user_shard"

[roles.project_hub]
kind = "canister"
package = "project_hub"

[roles.project_instance]
kind = "canister"
package = "project_instance"

[component_specs.users]
component_role = "user_hub"
maximum_instances = 2

[component_specs.users.children.user_shard]
kind = "shard"
maximum_instances = 8

[component_specs.users.sharding.pools.user_shards]
canister_role = "user_shard"
policy.capacity = 100
policy.initial_shards = 1
policy.max_shards = 8

[component_specs.projects]
component_role = "project_hub"
maximum_instances = 3

[component_specs.projects.limits]
maximum_children = 100
maximum_registry_bytes = 1048576

[component_specs.projects.limits.cycles_funding]
window_secs = 600
maximum_cycles = "250T"

[component_specs.projects.children.project_instance]
kind = "instance"
maximum_instances = 100

[component_specs.projects.binding.pools.projects]
canister_role = "project_instance"
key_name = "project_id"
"#;

fn component_spec(value: &str) -> ComponentSpecId {
    value.parse().expect("valid Component Spec ID")
}

fn topology() -> ComponentTopology {
    let config = Config::parse_toml(CONFIG).expect("valid Component config");
    ComponentTopology::compile(&config).expect("compile Component Topology")
}

fn admission(
    topology: &ComponentTopology,
    component_spec_id: &str,
    maximum_root_instances: u32,
) -> ComponentSpecAdmission {
    let component_spec = topology
        .get(&component_spec(component_spec_id))
        .expect("compiled Component Spec");
    ComponentSpecAdmission {
        component_spec: component_spec.component_spec.clone(),
        spec_hash: component_spec.spec_hash,
        maximum_root_instances,
    }
}

fn authority() -> FleetRegistryAuthority {
    FleetRegistryAuthority {
        binding: FleetCoordinatorBinding {
            fleet: FleetBinding {
                fleet: FleetKey {
                    canonical_network_id: CanonicalNetworkId::public_ic(),
                    fleet_id: FleetId::from_generated_bytes([1; 32]),
                },
                app: AppId::from("toko"),
            },
            coordinator_subnet: SubnetId::from_principal(Principal::from_slice(&[2; 29])),
            coordinator: Principal::from_slice(&[3; 29]),
        },
        epoch: 1,
    }
}

fn root_binding(
    topology: &ComponentTopology,
    root_byte: u8,
    subnet_byte: u8,
    admissions: Vec<ComponentSpecAdmission>,
) -> FleetSubnetRootBinding {
    let projection = topology
        .project_for_admissions(&admissions)
        .expect("root topology projection");
    FleetSubnetRootBinding {
        authority: authority(),
        placement_subnet: SubnetId::from_principal(Principal::from_slice(&[subnet_byte; 29])),
        fleet_subnet_root: Principal::from_slice(&[root_byte; 29]),
        component_admissions: admissions,
        component_topology_digest: projection.digest().expect("root topology digest"),
        limits: FleetSubnetRootLimits {
            maximum_component_instances: 10,
            maximum_managed_canisters: 1_000,
            maximum_registry_bytes: 4_194_304,
            maximum_wasm_store_bytes: 40_000_000,
            cycles_funding: CyclesFundingBudget {
                window_secs: 3_600,
                maximum_cycles: Cycles::new(1_000_000_000_000_000),
            },
        },
    }
}

fn component_binding(root: &FleetSubnetRootBinding) -> ComponentBinding {
    let admission = &root.component_admissions[0];
    ComponentBinding {
        authority: root.authority.clone(),
        component: ComponentInstanceId::from_generated_bytes([9; 32]),
        component_spec: admission.component_spec.clone(),
        spec_hash: admission.spec_hash,
        role: CanisterRole::from("project_hub"),
        placement_subnet: root.placement_subnet,
        fleet_subnet_root: root.fleet_subnet_root,
        canister_id: Principal::from_slice(&[10; 29]),
    }
}

#[test]
fn topology_compiles_specs_and_children_in_canonical_order() {
    let topology = topology();

    assert_eq!(
        topology
            .component_specs
            .iter()
            .map(|spec| spec.component_spec.as_str())
            .collect::<Vec<_>>(),
        vec!["projects", "users"],
    );
    assert_eq!(
        topology.component_specs[0]
            .children
            .iter()
            .map(|child| child.role.as_str())
            .collect::<Vec<_>>(),
        vec!["project_instance"],
    );
    assert_eq!(topology.component_specs[0].limits.maximum_children, 100);
    assert_eq!(
        topology.component_specs[0]
            .limits
            .cycles_funding
            .maximum_cycles
            .to_u128(),
        250_000_000_000_000,
    );
}

#[test]
fn canonical_spec_and_topology_hashes_match_frozen_golden_values() {
    let topology = topology();

    assert_eq!(
        hex_bytes(topology.component_specs[0].spec_hash),
        "b2e168a770160a4659512fc52c7199414cbf7aec2be2965914bd7a6de69f9dea",
    );
    assert_eq!(
        hex_bytes(topology.component_specs[1].spec_hash),
        "0f217c753d667e93e5b2a6ecf71c82b0ca273d61a7b4a9e508eed57cc0232ab5",
    );
    assert_eq!(
        topology.digest().expect("topology digest").to_string(),
        "35fa3b9f66bd647790b81cce4f7b79bce530d2006ab676ccd6ae66177f6f8faa",
    );
}

#[test]
fn spec_hash_binds_package_limits_pools_and_child_policy() {
    let config = Config::parse_toml(CONFIG).expect("valid Component config");
    let baseline = ComponentTopology::compile(&config)
        .expect("baseline topology")
        .get(&component_spec("projects"))
        .expect("projects")
        .spec_hash;

    let mut package = config.clone();
    package
        .roles
        .get_mut("project_instance")
        .expect("project instance declaration")
        .package = "renamed_project_instance".to_string();

    let mut limits = config.clone();
    limits
        .component_specs
        .get_mut("projects")
        .expect("projects")
        .limits
        .maximum_registry_bytes += 1;

    let mut binding = config.clone();
    binding
        .component_specs
        .get_mut("projects")
        .expect("projects")
        .binding
        .as_mut()
        .expect("binding")
        .pools
        .get_mut("projects")
        .expect("projects pool")
        .key_name = "another_key".to_string();

    let mut child = config;
    child
        .component_specs
        .get_mut("projects")
        .expect("projects")
        .children
        .get_mut("project_instance")
        .expect("project instance")
        .cycles_funding
        .cooldown_secs += 1;

    for changed in [package, limits, binding, child] {
        let changed_hash = ComponentTopology::compile(&changed)
            .expect("changed topology")
            .get(&component_spec("projects"))
            .expect("projects")
            .spec_hash;
        assert_ne!(changed_hash, baseline);
    }
}

#[test]
fn root_projection_requires_canonical_positive_exact_admissions() {
    let topology = topology();
    let projects = admission(&topology, "projects", 2);
    let users = admission(&topology, "users", 1);

    let projected = topology
        .project_for_admissions(&[projects.clone(), users.clone()])
        .expect("canonical admissions");
    assert_eq!(projected.component_specs.len(), 2);

    std::assert_matches!(
        topology.project_for_admissions(&[users.clone(), projects.clone()]),
        Err(ComponentTopologyError::NonCanonicalAdmissionOrder { .. })
    );

    let mut zero = projects.clone();
    zero.maximum_root_instances = 0;
    std::assert_matches!(
        topology.project_for_admissions(&[zero]),
        Err(ComponentTopologyError::ZeroRootAdmission { .. })
    );

    let mut wrong_hash = projects.clone();
    wrong_hash.spec_hash[0] ^= 0xff;
    std::assert_matches!(
        topology.project_for_admissions(&[wrong_hash]),
        Err(ComponentTopologyError::AdmissionSpecHashMismatch { .. })
    );

    let mut excessive = users;
    excessive.maximum_root_instances = 3;
    std::assert_matches!(
        topology.project_for_admissions(&[excessive]),
        Err(ComponentTopologyError::RootAdmissionExceedsFleetMaximum { .. })
    );
}

#[test]
fn root_topology_digest_is_independent_of_separate_capacity_admissions() {
    let topology = topology();
    let one = topology
        .project_for_admissions(&[admission(&topology, "projects", 1)])
        .expect("one-instance admission")
        .digest()
        .expect("one-instance topology digest");
    let two = topology
        .project_for_admissions(&[admission(&topology, "projects", 2)])
        .expect("two-instance admission")
        .digest()
        .expect("two-instance topology digest");

    assert_eq!(one, two);
}

#[test]
fn fleet_admissions_require_coverage_without_exceeding_spec_maxima() {
    let topology = topology();
    let first = vec![
        admission(&topology, "projects", 2),
        admission(&topology, "users", 1),
    ];
    let second = vec![
        admission(&topology, "projects", 1),
        admission(&topology, "users", 1),
    ];

    topology
        .validate_fleet_admissions(&[&first, &second])
        .expect("complete admissions within maxima");

    std::assert_matches!(topology.validate_fleet_admissions(&[&first]), Ok(()));

    let missing_users = vec![admission(&topology, "projects", 1)];
    std::assert_matches!(
        topology.validate_fleet_admissions(&[&missing_users]),
        Err(ComponentTopologyError::MissingFleetAdmission { component_spec })
            if component_spec.as_str() == "users"
    );

    let excessive_projects = vec![admission(&topology, "projects", 2)];
    std::assert_matches!(
        topology.validate_fleet_admissions(&[&first, &excessive_projects]),
        Err(ComponentTopologyError::FleetAdmissionsExceedMaximum { component_spec, .. })
            if component_spec.as_str() == "projects"
    );
}

#[test]
fn canonical_encoding_rejects_oversized_role_package_identity() {
    let mut config = Config::parse_toml(CONFIG).expect("valid Component config");
    config
        .roles
        .get_mut("project_hub")
        .expect("project hub declaration")
        .package = "x".repeat(MAX_COMPONENT_TOPOLOGY_CANONICAL_BYTES);

    std::assert_matches!(
        ComponentTopology::compile(&config),
        Err(ComponentTopologyError::CanonicalBytesExceeded {
            subject: "Component Spec",
            ..
        })
    );
}

#[test]
fn compiled_topology_roundtrips_at_the_candid_boundary() {
    let topology = topology();
    let bytes = candid::encode_one(&topology).expect("encode Component Topology");
    let decoded: ComponentTopology = candid::decode_one(&bytes).expect("decode Component Topology");

    assert_eq!(decoded, topology);
}

#[test]
fn protected_bindings_validate_exact_root_component_and_child_ownership() {
    let topology = topology();
    let root = root_binding(&topology, 4, 5, vec![admission(&topology, "projects", 2)]);
    let component = component_binding(&root);
    let child = ComponentChildBinding {
        component: component.clone(),
        role: CanisterRole::from("project_instance"),
        canister_id: Principal::from_slice(&[11; 29]),
    };

    topology
        .validate_root_binding(&root)
        .expect("exact root binding");
    topology
        .validate_component_binding(&root, &component)
        .expect("exact Component binding");
    topology
        .validate_component_child_binding(&root, &child)
        .expect("exact child binding");

    let mut wrong_role = child.clone();
    wrong_role.role = CanisterRole::from("user_shard");
    std::assert_matches!(
        topology.validate_component_child_binding(&root, &wrong_role),
        Err(ComponentTopologyError::ChildRoleNotAdmitted { .. })
    );

    let mut wrong_root = component;
    wrong_root.fleet_subnet_root = Principal::from_slice(&[12; 29]);
    std::assert_matches!(
        topology.validate_component_binding(&root, &wrong_root),
        Err(ComponentTopologyError::BindingRootMismatch)
    );
}

#[test]
fn fleet_root_bindings_enforce_one_root_per_fleet_subnet_and_admission_sums() {
    let topology = topology();
    let first = root_binding(
        &topology,
        4,
        5,
        vec![
            admission(&topology, "projects", 2),
            admission(&topology, "users", 1),
        ],
    );
    let second = root_binding(
        &topology,
        6,
        7,
        vec![
            admission(&topology, "projects", 1),
            admission(&topology, "users", 1),
        ],
    );

    topology
        .validate_fleet_root_bindings(&[first.clone(), second.clone()])
        .expect("distinct roots and Subnets");

    let mut duplicate_subnet = second.clone();
    duplicate_subnet.placement_subnet = first.placement_subnet;
    std::assert_matches!(
        topology.validate_fleet_root_bindings(&[first.clone(), duplicate_subnet]),
        Err(ComponentTopologyError::DuplicateFleetSubnetRootSubnet { .. })
    );

    let mut duplicate_root = second;
    duplicate_root.fleet_subnet_root = first.fleet_subnet_root;
    std::assert_matches!(
        topology.validate_fleet_root_bindings(&[first, duplicate_root]),
        Err(ComponentTopologyError::DuplicateFleetSubnetRootPrincipal { .. })
    );
}

#[test]
fn protected_binding_contracts_roundtrip_at_the_candid_boundary() {
    let topology = topology();
    let root = root_binding(&topology, 4, 5, vec![admission(&topology, "projects", 2)]);
    let child = ComponentChildBinding {
        component: component_binding(&root),
        role: CanisterRole::from("project_instance"),
        canister_id: Principal::from_slice(&[11; 29]),
    };

    let root_bytes = candid::encode_one(&root).expect("encode root binding");
    let child_bytes = candid::encode_one(&child).expect("encode child binding");
    let decoded_root: FleetSubnetRootBinding =
        candid::decode_one(&root_bytes).expect("decode root binding");
    let decoded_child: ComponentChildBinding =
        candid::decode_one(&child_bytes).expect("decode child binding");

    assert_eq!(decoded_root, root);
    assert_eq!(decoded_child, child);
}
