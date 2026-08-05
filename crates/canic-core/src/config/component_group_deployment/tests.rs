//! Focused proofs for independent deployment flattening and placement envelopes.

use super::*;
use crate::config::{ComponentDeploymentLabelValue, Config, ConfigError};

const CONFIG_PREFIX: &str = r#"
[app]
name = "deployment_composition"

[roles.root]
kind = "root"
package = "root"

[roles.a]
kind = "canister"
package = "a"

[roles.b]
kind = "canister"
package = "b"

[roles.c]
kind = "canister"
package = "c"

[roles.child]
kind = "canister"
package = "child"

[roles.once]
kind = "canister"
package = "once"

[component_specs.a]
component_role = "a"
maximum_instances = 8

[component_specs.a.children.child]
kind = "instance"

[component_specs.a.children.once]
kind = "singleton"

[component_specs.a.spawn_grants.a.child]
maximum_instances_per_parent = 10

[component_specs.a.spawn_grants.child.once]
maximum_instances_per_parent = 1

[component_specs.a.spawn_grants.child.child]
maximum_instances_per_parent = 5

[component_specs.b]
component_role = "b"
maximum_instances = 8

[component_specs.c]
component_role = "c"
maximum_instances = 8
"#;

fn parse(source: &str) -> Result<(ConfigModel, ComponentGroupDeploymentTopology), ConfigError> {
    let config = Config::parse_toml(&format!("{CONFIG_PREFIX}\n{source}"))?;
    let topology = config.compile_component_group_deployment_topology()?;
    Ok((config, topology))
}

fn deployment(value: &str) -> ComponentGroupDeploymentId {
    value.parse().expect("Component Group deployment ID")
}

fn deployment_envelope_source(
    initial_placements: u32,
    maximum_placements: u32,
    maximum_per_root: u32,
    minimum_distinct_roots: u32,
) -> String {
    format!(
        r#"
[component_groups.cell.components.a]
component_spec = "a"
[component_group_deployments.cell]
component_group = "cell"
initial_placements = {initial_placements}
maximum_placements = {maximum_placements}
placement.maximum_per_root = {maximum_per_root}
placement.minimum_distinct_roots = {minimum_distinct_roots}
"#
    )
}

#[test]
fn independent_deployments_flatten_every_occurrence_without_implicit_execution() {
    let (config, topology) = parse(
        r#"
[component_groups.group_two.components.c]
component_spec = "c"

[component_groups.group_one.components.a]
component_spec = "a"

[component_groups.group_one.components.b]
component_spec = "b"

[component_groups.group_one.groups.group_two]
component_group = "group_two"

[component_group_deployments.outer]
component_group = "group_one"
initial_placements = 1
maximum_placements = 1
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1

[component_group_deployments.inner]
component_group = "group_two"
initial_placements = 0
maximum_placements = 1
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1
"#,
    )
    .expect("valid independent deployments");
    let outer = topology
        .get(&deployment("outer"))
        .expect("outer deployment");
    let inner = topology
        .get(&deployment("inner"))
        .expect("inner deployment");

    assert_eq!(outer.members.len(), 3);
    assert_eq!(inner.members.len(), 1);
    assert_eq!(
        outer
            .members
            .iter()
            .map(|member| member.component_spec.as_str())
            .collect::<Vec<_>>(),
        vec!["a", "b", "c"]
    );
    assert_eq!(
        outer.members[2]
            .member_path
            .as_slice()
            .iter()
            .map(crate::ids::ComponentGroupMemberId::as_str)
            .collect::<Vec<_>>(),
        vec!["group_two", "c"]
    );
    assert_eq!(inner.members[0].component_spec.as_str(), "c");
    assert_eq!(inner.members[0].member_path.as_slice()[0].as_str(), "c");

    let component_topology = config
        .compile_component_topology()
        .expect("Component topology");
    for member in outer.members.iter().chain(&inner.members) {
        assert_eq!(
            member.component_spec_hash,
            component_topology
                .get(&member.component_spec)
                .expect("compiled Component Spec")
                .spec_hash
        );
    }
}

