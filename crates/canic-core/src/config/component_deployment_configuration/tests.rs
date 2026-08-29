//! Focused proofs for semantic Component deployment configuration identity.

use super::*;
use crate::{
    config::{
        ComponentDeploymentPurpose, Config, ConfigError, RoleRuntimeAuthority, schema::CanisterKind,
    },
    dto::component_deployment::ProtectedComponentDeployment,
    ids::{
        AppId, CanisterRole, CanonicalNetworkId, ComponentBinding, ComponentGroupDeploymentId,
        ComponentGroupMemberId, ComponentGroupMemberPath, ComponentGroupPlacementId,
        ComponentGroupSpecId, ComponentInstanceId, ComponentSpecId, FleetBinding,
        FleetCoordinatorBinding, FleetId, FleetKey, FleetRegistryAuthority, SubnetId,
    },
};
use candid::Principal;

const BASELINE: &str = r#"
[app]
name = "configuration_digest"

[roles.root]
kind = "root"
package = "root"

[roles.database]
kind = "canister"
package = "database"

[roles.child]
kind = "canister"
package = "child"

[roles.api]
kind = "canister"
package = "api"

[component_specs.database]
component_role = "database"
maximum_instances = 4

[component_specs.database.children.child]
kind = "instance"

[component_specs.database.spawn_grants.database.child]
maximum_instances_per_parent = 10

[component_specs.api]
component_role = "api"
maximum_instances = 2

[component_groups.databases.components.database]
component_spec = "database"
service = "database"
labels = { tier = "storage" }

[component_groups.apis.components.api]
component_spec = "api"
service = "api"

[component_group_deployments.authoritative]
component_group = "databases"
service_purpose = "authority"
initial_placements = 1
maximum_placements = 1
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1

[component_group_deployments.replicas]
component_group = "databases"
service_purpose = "replica"
labels = { zone = "west" }
initial_placements = 1
maximum_placements = 1
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1

[[component_group_deployments.replicas.member_limits]]
member = ["database"]
spawn_grants = [
  { parent_role = "database", child_role = "child", maximum_instances_per_parent = 5 },
]

[component_group_deployments.pool]
component_group = "apis"
service_purpose = "pool_member"
initial_placements = 1
maximum_placements = 2
placement.maximum_per_root = 2
placement.minimum_distinct_roots = 1

[services.fleet.targets.database]
role = "database"
component_spec = "database"
mode = "authority_replica"
authority_deployment = "authoritative"
authority_member = ["database"]
placement.maximum_members_per_root = 1
placement.minimum_distinct_roots = 1

[services.fleet.targets.api]
role = "api"
component_spec = "api"
mode = "active_pool"
placement.maximum_members_per_root = 2
placement.minimum_distinct_roots = 1
"#;

fn digest(source: &str) -> Result<ComponentDeploymentConfigurationDigest, ConfigError> {
    let config = Config::parse_toml(source)?;
    Ok(config.compile_component_deployment_configuration_digest()?)
}

#[test]
fn semantic_digest_is_stable_across_source_order_and_formatting() {
    let left = r#"
[app]
name = "order_independent"
[roles.root]
kind = "root"
package = "root"
[roles.a]
kind = "canister"
package = "a"
[roles.b]
kind = "canister"
package = "b"
[component_specs.a]
component_role = "a"
maximum_instances = 2
[component_specs.b]
component_role = "b"
maximum_instances = 2
[component_groups.cell.components.a]
component_spec = "a"
[component_groups.cell.components.b]
component_spec = "b"
[component_group_deployments.zeta]
component_group = "cell"
initial_placements = 0
maximum_placements = 1
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1
[component_group_deployments.alpha]
component_group = "cell"
initial_placements = 0
maximum_placements = 1
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1
"#;
    let right = r#"
# Comments and source order are not protected semantics.
[app]
name = "order_independent"

[roles.b]
package = "b"
kind = "canister"
[roles.a]
package = "a"
kind = "canister"
[roles.root]
package = "root"
kind = "root"

[component_specs.b]
maximum_instances = 2
component_role = "b"
[component_specs.a]
maximum_instances = 2
component_role = "a"

[component_groups.cell.components.b]
component_spec = "b"
[component_groups.cell.components.a]
component_spec = "a"

[component_group_deployments.alpha]
component_group = "cell"
maximum_placements = 1
initial_placements = 0
placement.minimum_distinct_roots = 1
placement.maximum_per_root = 1
[component_group_deployments.zeta]
component_group = "cell"
maximum_placements = 1
initial_placements = 0
placement.minimum_distinct_roots = 1
placement.maximum_per_root = 1
"#;

    assert_eq!(
        digest(left).expect("left digest"),
        digest(right).expect("right digest")
    );
}

