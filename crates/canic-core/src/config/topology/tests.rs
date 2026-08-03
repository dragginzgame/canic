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

[roles.project_ledger]
kind = "canister"
package = "project_ledger"

[roles.project_machine]
kind = "canister"
package = "project_machine"

[component_specs.users]
component_role = "user_hub"
maximum_instances = 2

[component_specs.users.children.user_shard]
kind = "shard"

[component_specs.users.spawn_grants.user_hub.user_shard]
maximum_instances_per_parent = 8

[component_specs.users.sharding.pools.user_shards]
canister_role = "user_shard"
policy.capacity = 100
policy.initial_shards = 1
policy.max_shards = 8

[component_specs.projects]
component_role = "project_hub"
maximum_instances = 3

[component_specs.projects.provisions.users]
maximum_instances_per_requester_per_root = 2

[component_specs.projects.limits]
maximum_descendants = 300
maximum_registry_bytes = 1048576

[component_specs.projects.limits.cycles_funding]
window_secs = 600
maximum_cycles = "250T"

[component_specs.projects.children.project_instance]
kind = "instance"

[component_specs.projects.children.project_ledger]
kind = "singleton"

[component_specs.projects.children.project_machine]
kind = "singleton"

[component_specs.projects.spawn_grants.project_hub.project_instance]
maximum_instances_per_parent = 100

[component_specs.projects.spawn_grants.project_instance.project_ledger]
maximum_instances_per_parent = 1

[component_specs.projects.spawn_grants.project_instance.project_machine]
maximum_instances_per_parent = 1

[component_specs.projects.index.pools.projects]
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
                    canonical_network_id: CanonicalNetworkId::ic_mainnet(),
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
            canister_pool: crate::ids::FleetSubnetCanisterPoolConfig {
                minimum_size: 1,
                maximum_size: 10,
                canister_cycles: Cycles::new(5_000_000_000_000),
            },
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
fn topology_compiles_specs_and_potential_child_roles_in_canonical_order() {
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
        vec!["project_instance", "project_ledger", "project_machine"],
    );
    assert_eq!(
        topology.component_specs[0].spawn_grants,
        vec![
            ComponentSpawnGrant {
                parent_role: CanisterRole::from("project_hub"),
                child_role: CanisterRole::from("project_instance"),
                maximum_instances_per_parent: 100,
            },
            ComponentSpawnGrant {
                parent_role: CanisterRole::from("project_instance"),
                child_role: CanisterRole::from("project_ledger"),
                maximum_instances_per_parent: 1,
            },
            ComponentSpawnGrant {
                parent_role: CanisterRole::from("project_instance"),
                child_role: CanisterRole::from("project_machine"),
                maximum_instances_per_parent: 1,
            },
        ],
    );
    assert!(
        topology.component_specs[0]
            .spawn_grant(
                &CanisterRole::from("project_instance"),
                &CanisterRole::from("project_ledger"),
            )
            .is_some(),
    );
    assert_eq!(
        topology.provisioning_grants,
        vec![ComponentProvisioningGrant {
            requester_component_spec: component_spec("projects"),
            target_component_spec: component_spec("users"),
            maximum_instances_per_requester_per_root: 2,
        }]
    );
    assert_eq!(topology.component_specs[0].limits.maximum_descendants, 300);
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
        "c7c76c00e156c56f76d97601ace207a854b501d80e8acfc849c8e6cca5dd11b0",
    );
    assert_eq!(
        hex_bytes(topology.component_specs[1].spec_hash),
        "938046a0febe19bb17380024dff3f5458d84fab5f22829f1c1c381c5c7fb07b8",
    );
    assert_eq!(
        topology.digest().expect("topology digest").to_string(),
        "436945fa61c71c65b079ddcd633e973ffa1114850564ebd0f28c9bfc49b20e08",
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

    let mut index = config.clone();
    index
        .component_specs
        .get_mut("projects")
        .expect("projects")
        .index
        .as_mut()
        .expect("index")
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

    let mut spawn_grant = child.clone();
    spawn_grant
        .component_specs
        .get_mut("projects")
        .expect("projects")
        .spawn_grants
        .get_mut("project_hub")
        .expect("project hub grants")
        .get_mut("project_instance")
        .expect("project instance grant")
        .maximum_instances_per_parent += 1;

    for changed in [package, limits, index, child, spawn_grant] {
        let changed_hash = ComponentTopology::compile(&changed)
            .expect("changed topology")
            .get(&component_spec("projects"))
            .expect("projects")
            .spec_hash;
        assert_ne!(changed_hash, baseline);
    }
}