#[test]
fn deployment_source_order_does_not_change_the_canonical_projection() {
    let declarations = r#"
[component_groups.cell.components.a]
component_spec = "a"
"#;
    let left = parse(&format!(
        "{declarations}\n{}",
        r#"
[component_group_deployments.zeta]
component_group = "cell"
initial_placements = 0
maximum_placements = 1
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1

[component_group_deployments.alpha]
component_group = "cell"
initial_placements = 1
maximum_placements = 1
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1
"#
    ))
    .expect("left deployments")
    .1;
    let right = parse(&format!(
        "{declarations}\n{}",
        r#"
[component_group_deployments.alpha]
component_group = "cell"
initial_placements = 1
maximum_placements = 1
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1

[component_group_deployments.zeta]
component_group = "cell"
initial_placements = 0
maximum_placements = 1
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1
"#
    ))
    .expect("right deployments")
    .1;

    assert_eq!(left, right);
    assert_eq!(
        left.component_group_deployments
            .iter()
            .map(|candidate| candidate.deployment.as_str())
            .collect::<Vec<_>>(),
        vec!["alpha", "zeta"]
    );
}

#[test]
fn maximum_placement_demand_counts_each_deployment_and_member_occurrence() {
    let exact = r#"
[component_groups.repeated.components.left]
component_spec = "a"

[component_groups.repeated.components.right]
component_spec = "a"

[component_group_deployments.first]
component_group = "repeated"
initial_placements = 1
maximum_placements = 2
placement.maximum_per_root = 2
placement.minimum_distinct_roots = 1

[component_group_deployments.second]
component_group = "repeated"
initial_placements = 0
maximum_placements = 2
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 2
"#;
    parse(exact).expect("eight maximum a occurrences fit the Spec");

    let excessive = parse(&format!(
        "{exact}\n{}",
        r#"
[component_group_deployments.third]
component_group = "repeated"
initial_placements = 0
maximum_placements = 1
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1
"#
    ))
    .expect_err("two more maximum occurrences must exceed the Spec");
    assert!(matches!(
        excessive,
        ConfigError::ComponentGroupDeploymentTopology(
            ComponentGroupDeploymentTopologyError::ComponentSpecDemandExceedsMaximum {
                required: 10,
                maximum_fleet_instances: 8,
                ..
            }
        )
    ));
}

#[test]
fn missing_groups_and_unknown_future_fields_reject() {
    let missing_group = parse(
        r#"
[component_group_deployments.missing]
component_group = "missing"
initial_placements = 0
maximum_placements = 1
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1
"#,
    )
    .expect_err("unknown group must reject");
    assert!(matches!(
        missing_group,
        ConfigError::ComponentGroupDeploymentTopology(
            ComponentGroupDeploymentTopologyError::ComponentGroupTopology(
                ComponentGroupTopologyError::UnknownGroup { .. }
            )
        )
    ));

    let unknown_future_field = Config::parse_toml(&format!(
        "{CONFIG_PREFIX}\n{}",
        r#"
[component_groups.cell.components.a]
component_spec = "a"
[component_group_deployments.cell]
component_group = "cell"
protected_context = "not-yet-available"
initial_placements = 1
maximum_placements = 1
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1
"#
    ))
    .expect_err("protected context must remain unavailable until its compiler lands");
    assert!(matches!(
        unknown_future_field,
        ConfigError::CannotParseToml { .. }
    ));
}