#[test]
fn semantic_digest_changes_for_every_protected_projection() {
    let baseline = digest(BASELINE).expect("baseline digest");
    let mut swapped_purpose = BASELINE
        .replace(
            "service_purpose = \"authority\"",
            "service_purpose = \"pending\"",
        )
        .replace(
            "service_purpose = \"replica\"",
            "service_purpose = \"authority\"",
        )
        .replace(
            "service_purpose = \"pending\"",
            "service_purpose = \"replica\"",
        );
    swapped_purpose = swapped_purpose.replace(
        "authority_deployment = \"authoritative\"",
        "authority_deployment = \"replicas\"",
    );
    let changed = [
        BASELINE.replace("package = \"database\"", "package = \"database_v2\""),
        BASELINE
            .replace("components.database]", "components.primary_database]")
            .replace("member = [\"database\"]", "member = [\"primary_database\"]")
            .replace(
                "authority_member = [\"database\"]",
                "authority_member = [\"primary_database\"]",
            ),
        swapped_purpose,
        BASELINE.replace("zone = \"west\"", "zone = \"east\""),
        BASELINE.replace("maximum_instances_per_parent = 5", "maximum_instances_per_parent = 4"),
        BASELINE.replace(
            "component_group = \"apis\"\nservice_purpose = \"pool_member\"\ninitial_placements = 1",
            "component_group = \"apis\"\nservice_purpose = \"pool_member\"\ninitial_placements = 2",
        ),
        BASELINE.replace(
            "component_group = \"apis\"\nservice_purpose = \"pool_member\"\ninitial_placements = 1\nmaximum_placements = 2\nplacement.maximum_per_root = 2",
            "component_group = \"apis\"\nservice_purpose = \"pool_member\"\ninitial_placements = 1\nmaximum_placements = 2\nplacement.maximum_per_root = 1",
        ),
        BASELINE.replace(
            "mode = \"active_pool\"\nplacement.maximum_members_per_root = 2",
            "mode = \"active_pool\"\nplacement.maximum_members_per_root = 1",
        ),
    ];

    for source in changed {
        assert_ne!(digest(&source).expect("valid changed digest"), baseline);
    }
}

#[test]
fn equal_to_spec_reduction_has_one_semantic_digest() {
    let explicit = BASELINE.replace(
        "maximum_instances_per_parent = 5",
        "maximum_instances_per_parent = 10",
    );
    let omitted = explicit.replace(
        r#"
[[component_group_deployments.replicas.member_limits]]
member = ["database"]
spawn_grants = [
  { parent_role = "database", child_role = "child", maximum_instances_per_parent = 10 },
]
"#,
        "",
    );

    assert_eq!(
        digest(&explicit).expect("explicit equal-to-Spec digest"),
        digest(&omitted).expect("omitted equal-to-Spec digest")
    );
}

#[test]
fn canonical_configuration_digest_matches_schema_one_golden_vector() {
    assert_eq!(
        digest(BASELINE).expect("golden digest").to_string(),
        "36a0228d13ffd4dd304881c8a9e924f6fefd18ffdae6afc163aa60e293b4b18c"
    );
}