#[test]
fn provisioning_grants_are_topology_authority_not_spec_content() {
    let config = Config::parse_toml(CONFIG).expect("valid Component config");
    let baseline = ComponentTopology::compile(&config).expect("baseline topology");
    let baseline_projects_hash = baseline
        .get(&component_spec("projects"))
        .expect("projects")
        .spec_hash;

    let mut changed = config;
    changed
        .component_specs
        .get_mut("projects")
        .expect("projects")
        .provisions
        .get_mut("users")
        .expect("projects to users grant")
        .maximum_instances_per_requester_per_root += 1;
    let changed = ComponentTopology::compile(&changed).expect("changed topology");

    assert_eq!(
        changed
            .get(&component_spec("projects"))
            .expect("projects")
            .spec_hash,
        baseline_projects_hash,
    );
    assert_ne!(
        changed.digest().expect("changed topology digest"),
        baseline.digest().expect("baseline topology digest"),
    );
}

#[test]
fn provisioning_grants_reject_unknown_self_zero_and_cyclic_edges() {
    use crate::config::schema::ComponentProvisioningGrantConfig;

    let config = Config::parse_toml(CONFIG).expect("valid Component config");
    let grant = |maximum_instances_per_requester_per_root| ComponentProvisioningGrantConfig {
        maximum_instances_per_requester_per_root,
    };

    let mut unknown = config.clone();
    unknown
        .component_specs
        .get_mut("projects")
        .expect("projects")
        .provisions
        .insert(component_spec("missing"), grant(1));
    std::assert_matches!(
        ComponentTopology::compile(&unknown),
        Err(ComponentTopologyError::UnknownProvisioningGrantTarget { .. })
    );

    let mut self_target = config.clone();
    self_target
        .component_specs
        .get_mut("projects")
        .expect("projects")
        .provisions
        .insert(component_spec("projects"), grant(1));
    std::assert_matches!(
        ComponentTopology::compile(&self_target),
        Err(ComponentTopologyError::SelfProvisioningGrant { .. })
    );

    let mut zero = config.clone();
    zero.component_specs
        .get_mut("projects")
        .expect("projects")
        .provisions
        .get_mut("users")
        .expect("projects to users grant")
        .maximum_instances_per_requester_per_root = 0;
    std::assert_matches!(
        ComponentTopology::compile(&zero),
        Err(ComponentTopologyError::ZeroProvisioningGrantLimit { .. })
    );

    let mut cyclic = config;
    cyclic
        .component_specs
        .get_mut("users")
        .expect("users")
        .provisions
        .insert(component_spec("projects"), grant(1));
    std::assert_matches!(
        ComponentTopology::compile(&cyclic),
        Err(ComponentTopologyError::CyclicProvisioningGrant { .. })
    );
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
    assert_eq!(projected.provisioning_grants.len(), 1);

    let users_only = topology
        .project_for_admissions(std::slice::from_ref(&users))
        .expect("target-only root projection");
    assert_eq!(users_only.provisioning_grants.len(), 1);
    assert_eq!(
        users_only.provisioning_grants[0].requester_component_spec,
        component_spec("projects"),
    );
    assert!(
        users_only.get(&component_spec("projects")).is_none(),
        "an incoming grant must not admit its requester Spec"
    );

    let projects_only = topology
        .project_for_admissions(std::slice::from_ref(&projects))
        .expect("requester-only root projection");
    assert!(projects_only.provisioning_grants.is_empty());

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

    let mut wrong_hash = projects;
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
fn canonical_encoding_rejects_malformed_compiled_order_and_spawn_grants() {
    let topology = topology();

    let mut spec_order = topology.clone();
    spec_order.component_specs.reverse();
    std::assert_matches!(
        spec_order.canonical_bytes(),
        Err(ComponentTopologyError::NonCanonicalComponentSpecOrder { .. })
    );

    let mut grant_order = topology.clone();
    grant_order
        .provisioning_grants
        .push(grant_order.provisioning_grants[0].clone());
    std::assert_matches!(
        grant_order.canonical_bytes(),
        Err(ComponentTopologyError::NonCanonicalProvisioningGrantOrder { .. })
    );

    let mut zero_spawn_limit = topology;
    zero_spawn_limit.component_specs[1].spawn_grants[0].maximum_instances_per_parent = 0;
    std::assert_matches!(
        zero_spawn_limit.canonical_bytes(),
        Err(ComponentTopologyError::ZeroSpawnGrantLimit { .. })
    );
}

#[test]
fn protected_bindings_validate_exact_root_component_and_multilevel_child_shape() {
    let topology = topology();
    let root = root_binding(&topology, 4, 5, vec![admission(&topology, "projects", 2)]);
    let component = component_binding(&root);
    let child = ComponentChildBinding {
        component: component.clone(),
        parent_canister_id: component.canister_id,
        role: CanisterRole::from("project_instance"),
        canister_id: Principal::from_slice(&[11; 29]),
    };
    let grandchild = ComponentChildBinding {
        component: component.clone(),
        parent_canister_id: child.canister_id,
        role: CanisterRole::from("project_ledger"),
        canister_id: Principal::from_slice(&[12; 29]),
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
    topology
        .validate_component_child_binding(&root, &grandchild)
        .expect("a child may own another child in the same Component tree");

    let mut wrong_role = child.clone();
    wrong_role.role = CanisterRole::from("user_shard");
    std::assert_matches!(
        topology.validate_component_child_binding(&root, &wrong_role),
        Err(ComponentTopologyError::ChildRoleNotAdmitted { .. })
    );

    let mut self_parent = child.clone();
    self_parent.parent_canister_id = self_parent.canister_id;
    std::assert_matches!(
        topology.validate_component_child_binding(&root, &self_parent),
        Err(ComponentTopologyError::ChildPrincipalConflictsWithOwner)
    );

    let mut root_parent = child;
    root_parent.parent_canister_id = root.fleet_subnet_root;
    std::assert_matches!(
        topology.validate_component_child_binding(&root, &root_parent),
        Err(ComponentTopologyError::ChildParentConflictsWithAuthority)
    );

    let mut wrong_root = component;
    wrong_root.fleet_subnet_root = Principal::from_slice(&[13; 29]);
    std::assert_matches!(
        topology.validate_component_binding(&root, &wrong_root),
        Err(ComponentTopologyError::BindingRootMismatch)
    );
}

#[test]
fn fleet_subnet_root_bindings_enforce_one_root_per_fleet_subnet_and_admission_sums() {
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
        .validate_fleet_subnet_root_bindings(&[first.clone(), second.clone()])
        .expect("distinct roots and Subnets");

    let mut duplicate_subnet = second.clone();
    duplicate_subnet.placement_subnet = first.placement_subnet;
    std::assert_matches!(
        topology.validate_fleet_subnet_root_bindings(&[first.clone(), duplicate_subnet]),
        Err(ComponentTopologyError::DuplicateFleetSubnetRootSubnet { .. })
    );

    let mut duplicate_root = second;
    duplicate_root.fleet_subnet_root = first.fleet_subnet_root;
    std::assert_matches!(
        topology.validate_fleet_subnet_root_bindings(&[first, duplicate_root]),
        Err(ComponentTopologyError::DuplicateFleetSubnetRootPrincipal { .. })
    );
}

#[test]
fn protected_binding_contracts_roundtrip_at_the_candid_boundary() {
    let topology = topology();
    let root = root_binding(&topology, 4, 5, vec![admission(&topology, "projects", 2)]);
    let child = ComponentChildBinding {
        component: component_binding(&root),
        parent_canister_id: Principal::from_slice(&[10; 29]),
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