#[test]
fn deployments_reuse_one_group_with_distinct_reduction_only_limits() {
    let (_, topology) = parse(
        r#"
[component_groups.shared.components.a]
component_spec = "a"

[component_groups.cell.groups.shared]
component_group = "shared"

[component_group_deployments.large]
component_group = "cell"
initial_placements = 1
maximum_placements = 1
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1

[[component_group_deployments.large.member_limits]]
member = ["shared", "a"]
maximum_descendants = 20_000
maximum_registry_bytes = 16_777_216
spawn_grants = [
  { parent_role = "a", child_role = "child", maximum_instances_per_parent = 10 },
  { parent_role = "child", child_role = "once", maximum_instances_per_parent = 1 },
]

[component_group_deployments.small]
component_group = "cell"
initial_placements = 1
maximum_placements = 1
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1

[[component_group_deployments.small.member_limits]]
member = ["shared", "a"]
maximum_descendants = 4_000
maximum_registry_bytes = 8_388_608
spawn_grants = [
  { parent_role = "a", child_role = "child", maximum_instances_per_parent = 2 },
]
"#,
    )
    .expect("same group with distinct deployment reductions");

    let large = topology
        .get(&deployment("large"))
        .expect("large deployment");
    let small = topology
        .get(&deployment("small"))
        .expect("small deployment");
    assert!(large.member_limits.is_empty());
    assert_eq!(large.members[0].limits.maximum_descendants, 20_000);
    assert_eq!(large.members[0].limits.maximum_registry_bytes, 16_777_216);
    assert!(large.members[0].limits.spawn_grant_reductions.is_empty());

    assert_eq!(small.member_limits.len(), 1);
    assert_eq!(small.members[0].limits.maximum_descendants, 4_000);
    assert_eq!(small.members[0].limits.maximum_registry_bytes, 8_388_608);
    assert_eq!(small.members[0].limits.spawn_grant_reductions.len(), 1);
    assert_eq!(
        small.members[0].limits.spawn_grant_reductions[0].maximum_instances_per_parent,
        2
    );
}

#[test]
fn member_limit_paths_and_grants_are_exact_unique_and_canonical() {
    let base = r#"
[component_groups.cell.components.a]
component_spec = "a"
[component_group_deployments.cell]
component_group = "cell"
initial_placements = 1
maximum_placements = 1
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1
"#;
    let unknown = parse(&format!(
        "{base}\n{}",
        r#"
[[component_group_deployments.cell.member_limits]]
member = ["missing"]
maximum_descendants = 1
"#
    ))
    .expect_err("unknown member path must reject");
    assert!(matches!(
        unknown,
        ConfigError::ComponentGroupDeploymentTopology(
            ComponentGroupDeploymentTopologyError::MemberLimit(
                ComponentDeploymentMemberLimitError::UnknownMemberLimitPath { .. }
            )
        )
    ));

    let duplicate_path = parse(&format!(
        "{base}\n{}",
        r#"
[[component_group_deployments.cell.member_limits]]
member = ["a"]
maximum_descendants = 10
[[component_group_deployments.cell.member_limits]]
member = ["a"]
maximum_registry_bytes = 10
"#
    ))
    .expect_err("duplicate member path must reject");
    assert!(matches!(
        duplicate_path,
        ConfigError::ComponentGroupDeploymentTopology(
            ComponentGroupDeploymentTopologyError::MemberLimit(
                ComponentDeploymentMemberLimitError::DuplicateMemberLimitPath { .. }
            )
        )
    ));

    let duplicate_grant = parse(&format!(
        "{base}\n{}",
        r#"
[[component_group_deployments.cell.member_limits]]
member = ["a"]
spawn_grants = [
  { parent_role = "a", child_role = "child", maximum_instances_per_parent = 2 },
  { parent_role = "a", child_role = "child", maximum_instances_per_parent = 3 },
]
"#
    ))
    .expect_err("duplicate spawn-grant reduction must reject");
    assert!(matches!(
        duplicate_grant,
        ConfigError::ComponentGroupDeploymentTopology(
            ComponentGroupDeploymentTopologyError::MemberLimit(
                ComponentDeploymentMemberLimitError::DuplicateSpawnGrantLimit { .. }
            )
        )
    ));
}