#[test]
fn compiled_configuration_round_trips_with_the_exact_semantic_digest() {
    let config = Config::parse_toml(BASELINE).expect("valid deployment configuration");
    let compiled = config
        .compile_component_deployment_configuration()
        .expect("compiled deployment configuration");
    let expected = config
        .compile_component_deployment_configuration_digest()
        .expect("configuration digest");

    assert_eq!(compiled.digest().expect("compiled digest"), expected);
    let encoded = candid::encode_one(&compiled).expect("encode compiled configuration");
    let decoded: ComponentDeploymentConfiguration =
        candid::decode_one(&encoded).expect("decode compiled configuration");
    assert_eq!(decoded, compiled);
    assert_eq!(decoded.digest().expect("decoded digest"), expected);
}

#[test]
fn decoded_compiled_configuration_rejects_noncanonical_topology() {
    let config = Config::parse_toml(BASELINE).expect("valid deployment configuration");
    let mut compiled = config
        .compile_component_deployment_configuration()
        .expect("compiled deployment configuration");
    compiled.component_group_topology.component_groups.reverse();

    assert!(matches!(
        compiled.digest(),
        Err(
            ComponentDeploymentConfigurationDigestError::ComponentGroupTopology(
                ComponentGroupTopologyError::NonCanonicalGroupOrder { .. }
            )
        )
    ));
}

#[test]
fn group_member_context_matches_only_the_exact_compiled_projection() {
    let config = Config::parse_toml(BASELINE).expect("valid deployment configuration");
    let deployment_topology = config
        .compile_component_group_deployment_topology()
        .expect("deployment topology");
    let deployment = deployment_topology
        .get(&"replicas".parse().expect("deployment ID"))
        .expect("replica deployment");
    let member = deployment.members.first().expect("database member");
    let binding = component_binding(member.component_spec.clone(), member.component_spec_hash);
    let context = ProtectedComponentDeployment::GroupMember {
        binding: binding.clone(),
        configuration_digest: config
            .compile_component_deployment_configuration_digest()
            .expect("configuration digest"),
        group_placement: ComponentGroupPlacementId {
            deployment: deployment.deployment.clone(),
            ordinal: 7,
        },
        component_group: deployment.component_group.clone(),
        member_path: member.member_path.clone(),
        purpose: member.purpose.clone(),
        labels: member.labels.clone(),
        limits: member.limits.clone(),
    };

    config
        .validate_protected_component_deployment(&context, &binding)
        .expect("exact protected context");
    let runtime = assert_compiled_role_runtime_projection(&config, &binding);
    assert_runtime_projection_rejects_substitutions(&runtime, &context, &binding);
    let encoded = candid::encode_one(&context).expect("encode protected context");
    let decoded: ProtectedComponentDeployment =
        candid::decode_one(&encoded).expect("decode protected context");
    assert_eq!(decoded, context);

    let mut wrong_digest = context.clone();
    let ProtectedComponentDeployment::GroupMember {
        configuration_digest,
        ..
    } = &mut wrong_digest
    else {
        unreachable!()
    };
    *configuration_digest = ComponentDeploymentConfigurationDigest::from_bytes([99; 32]);
    assert!(matches!(
        config.validate_protected_component_deployment(&wrong_digest, &binding),
        Err(ProtectedComponentDeploymentError::ConfigurationDigestMismatch)
    ));

    let mut wrong_purpose = context.clone();
    let ProtectedComponentDeployment::GroupMember { purpose, .. } = &mut wrong_purpose else {
        unreachable!()
    };
    *purpose = ComponentDeploymentPurpose::Ordinary;
    assert!(matches!(
        config.validate_protected_component_deployment(&wrong_purpose, &binding),
        Err(ProtectedComponentDeploymentError::PurposeMismatch { .. })
    ));

    let mut wrong_limits = context.clone();
    let ProtectedComponentDeployment::GroupMember { limits, .. } = &mut wrong_limits else {
        unreachable!()
    };
    limits.maximum_descendants -= 1;
    assert!(matches!(
        config.validate_protected_component_deployment(&wrong_limits, &binding),
        Err(ProtectedComponentDeploymentError::LimitsMismatch { .. })
    ));

    let mut wrong_member = context;
    let ProtectedComponentDeployment::GroupMember { member_path, .. } = &mut wrong_member else {
        unreachable!()
    };
    *member_path = ComponentGroupMemberPath::try_from(vec![
        "missing"
            .parse::<ComponentGroupMemberId>()
            .expect("member ID"),
    ])
    .expect("member path");
    assert!(matches!(
        config.validate_protected_component_deployment(&wrong_member, &binding),
        Err(ProtectedComponentDeploymentError::UnknownMember { .. })
    ));
}

