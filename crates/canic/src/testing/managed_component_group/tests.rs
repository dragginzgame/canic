use super::*;

const GROUP_CONFIG: &str = r#"
[app]
name = "qualification"

[roles.root]
kind = "root"
package = "root"

[roles.hub]
kind = "canister"
package = "hub"

[roles.index_child]
kind = "canister"
package = "index_child"
fleet_admission = true

[roles.scale_child]
kind = "canister"
package = "scale_child"
fleet_admission = true

[roles.shard_child]
kind = "canister"
package = "shard_child"
fleet_admission = true

[component_specs.tree]
component_role = "hub"
maximum_instances = 1

[component_specs.tree.children.index_child]
kind = "instance"

[component_specs.tree.children.scale_child]
kind = "replica"

[component_specs.tree.children.shard_child]
kind = "shard"

[component_specs.tree.spawn_grants.hub.index_child]
maximum_instances_per_parent = 4

[component_specs.tree.spawn_grants.hub.scale_child]
maximum_instances_per_parent = 4

[component_specs.tree.spawn_grants.hub.shard_child]
maximum_instances_per_parent = 4

[component_specs.tree.index.pools.indexed]
canister_role = "index_child"
key_name = "item_id"

[component_specs.tree.scaling.pools.scaled]
canister_role = "scale_child"
policy.initial_workers = 1
policy.max_workers = 4
policy.min_workers = 1

[component_specs.tree.sharding.pools.sharded]
canister_role = "shard_child"
policy.capacity = 10
policy.initial_shards = 1
policy.max_shards = 4

[component_groups.tree.components.hub]
component_spec = "tree"

[component_group_deployments.tree]
component_group = "tree"
initial_placements = 1
maximum_placements = 1
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1
"#;

fn artifact(role: &str) -> ManagedRoleQualificationArtifact {
    ManagedRoleQualificationArtifact::new(role.parse().expect("canonical role"), Vec::new())
}

#[test]
fn role_artifact_contract_covers_sharding_scaling_and_index_children() {
    let config = parse_config_model(GROUP_CONFIG).expect("managed tree config");
    let topology = config
        .compile_component_topology()
        .expect("managed tree topology");
    let deployments = config
        .compile_component_group_deployment_topology()
        .expect("managed tree deployments");
    let deployment = deployments
        .get(&"tree".parse().expect("deployment ID"))
        .expect("tree deployment");

    let artifacts = validate_role_artifacts(
        &topology,
        deployment,
        vec![
            artifact("hub"),
            artifact("index_child"),
            artifact("scale_child"),
            artifact("shard_child"),
        ],
    )
    .expect("all placement roles are accepted");

    for role in ["index_child", "scale_child", "shard_child"] {
        let role: CanisterRole = role.parse().expect("canonical role");
        assert!(artifacts.contains_key(&role));
        assert!(
            topology
                .get(&"tree".parse().expect("Component Spec ID"))
                .expect("tree Component Spec")
                .spawn_grant(&CanisterRole::new("hub"), &role)
                .is_some()
        );
    }
}

#[test]
fn role_artifact_contract_rejects_a_missing_placement_child() {
    let config = parse_config_model(GROUP_CONFIG).expect("managed tree config");
    let topology = config
        .compile_component_topology()
        .expect("managed tree topology");
    let deployments = config
        .compile_component_group_deployment_topology()
        .expect("managed tree deployments");
    let deployment = deployments
        .get(&"tree".parse().expect("deployment ID"))
        .expect("tree deployment");

    let error = validate_role_artifacts(
        &topology,
        deployment,
        vec![
            artifact("hub"),
            artifact("scale_child"),
            artifact("shard_child"),
        ],
    )
    .expect_err("index child artifact must be present");

    assert!(matches!(
        error,
        ManagedComponentGroupQualificationError::Config(_)
    ));
}

#[test]
fn child_binding_validation_is_placement_strategy_neutral() {
    let config = parse_config_model(GROUP_CONFIG).expect("managed tree config");
    let topology = config
        .compile_component_topology()
        .expect("managed tree topology");
    let deployments = config
        .compile_component_group_deployment_topology()
        .expect("managed tree deployments");
    let deployment = deployments
        .get(&"tree".parse().expect("deployment ID"))
        .expect("tree deployment");
    let root_principal = Principal::from_slice(&[41; 29]);
    let root = compile_root(&config, &topology, deployment, root_principal, &[42; 32])
        .expect("compile fixture Root");
    let spec = topology
        .get(&"tree".parse().expect("Component Spec ID"))
        .expect("tree Component Spec");
    let component = ComponentBinding {
        authority: root.authority.clone(),
        canister_id: Principal::from_slice(&[43; 29]),
        component: ComponentInstanceId::from_generated_bytes([44; 32]),
        component_spec: spec.component_spec.clone(),
        fleet_subnet_root: root_principal,
        placement_subnet: root.placement_subnet,
        role: spec.component_role.clone(),
        spec_hash: spec.spec_hash,
    };

    for (index, role) in ["index_child", "scale_child", "shard_child"]
        .into_iter()
        .enumerate()
    {
        let binding = ComponentChildBinding {
            component: component.clone(),
            parent_canister_id: component.canister_id,
            role: role.parse().expect("canonical role"),
            canister_id: Principal::from_slice(
                &[45 + u8::try_from(index).expect("small index"); 29],
            ),
        };
        topology
            .validate_component_child_binding(&root, &binding)
            .expect("all placement strategies use the same exact child authority");
    }
}