#[test]
fn member_limit_source_order_does_not_change_canonical_policy() {
    let (_, topology) = parse(
        r#"
[component_groups.cell.components.a]
component_spec = "a"
[component_groups.cell.components.b]
component_spec = "b"

[component_group_deployments.cell]
component_group = "cell"
initial_placements = 1
maximum_placements = 1
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1

[[component_group_deployments.cell.member_limits]]
member = ["b"]
maximum_descendants = 100

[[component_group_deployments.cell.member_limits]]
member = ["a"]
spawn_grants = [
  { parent_role = "child", child_role = "child", maximum_instances_per_parent = 3 },
  { parent_role = "a", child_role = "child", maximum_instances_per_parent = 2 },
]
"#,
    )
    .expect("out-of-order source reductions");
    let limits = &topology.component_group_deployments[0].member_limits;
    assert_eq!(
        limits
            .iter()
            .map(|limit| limit.member.as_slice()[0].as_str())
            .collect::<Vec<_>>(),
        vec!["a", "b"]
    );
    assert_eq!(
        limits[0]
            .spawn_grants
            .iter()
            .map(|grant| grant.parent_role.as_str())
            .collect::<Vec<_>>(),
        vec!["a", "child"]
    );
}

#[test]
fn zero_raised_unknown_and_invalid_singleton_limits_reject() {
    #[derive(Clone, Copy)]
    enum Expected {
        AggregateAboveSpec,
        InvalidSingleton,
        SpawnGrantAboveSpec,
        UnknownSpawnGrant,
        ZeroAggregate,
        ZeroSpawnGrant,
    }

    let base = r#"
[component_groups.cell.components.a]
component_spec = "a"
[component_group_deployments.cell]
component_group = "cell"
initial_placements = 1
maximum_placements = 1
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1
"#;
    let cases = [
        (
            "maximum_descendants = 0",
            "zero aggregate",
            Expected::ZeroAggregate,
        ),
        (
            "maximum_registry_bytes = 20_000_000",
            "raised aggregate",
            Expected::AggregateAboveSpec,
        ),
        (
            "spawn_grants = [{ parent_role = \"a\", child_role = \"child\", maximum_instances_per_parent = 0 }]",
            "zero spawn grant",
            Expected::ZeroSpawnGrant,
        ),
        (
            "spawn_grants = [{ parent_role = \"a\", child_role = \"child\", maximum_instances_per_parent = 11 }]",
            "raised spawn grant",
            Expected::SpawnGrantAboveSpec,
        ),
        (
            "spawn_grants = [{ parent_role = \"child\", child_role = \"missing\", maximum_instances_per_parent = 1 }]",
            "unknown spawn grant",
            Expected::UnknownSpawnGrant,
        ),
        (
            "spawn_grants = [{ parent_role = \"child\", child_role = \"once\", maximum_instances_per_parent = 2 }]",
            "invalid Singleton grant",
            Expected::InvalidSingleton,
        ),
    ];
    for (limit, expectation, expected) in cases {
        let error = parse(&format!(
            "{base}\n[[component_group_deployments.cell.member_limits]]\nmember = [\"a\"]\n{limit}"
        ))
        .expect_err(expectation);
        let ConfigError::ComponentGroupDeploymentTopology(
            ComponentGroupDeploymentTopologyError::MemberLimit(error),
        ) = error
        else {
            panic!("expected typed member-limit error for {expectation}");
        };
        assert!(matches!(
            (expected, &error),
            (
                Expected::AggregateAboveSpec,
                ComponentDeploymentMemberLimitError::AggregateLimitExceedsSpec { .. }
            ) | (
                Expected::InvalidSingleton,
                ComponentDeploymentMemberLimitError::InvalidSingletonSpawnGrantLimit { .. }
            ) | (
                Expected::SpawnGrantAboveSpec,
                ComponentDeploymentMemberLimitError::SpawnGrantLimitExceedsSpec { .. }
            ) | (
                Expected::UnknownSpawnGrant,
                ComponentDeploymentMemberLimitError::UnknownSpawnGrant { .. }
            ) | (
                Expected::ZeroAggregate,
                ComponentDeploymentMemberLimitError::ZeroAggregateLimit { .. }
            ) | (
                Expected::ZeroSpawnGrant,
                ComponentDeploymentMemberLimitError::ZeroSpawnGrantLimit { .. }
            )
        ));
    }
}