fn assert_compiled_role_runtime_projection(
    config: &ConfigModel,
    binding: &ComponentBinding,
) -> RoleRuntimeAuthority {
    let runtime = RoleRuntimeAuthority::compile(config, &CanisterRole::from("database"))
        .expect("compiled runtime authority");
    assert_eq!(runtime.component_topology.component_specs.len(), 2);
    for component_spec in ["database", "api"] {
        assert!(
            runtime
                .component_topology
                .get(&component_spec.parse().expect("Component Spec ID"))
                .is_some()
        );
    }
    assert!(
        runtime
            .canister(
                Some(&binding.component_spec),
                &CanisterRole::from("database")
            )
            .is_some()
    );
    assert!(
        runtime
            .canister(None, &CanisterRole::from("database"))
            .is_none()
    );
    let child = runtime
        .child(&binding.component_spec, &CanisterRole::from("child"))
        .expect("admitted child authority");
    assert_eq!(child.kind, CanisterKind::Instance);
    assert_eq!(
        child.cycles_funding.max_per_request.to_u128(),
        5_000_000_000_000
    );
    assert!(
        runtime
            .canister(Some(&binding.component_spec), &CanisterRole::from("api"))
            .is_none()
    );
    assert!(
        runtime
            .deployment_members
            .iter()
            .all(|authority| authority.member.component_spec == binding.component_spec)
    );

    let root_runtime = RoleRuntimeAuthority::compile(config, &CanisterRole::ROOT)
        .expect("compiled Root runtime authority");
    assert!(root_runtime.component_topology.component_specs.is_empty());
    root_runtime
        .component_topology
        .canonical_bytes()
        .expect("empty Root runtime topology remains canonical");
    assert!(root_runtime.canister(None, &CanisterRole::ROOT).is_some());
    assert!(root_runtime.children.is_empty());
    for role in std::iter::once(CanisterRole::WASM_STORE)
        .chain(["database", "api", "child"].map(CanisterRole::from))
    {
        assert!(root_runtime.canister(None, &role).is_none());
    }

    runtime
}

fn assert_runtime_projection_rejects_substitutions(
    runtime: &RoleRuntimeAuthority,
    context: &ProtectedComponentDeployment,
    binding: &ComponentBinding,
) {
    runtime
        .validate_protected_component_deployment(context, binding)
        .expect("runtime authority accepts the exact protected context");

    let mut wrong_digest = context.clone();
    let ProtectedComponentDeployment::GroupMember {
        configuration_digest,
        ..
    } = &mut wrong_digest
    else {
        unreachable!()
    };
    *configuration_digest = ComponentDeploymentConfigurationDigest::from_bytes([99; 32]);

    let mut wrong_purpose = context.clone();
    let ProtectedComponentDeployment::GroupMember { purpose, .. } = &mut wrong_purpose else {
        unreachable!()
    };
    *purpose = ComponentDeploymentPurpose::Ordinary;

    let mut wrong_limits = context.clone();
    let ProtectedComponentDeployment::GroupMember { limits, .. } = &mut wrong_limits else {
        unreachable!()
    };
    limits.maximum_descendants -= 1;

    let mut wrong_member = context.clone();
    let ProtectedComponentDeployment::GroupMember { member_path, .. } = &mut wrong_member else {
        unreachable!()
    };
    *member_path = ComponentGroupMemberPath::try_from(vec![
        "missing"
            .parse::<ComponentGroupMemberId>()
            .expect("member ID"),
    ])
    .expect("member path");

    for changed in [wrong_digest, wrong_purpose, wrong_limits, wrong_member] {
        assert!(
            runtime
                .validate_protected_component_deployment(&changed, binding)
                .is_err()
        );
    }
}

