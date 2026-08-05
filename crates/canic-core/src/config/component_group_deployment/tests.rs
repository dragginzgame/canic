//! Focused proofs for independent deployment flattening and placement envelopes.

use super::*;
use crate::config::{Config, ConfigError};

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

[component_specs.a]
component_role = "a"
maximum_instances = 8

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
fn missing_groups_and_partial_future_fields_reject() {
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

    let partial_future_field = Config::parse_toml(&format!(
        "{CONFIG_PREFIX}\n{}",
        r#"
[component_groups.cell.components.a]
component_spec = "a"
[component_group_deployments.cell]
component_group = "cell"
labels = { tier = "future" }
initial_placements = 1
maximum_placements = 1
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1
"#
    ))
    .expect_err("labels must remain unavailable until their compiler lands");
    assert!(matches!(
        partial_future_field,
        ConfigError::CannotParseToml { .. }
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

    let mut wrong_order = topology;
    wrong_order.component_group_deployments.swap(0, 1);
    assert!(matches!(
        wrong_order.validate(&groups, &components),
        Err(ComponentGroupDeploymentTopologyError::NonCanonicalDeploymentOrder { .. })
    ));
}