#[test]
fn deployment_labels_inherit_without_altering_typed_purpose() {
    let (_, topology) = parse(
        r#"
[component_groups.services.components.database]
component_spec = "a"
service = "database"
service_purpose = "replica"
labels = { replica = "authority", zone = "west" }

[component_groups.cell.groups.services]
component_group = "services"
labels = { tier = "storage" }

[component_group_deployments.cell]
component_group = "cell"
labels = { authority = "writer", workload = "database" }
initial_placements = 1
maximum_placements = 1
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1

[services.fleet.targets.database]
role = "a"
component_spec = "a"
mode = "authority_replica"
authority_deployment = "authority"
authority_member = ["database"]
placement.maximum_members_per_root = 1
placement.minimum_distinct_roots = 1

[component_groups.authority.components.database]
component_spec = "a"
service = "database"
service_purpose = "authority"

[component_group_deployments.authority]
component_group = "authority"
initial_placements = 1
maximum_placements = 1
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1
"#,
    )
    .expect("valid inert labels");
    let cell = topology.get(&deployment("cell")).expect("cell deployment");
    let member = &cell.members[0];

    assert!(matches!(
        member.purpose,
        ComponentDeploymentPurpose::FleetServiceMember {
            member_purpose: FleetServiceMemberPurpose::Replica,
            ..
        }
    ));
    assert_eq!(
        member
            .labels
            .iter()
            .map(|label| (label.key.as_str(), label.value.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("authority", "writer"),
            ("replica", "authority"),
            ("tier", "storage"),
            ("workload", "database"),
            ("zone", "west"),
        ]
    );
}

#[test]
fn deployment_label_cannot_override_an_inherited_key() {
    let duplicate = parse(
        r#"
[component_groups.cell.components.a]
component_spec = "a"
labels = { workload = "leaf" }

[component_group_deployments.cell]
component_group = "cell"
labels = { workload = "deployment" }
initial_placements = 1
maximum_placements = 1
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1
"#,
    )
    .expect_err("deployment/member duplicate label key must reject");

    assert!(matches!(
        duplicate,
        ConfigError::ComponentGroupDeploymentTopology(
            ComponentGroupDeploymentTopologyError::DuplicateEffectiveLabel { .. }
        )
    ));
}

#[test]
fn reusable_service_group_resolves_exact_purpose_per_deployment_path() {
    let (config, topology) = parse(
        r#"
[component_groups.databases.components.database]
component_spec = "a"
service = "database"

[component_groups.databases.components.helper]
component_spec = "b"

[component_groups.project_cell.components.hub]
component_spec = "b"
service = "project-hubs"
service_purpose = "pool_member"

[component_groups.project_cell.groups.databases]
component_group = "databases"
service_purpose = "replica"

[component_group_deployments.authoritative]
component_group = "databases"
service_purpose = "authority"
initial_placements = 1
maximum_placements = 1
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1

[component_group_deployments.projects]
component_group = "project_cell"
initial_placements = 1
maximum_placements = 2
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1

[services.fleet.targets.database]
role = "a"
component_spec = "a"
mode = "authority_replica"
authority_deployment = "authoritative"
authority_member = ["database"]
placement.maximum_members_per_root = 1
placement.minimum_distinct_roots = 1

[services.fleet.targets.project-hubs]
role = "b"
component_spec = "b"
mode = "active_pool"
placement.maximum_members_per_root = 2
placement.minimum_distinct_roots = 1
"#,
    )
    .expect("valid reusable service deployments");
    let authoritative = topology
        .get(&deployment("authoritative"))
        .expect("Authority deployment");
    let projects = topology
        .get(&deployment("projects"))
        .expect("project deployment");

    assert!(matches!(
        authoritative.members[0].purpose,
        ComponentDeploymentPurpose::FleetServiceMember {
            ref service,
            member_purpose: FleetServiceMemberPurpose::Authority,
        } if service.as_str() == "database"
    ));
    assert!(matches!(
        authoritative.members[1].purpose,
        ComponentDeploymentPurpose::Ordinary
    ));
    assert!(matches!(
        projects.members[0].purpose,
        ComponentDeploymentPurpose::FleetServiceMember {
            ref service,
            member_purpose: FleetServiceMemberPurpose::Replica,
        } if service.as_str() == "database"
    ));
    assert!(matches!(
        projects.members[1].purpose,
        ComponentDeploymentPurpose::Ordinary
    ));
    assert!(matches!(
        projects.members[2].purpose,
        ComponentDeploymentPurpose::FleetServiceMember {
            ref service,
            member_purpose: FleetServiceMemberPurpose::PoolMember,
        } if service.as_str() == "project-hubs"
    ));

    let encoded = candid::encode_one(&topology).expect("encode purpose topology");
    let decoded: ComponentGroupDeploymentTopology =
        candid::decode_one(&encoded).expect("decode purpose topology");
    assert_eq!(decoded, topology);
    decoded
        .validate(
            &config
                .compile_component_group_topology()
                .expect("Component Group topology"),
            &config
                .compile_component_topology()
                .expect("Component topology"),
        )
        .expect("decoded purpose topology remains exact");
}

#[test]
fn service_leaf_requires_exactly_one_purpose_assignment() {
    let missing = parse(
        r#"
[component_groups.database.components.database]
component_spec = "a"
service = "database"
[component_group_deployments.database]
component_group = "database"
initial_placements = 1
maximum_placements = 1
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1
"#,
    )
    .expect_err("missing service purpose must reject");
    assert!(matches!(
        missing,
        ConfigError::ComponentGroupDeploymentTopology(
            ComponentGroupDeploymentTopologyError::MissingServicePurposeAssignment { .. }
        )
    ));

    let duplicate = parse(
        r#"
[component_groups.database.components.database]
component_spec = "a"
service = "database"
service_purpose = "replica"
[component_group_deployments.database]
component_group = "database"
service_purpose = "authority"
initial_placements = 1
maximum_placements = 1
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1
"#,
    )
    .expect_err("multiple service purposes must reject");
    assert!(matches!(
        duplicate,
        ConfigError::ComponentGroupDeploymentTopology(
            ComponentGroupDeploymentTopologyError::MultipleServicePurposeAssignments {
                actual: 2,
                ..
            }
        )
    ));

    let unused = parse(
        r#"
[component_groups.ordinary.components.a]
component_spec = "a"
[component_group_deployments.ordinary]
component_group = "ordinary"
service_purpose = "authority"
initial_placements = 1
maximum_placements = 1
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1
"#,
    )
    .expect_err("unused deployment purpose must reject");
    assert!(matches!(
        unused,
        ConfigError::ComponentGroupDeploymentTopology(
            ComponentGroupDeploymentTopologyError::InapplicableServicePurposeAssignment { .. }
        )
    ));
}

#[test]
fn invalid_deployment_placement_envelopes_reject() {
    let zero_maximum =
        parse(&deployment_envelope_source(0, 0, 1, 1)).expect_err("zero maximum must reject");
    assert!(matches!(
        zero_maximum,
        ConfigError::ComponentGroupDeploymentTopology(
            ComponentGroupDeploymentTopologyError::ZeroMaximumPlacements { .. }
        )
    ));

    let excessive_initial = parse(&deployment_envelope_source(3, 2, 1, 1))
        .expect_err("initial placements over deployment maximum must reject");
    assert!(matches!(
        excessive_initial,
        ConfigError::ComponentGroupDeploymentTopology(
            ComponentGroupDeploymentTopologyError::InitialPlacementsExceedMaximum { .. }
        )
    ));

    let zero_density = parse(&deployment_envelope_source(1, 2, 0, 1))
        .expect_err("zero per-root density must reject");
    assert!(matches!(
        zero_density,
        ConfigError::ComponentGroupDeploymentTopology(
            ComponentGroupDeploymentTopologyError::ZeroMaximumPerRoot { .. }
        )
    ));

    let invalid_density = parse(&deployment_envelope_source(1, 2, 3, 1))
        .expect_err("density over deployment maximum must reject");
    assert!(matches!(
        invalid_density,
        ConfigError::ComponentGroupDeploymentTopology(
            ComponentGroupDeploymentTopologyError::MaximumPerRootExceedsMaximumPlacements { .. }
        )
    ));

    let zero_spread = parse(&deployment_envelope_source(1, 2, 1, 0))
        .expect_err("zero minimum spread must reject");
    assert!(matches!(
        zero_spread,
        ConfigError::ComponentGroupDeploymentTopology(
            ComponentGroupDeploymentTopologyError::ZeroMinimumDistinctRoots { .. }
        )
    ));

    let invalid_spread = parse(&deployment_envelope_source(1, 2, 1, 3))
        .expect_err("spread over deployment maximum must reject");
    assert!(matches!(
        invalid_spread,
        ConfigError::ComponentGroupDeploymentTopology(
            ComponentGroupDeploymentTopologyError::MinimumDistinctRootsExceedMaximumPlacements { .. }
        )
    ));
}

#[test]
fn decoded_projection_revalidates_order_members_and_component_spec_hashes() {
    let (config, topology) = parse(
        r#"
[component_groups.cell.components.a]
component_spec = "a"
[component_group_deployments.alpha]
component_group = "cell"
initial_placements = 1
maximum_placements = 1
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1
[component_group_deployments.zeta]
component_group = "cell"
initial_placements = 0
maximum_placements = 1
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1
"#,
    )
    .expect("valid decoded projection fixture");
    let groups = config
        .compile_component_group_topology()
        .expect("group graph");
    let components = config
        .compile_component_topology()
        .expect("Component topology");

    let mut wrong_hash = topology.clone();
    wrong_hash.component_group_deployments[0].members[0].component_spec_hash = [0; 32];
    assert!(matches!(
        wrong_hash.validate(&groups, &components),
        Err(ComponentGroupDeploymentTopologyError::ComponentSpecHashMismatch { .. })
    ));

    let mut wrong_purpose = topology.clone();
    wrong_purpose.component_group_deployments[0].members[0].purpose =
        ComponentDeploymentPurpose::FleetServiceMember {
            service: "fabricated".parse().expect("Fleet service ID"),
            member_purpose: FleetServiceMemberPurpose::Authority,
        };
    assert!(matches!(
        wrong_purpose.validate(&groups, &components),
        Err(ComponentGroupDeploymentTopologyError::MemberProjectionMismatch { .. })
    ));

    let mut wrong_labels = topology.clone();
    wrong_labels.component_group_deployments[0].members[0]
        .labels
        .push(ComponentDeploymentLabel {
            key: ComponentDeploymentLabelKey::try_from("fabricated".to_string())
                .expect("label key"),
            value: ComponentDeploymentLabelValue::try_from("value".to_string())
                .expect("label value"),
        });
    assert!(matches!(
        wrong_labels.validate(&groups, &components),
        Err(ComponentGroupDeploymentTopologyError::MemberLabelProjectionMismatch { .. })
    ));

    let mut wrong_limits = topology.clone();
    wrong_limits.component_group_deployments[0].members[0]
        .limits
        .maximum_descendants = 1;
    assert!(matches!(
        wrong_limits.validate(&groups, &components),
        Err(ComponentGroupDeploymentTopologyError::MemberLimit(
            ComponentDeploymentMemberLimitError::EffectiveLimitProjectionMismatch { .. }
        ))
    ));

    let mut redundant_limit = topology.clone();
    let member = redundant_limit.component_group_deployments[0].members[0]
        .member_path
        .clone();
    redundant_limit.component_group_deployments[0]
        .member_limits
        .push(ComponentDeploymentMemberLimit {
            member,
            maximum_descendants: Some(20_000),
            maximum_registry_bytes: None,
            spawn_grants: Vec::new(),
        });
    assert!(matches!(
        redundant_limit.validate(&groups, &components),
        Err(ComponentGroupDeploymentTopologyError::MemberLimit(
            ComponentDeploymentMemberLimitError::NonCanonicalMemberLimitProjection { .. }
        ))
    ));

    let mut wrong_order = topology;
    wrong_order.component_group_deployments.swap(0, 1);
    assert!(matches!(
        wrong_order.validate(&groups, &components),
        Err(ComponentGroupDeploymentTopologyError::NonCanonicalDeploymentOrder { .. })
    ));
}