#[test]
fn deployment_context_rejects_binding_or_plan_identity_substitution() {
    let config = Config::parse_toml(BASELINE).expect("valid deployment configuration");
    let binding = component_binding("database".parse().expect("Component Spec"), [10; 32]);
    let ordinary = ProtectedComponentDeployment::UngroupedOrdinary {
        binding: binding.clone(),
    };
    config
        .validate_protected_component_deployment(&ordinary, &binding)
        .expect("exact ungrouped binding");

    let mut other_binding = binding;
    other_binding.component = ComponentInstanceId::from_generated_bytes([22; 32]);
    assert!(matches!(
        config.validate_protected_component_deployment(&ordinary, &other_binding),
        Err(ProtectedComponentDeploymentError::BindingMismatch)
    ));

    let deployment_topology = config
        .compile_component_group_deployment_topology()
        .expect("deployment topology");
    let deployment = deployment_topology
        .get(&"replicas".parse().expect("deployment ID"))
        .expect("replica deployment");
    let member = deployment.members.first().expect("database member");
    let exact_binding =
        component_binding(member.component_spec.clone(), member.component_spec_hash);
    let mut context = ProtectedComponentDeployment::GroupMember {
        binding: exact_binding.clone(),
        configuration_digest: config
            .compile_component_deployment_configuration_digest()
            .expect("configuration digest"),
        group_placement: ComponentGroupPlacementId {
            deployment: ComponentGroupDeploymentId::try_from("missing".to_string())
                .expect("deployment ID"),
            ordinal: 1,
        },
        component_group: ComponentGroupSpecId::try_from("databases".to_string())
            .expect("Component Group ID"),
        member_path: member.member_path.clone(),
        purpose: member.purpose.clone(),
        labels: member.labels.clone(),
        limits: member.limits.clone(),
    };
    assert!(matches!(
        config.validate_protected_component_deployment(&context, &exact_binding),
        Err(ProtectedComponentDeploymentError::UnknownDeployment { .. })
    ));

    let ProtectedComponentDeployment::GroupMember {
        group_placement,
        component_group,
        ..
    } = &mut context
    else {
        unreachable!()
    };
    group_placement.deployment = deployment.deployment.clone();
    *component_group =
        ComponentGroupSpecId::try_from("apis".to_string()).expect("Component Group ID");
    assert!(matches!(
        config.validate_protected_component_deployment(&context, &exact_binding),
        Err(ProtectedComponentDeploymentError::ComponentGroupMismatch { .. })
    ));
}

fn component_binding(component_spec: ComponentSpecId, spec_hash: [u8; 32]) -> ComponentBinding {
    let coordinator_subnet = SubnetId::from_principal(Principal::from_slice(&[2; 29]));
    let root_subnet = SubnetId::from_principal(Principal::from_slice(&[3; 29]));
    ComponentBinding {
        authority: FleetRegistryAuthority {
            binding: FleetCoordinatorBinding {
                fleet: FleetBinding {
                    fleet: FleetKey {
                        canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                        fleet_id: FleetId::from_generated_bytes([1; 32]),
                    },
                    app: AppId::from("configuration_digest"),
                },
                coordinator_subnet,
                coordinator: Principal::from_slice(&[4; 29]),
            },
            epoch: 1,
        },
        component: ComponentInstanceId::from_generated_bytes([5; 32]),
        component_spec,
        spec_hash,
        role: "database".into(),
        placement_subnet: root_subnet,
        fleet_subnet_root: Principal::from_slice(&[6; 29]),
        canister_id: Principal::from_slice(&[7; 29]),
    }
}
