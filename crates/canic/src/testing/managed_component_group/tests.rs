use super::*;

#[test]
fn application_init_binds_the_actual_canister_without_changing_static_bytes() {
    fn encode(canister: Principal) -> Vec<u8> {
        canic_contracts::protocol::initialization::encode(canister).expect("Principal argument")
    }
    let first = Principal::self_authenticating([1]);
    let second = Principal::self_authenticating([2]);
    let bound = ManagedApplicationInit::ForCanister(encode);
    assert_eq!(bound.for_canister(first), Some(encode(first)));
    assert_eq!(bound.for_canister(second), Some(encode(second)));
    let encoded = ManagedApplicationInit::Encoded(encode(first));
    assert_eq!(encoded.for_canister(second), Some(encode(first)));
    assert_eq!(ManagedApplicationInit::None.for_canister(first), None);
}

const GROUP_CONFIG: &str = r#"
[app]
name = "qualification"

[roles.root]
kind = "root"

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

#[test]
fn app_failure_projection_retains_decoder_limits_and_transport_classification() {
    use super::super::managed_app::ManagedAppQualificationError as App;
    use ic_platform::candid::DecodeError;
    use {
        ic_platform::pocket_ic::{ErrorCode, RejectCode, RejectResponse},
        ic_testkit::pic::PocketIcOperationError,
    };

    let error =
        ManagedComponentGroupQualificationError::from(App::Decode(DecodeError::ReplyTooLarge {
            actual: 42,
            maximum: 8,
        }));
    assert!(matches!(
        error,
        ManagedComponentGroupQualificationError::Decode(DecodeError::ReplyTooLarge {
            actual: 42,
            maximum: 8
        })
    ));
    let response = RejectResponse {
        reject_code: RejectCode::CanisterReject,
        reject_message: "refused".into(),
        error_code: ErrorCode::CanisterRejectedMessage,
        certified: false,
    };
    let error = ManagedComponentGroupQualificationError::from(App::Transport(
        PocketIcCallFailure::Rejected(response.clone()),
    ));
    let ManagedComponentGroupQualificationError::Transport(PocketIcCallFailure::Rejected(actual)) =
        error
    else {
        panic!("rejection must remain distinguishable from transport failure");
    };
    assert_eq!(actual, response);
    let source = PocketIcOperationError::new("fixture transport failure");
    let error = ManagedComponentGroupQualificationError::from(App::Transport(
        PocketIcCallFailure::Operation(source.clone()),
    ));
    let ManagedComponentGroupQualificationError::Transport(PocketIcCallFailure::Operation(actual)) =
        error
    else {
        panic!("operation failure must retain its typed source");
    };
    assert_eq!(actual, source);
}

#[test]
fn allocation_reply_keeps_settlement_and_acknowledgement_slots_distinct() {
    let parent = ic_platform::Principal::self_authenticating([1]);
    let child = ic_platform::Principal::self_authenticating([2]);
    let role = CanisterRole::from("shard_child");
    let spec = child_control_spec("testing_component_child_allocations", 0x5445_5354_0000_0b00);
    for settled in [false, true] {
        for acknowledged in [false, true] {
            // Independent positional producer shape includes both trailing booleans.
            let payload = vec![(
                [9_u8; 32],
                parent,
                role.clone(),
                Some(vec![7_u8]),
                child,
                settled,
                acknowledged,
            )];
            let bytes =
                ic_wire::native::encode_response(spec.method_id, &payload, spec.response_limits)
                    .unwrap();
            let mut decoded: Vec<WireChildAllocation> =
                ic_wire::native::decode_response(spec.method_id, &bytes, spec.response_limits)
                    .unwrap();
            let observation = decoded.pop().unwrap();
            assert!(decoded.is_empty());
            assert_eq!(observation._settled, settled);
            let allocation = FixtureChildAllocation::from(observation);
            assert_eq!(allocation.acknowledged, acknowledged);
            assert_eq!(allocation.request_id, [9; 32]);
            assert_eq!(allocation.parent, parent.into());
            assert_eq!(allocation.child, child.into());
            assert_eq!(allocation.canister_role, role);
            assert_eq!(allocation.extra_arg, Some(vec![7]));
        }
    }
}
