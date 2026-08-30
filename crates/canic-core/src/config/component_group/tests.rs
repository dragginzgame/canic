//! Focused proofs for bounded Component Group graph compilation and flattening.

use super::*;
use crate::config::{
    Config, ConfigError,
    schema::{
        ComponentGroupComponentConfig, ComponentGroupIncludeConfig, ComponentGroupSpecConfig,
        ConfigModel,
    },
};
use std::{collections::BTreeMap, fmt::Write as _};

const CONFIG_PREFIX: &str = r#"
[app]
name = "composition"

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

fn parse(source: &str) -> Result<ComponentGroupTopology, ConfigError> {
    let config = Config::parse_toml(&format!("{CONFIG_PREFIX}\n{source}"))?;
    Ok(config.compile_component_group_topology()?)
}

fn group(value: &str) -> ComponentGroupSpecId {
    value.parse().expect("Component Group Spec ID")
}

fn nested_group_source(path_segments: usize) -> String {
    assert!(path_segments > 0, "test path must contain a member");
    let mut source = String::new();
    for index in 0..path_segments {
        if index + 1 == path_segments {
            write!(
                &mut source,
                "[component_groups.g{index}.components.a]\ncomponent_spec = \"a\"\n"
            )
            .expect("write depth fixture");
        } else {
            let next = index + 1;
            write!(
                &mut source,
                "[component_groups.g{index}.groups.g{next}]\ncomponent_group = \"g{next}\"\n"
            )
            .expect("write depth fixture");
        }
    }
    source
}

fn repeated_component_group_source(member_count: usize) -> String {
    let mut source = String::new();
    for index in 0..member_count {
        write!(
            &mut source,
            "[component_groups.repeated.components.m{index:03}]\ncomponent_spec = \"a\"\n"
        )
        .expect("write member-count fixture");
    }
    source
}

fn config_model() -> ConfigModel {
    Config::parse_toml(CONFIG_PREFIX).expect("valid Component Group test config")
}

fn component_member() -> ComponentGroupComponentConfig {
    ComponentGroupComponentConfig {
        component_spec: "a".parse().expect("Component Spec ID"),
        service: None,
        service_purpose: None,
        labels: BTreeMap::new(),
    }
}

fn group_with_component_members(member_count: usize) -> ComponentGroupSpecConfig {
    let components = (0..member_count)
        .map(|index| {
            (
                format!("m{index:03}")
                    .parse()
                    .expect("Component Group member ID"),
                component_member(),
            )
        })
        .collect();
    ComponentGroupSpecConfig {
        components,
        groups: BTreeMap::new(),
    }
}

fn group_with_inclusions(
    inclusion_count: usize,
    target: &ComponentGroupSpecId,
) -> ComponentGroupSpecConfig {
    let groups = (0..inclusion_count)
        .map(|index| {
            (
                format!("i{index:03}")
                    .parse()
                    .expect("Component Group inclusion ID"),
                ComponentGroupIncludeConfig {
                    component_group: target.clone(),
                    service_purpose: None,
                    labels: BTreeMap::new(),
                },
            )
        })
        .collect();
    ComponentGroupSpecConfig {
        components: BTreeMap::new(),
        groups,
    }
}

#[test]
fn nested_groups_flatten_occurrence_by_occurrence_in_canonical_path_order() {
    let topology = parse(
        r#"
[component_groups.group_two.components.c]
component_spec = "c"

[component_groups.group_one.components.a]
component_spec = "a"

[component_groups.group_one.components.b]
component_spec = "b"

[component_groups.group_one.groups.group_two]
component_group = "group_two"
"#,
    )
    .expect("valid nested groups");
    let group_one = topology.flatten(&group("group_one")).expect("group one");
    let group_two = topology.flatten(&group("group_two")).expect("group two");

    assert_eq!(
        group_one
            .components
            .iter()
            .map(|member| {
                (
                    member
                        .member_path
                        .as_slice()
                        .iter()
                        .map(ComponentGroupMemberId::as_str)
                        .collect::<Vec<_>>(),
                    member.component_spec.as_str(),
                )
            })
            .collect::<Vec<_>>(),
        vec![
            (vec!["a"], "a"),
            (vec!["b"], "b"),
            (vec!["group_two", "c"], "c"),
        ]
    );
    assert_eq!(group_two.components.len(), 1);

    let independently_selected = group_one
        .components
        .iter()
        .chain(group_two.components.iter())
        .filter(|member| member.component_spec.as_str() == "c")
        .count();
    assert_eq!(independently_selected, 2);
}

#[test]
fn equal_component_specs_on_distinct_paths_are_not_deduplicated() {
    let topology = parse(
        r#"
[component_groups.repeated.components.left]
component_spec = "a"

[component_groups.repeated.components.right]
component_spec = "a"
"#,
    )
    .expect("valid repeated Component Spec");
    let flattened = topology.flatten(&group("repeated")).expect("flatten group");

    assert_eq!(flattened.components.len(), 2);
    assert_eq!(
        flattened
            .components
            .iter()
            .map(|member| member.member_path.as_slice()[0].as_str())
            .collect::<Vec<_>>(),
        vec!["left", "right"]
    );
    assert!(
        flattened
            .components
            .iter()
            .all(|member| member.component_spec.as_str() == "a")
    );
}

#[test]
fn canonical_graph_bytes_ignore_source_table_order() {
    let left = parse(
        r#"
[component_groups.cell.components.b]
component_spec = "b"
[component_groups.cell.components.a]
component_spec = "a"
labels = { zone = "west", tier = "api" }
"#,
    )
    .expect("left graph");
    let right = parse(
        r#"
[component_groups.cell.components.a]
component_spec = "a"
labels = { tier = "api", zone = "west" }
[component_groups.cell.components.b]
component_spec = "b"
"#,
    )
    .expect("right graph");

    assert_eq!(left, right);
    assert_eq!(
        left.canonical_bytes().expect("left canonical bytes"),
        right.canonical_bytes().expect("right canonical bytes")
    );
}

#[test]
fn component_group_graph_has_exact_schema_one_golden_bytes() {
    let topology = parse(
        r#"
[component_groups.cell.components.a]
component_spec = "a"
labels = { tier = "api" }
"#,
    )
    .expect("golden graph");
    let mut expected = Vec::new();
    expected.extend_from_slice(&30_u64.to_be_bytes());
    expected.extend_from_slice(b"canic/component-group-graph/v1");
    expected.extend_from_slice(&1_u32.to_be_bytes());
    expected.extend_from_slice(&1_u64.to_be_bytes());
    expected.extend_from_slice(&4_u64.to_be_bytes());
    expected.extend_from_slice(b"cell");
    expected.extend_from_slice(&1_u64.to_be_bytes());
    expected.push(0);
    expected.extend_from_slice(&1_u64.to_be_bytes());
    expected.extend_from_slice(b"a");
    expected.extend_from_slice(&1_u64.to_be_bytes());
    expected.extend_from_slice(b"a");
    expected.push(0);
    expected.push(0);
    expected.extend_from_slice(&1_u64.to_be_bytes());
    expected.extend_from_slice(&4_u64.to_be_bytes());
    expected.extend_from_slice(b"tier");
    expected.extend_from_slice(&3_u64.to_be_bytes());
    expected.extend_from_slice(b"api");

    assert_eq!(
        topology.canonical_bytes().expect("canonical bytes"),
        expected
    );
}

#[test]
fn nested_group_labels_inherit_in_canonical_key_order() {
    let topology = parse(
        r#"
[component_groups.inner.components.a]
component_spec = "a"
labels = { zone = "primary", authority = "metadata-only" }

[component_groups.outer.groups.inner]
component_group = "inner"
labels = { workload = "database" }
"#,
    )
    .expect("valid inherited labels");
    let flattened = topology.flatten(&group("outer")).expect("flatten outer");

    assert_eq!(
        flattened.components[0]
            .labels
            .iter()
            .map(|label| (label.key.as_str(), label.value.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("authority", "metadata-only"),
            ("workload", "database"),
            ("zone", "primary"),
        ]
    );
}

#[test]
fn duplicate_label_key_on_one_inclusion_path_rejects() {
    let duplicate = parse(
        r#"
[component_groups.inner.components.a]
component_spec = "a"
labels = { workload = "leaf" }

[component_groups.outer.groups.inner]
component_group = "inner"
labels = { workload = "include" }
"#,
    )
    .expect_err("duplicate inherited label key must reject");

    assert!(matches!(
        duplicate,
        ConfigError::ComponentGroupTopology(
            ComponentGroupTopologyError::DuplicateEffectiveLabel { .. }
        )
    ));
}

#[test]
fn label_text_and_effective_count_are_bounded() {
    ComponentDeploymentLabelKey::try_from("k".repeat(MAX_COMPONENT_DEPLOYMENT_LABEL_KEY_BYTES))
        .expect("maximum label key must decode");
    ComponentDeploymentLabelValue::try_from("v".repeat(MAX_COMPONENT_DEPLOYMENT_LABEL_VALUE_BYTES))
        .expect("maximum label value must decode");
    assert!(matches!(
        ComponentDeploymentLabelValue::try_from("line\nbreak".to_string()),
        Err(ComponentDeploymentLabelParseError::InvalidValueCharacters)
    ));

    let long_key = "k".repeat(MAX_COMPONENT_DEPLOYMENT_LABEL_KEY_BYTES + 1);
    let invalid_key = Config::parse_toml(&format!(
        "{CONFIG_PREFIX}\n[component_groups.cell.components.a]\ncomponent_spec = \"a\"\nlabels = {{ {long_key} = \"value\" }}\n"
    ))
    .expect_err("first excessive label key must reject during decode");
    assert!(matches!(invalid_key, ConfigError::CannotParseToml { .. }));

    let long_value = "v".repeat(MAX_COMPONENT_DEPLOYMENT_LABEL_VALUE_BYTES + 1);
    let invalid_value = Config::parse_toml(&format!(
        "{CONFIG_PREFIX}\n[component_groups.cell.components.a]\ncomponent_spec = \"a\"\nlabels = {{ key = \"{long_value}\" }}\n"
    ))
    .expect_err("first excessive label value must reject during decode");
    assert!(matches!(invalid_value, ConfigError::CannotParseToml { .. }));

    let mut labels = String::new();
    for index in 0..MAX_COMPONENT_DEPLOYMENT_LABELS {
        if index > 0 {
            labels.push_str(", ");
        }
        write!(&mut labels, "k{index} = \"v\"").expect("write label fixture");
    }
    parse(&format!(
        "[component_groups.cell.components.a]\ncomponent_spec = \"a\"\nlabels = {{ {labels} }}\n"
    ))
    .expect("maximum effective label count must compile");

    let first_excess = MAX_COMPONENT_DEPLOYMENT_LABELS;
    write!(&mut labels, ", k{first_excess} = \"v\"").expect("write first excessive label");
    let excessive = parse(&format!(
        "[component_groups.cell.components.a]\ncomponent_spec = \"a\"\nlabels = {{ {labels} }}\n"
    ))
    .expect_err("first excessive effective label count must reject");
    assert!(matches!(
        excessive,
        ConfigError::ComponentGroupTopology(ComponentGroupTopologyError::LabelBoundExceeded {
            actual,
            maximum,
            ..
        }) if actual == MAX_COMPONENT_DEPLOYMENT_LABELS + 1
            && maximum == MAX_COMPONENT_DEPLOYMENT_LABELS
    ));
}

#[test]
fn inclusion_purpose_applies_only_to_service_bearing_descendants() {
    let topology = parse(
        r#"
[component_groups.services.components.database]
component_spec = "a"
service = "database"

[component_groups.services.components.ordinary]
component_spec = "b"

[component_groups.cell.groups.services]
component_group = "services"
service_purpose = "replica"
"#,
    )
    .expect("valid mixed service inclusion");
    let flattened = topology.flatten(&group("cell")).expect("flatten cell");

    assert!(matches!(
        flattened.components[0].kind,
        ComponentGroupLeafKind::FleetService { ref service }
            if service.as_str() == "database"
    ));
    assert_eq!(
        flattened.components[0].service_purpose_assignments,
        vec![FleetServiceMemberPurpose::Replica]
    );
    assert!(matches!(
        flattened.components[1].kind,
        ComponentGroupLeafKind::Ordinary
    ));
    assert!(
        flattened.components[1]
            .service_purpose_assignments
            .is_empty()
    );
}

#[test]
fn purpose_assignment_without_a_service_leaf_rejects() {
    let leaf = parse(
        r#"
[component_groups.cell.components.a]
component_spec = "a"
service_purpose = "authority"
"#,
    )
    .expect_err("ordinary leaf purpose must reject");
    assert!(matches!(
        leaf,
        ConfigError::ComponentGroupTopology(
            ComponentGroupTopologyError::InapplicableServicePurposeAssignment { .. }
        )
    ));

    let inclusion = parse(
        r#"
[component_groups.ordinary.components.a]
component_spec = "a"
[component_groups.cell.groups.ordinary]
component_group = "ordinary"
service_purpose = "replica"
"#,
    )
    .expect_err("ordinary inclusion purpose must reject");
    assert!(matches!(
        inclusion,
        ConfigError::ComponentGroupTopology(
            ComponentGroupTopologyError::InapplicableServicePurposeAssignment { .. }
        )
    ));
}

#[test]
fn missing_references_duplicate_names_and_cycles_reject() {
    let unknown_spec = parse(
        r#"
[component_groups.cell.components.unknown]
component_spec = "unknown"
"#,
    )
    .expect_err("unknown Component Spec must reject");
    assert!(matches!(
        unknown_spec,
        ConfigError::ComponentGroupTopology(
            ComponentGroupTopologyError::UnknownComponentSpec { .. }
        )
    ));

    let unknown_group = parse(
        r#"
[component_groups.cell.groups.missing]
component_group = "missing"
"#,
    )
    .expect_err("unknown included Component Group must reject");
    assert!(matches!(
        unknown_group,
        ConfigError::ComponentGroupTopology(
            ComponentGroupTopologyError::UnknownIncludedGroup { .. }
        )
    ));

    let duplicate_member = parse(
        r#"
[component_groups.cell.components.shared]
component_spec = "a"
[component_groups.cell.groups.shared]
component_group = "other"
[component_groups.other.components.a]
component_spec = "a"
"#,
    )
    .expect_err("duplicate member must reject");
    assert!(matches!(
        duplicate_member,
        ConfigError::ComponentGroupTopology(ComponentGroupTopologyError::DuplicateMember { .. })
    ));

    let cycle = parse(
        r#"
[component_groups.one.groups.two]
component_group = "two"
[component_groups.two.groups.one]
component_group = "one"
"#,
    )
    .expect_err("inclusion cycle must reject");
    assert!(matches!(
        cycle,
        ConfigError::ComponentGroupTopology(ComponentGroupTopologyError::InclusionCycle { .. })
    ));
}

#[test]
fn strict_schema_and_member_path_depth_fail_before_use() {
    let unknown_field = Config::parse_toml(&format!(
        "{CONFIG_PREFIX}\n{}",
        r#"
[component_groups.cell.components.a]
component_spec = "a"
runtime_parent = true
"#
    ))
    .expect_err("unknown group member field must reject");
    assert!(matches!(unknown_field, ConfigError::CannotParseToml { .. }));

    let maximum_depth = crate::ids::COMPONENT_GROUP_MEMBER_PATH_MAX_SEGMENTS;
    parse(&nested_group_source(maximum_depth)).expect("maximum inclusion depth must compile");
    let excessive_depth = parse(&nested_group_source(maximum_depth + 1))
        .expect_err("first excessive inclusion depth must reject");
    assert!(matches!(
        excessive_depth,
        ConfigError::ComponentGroupTopology(ComponentGroupTopologyError::InvalidMemberPath { .. })
    ));
}

#[test]
fn direct_member_bound_accepts_the_limit_and_rejects_its_first_excess() {
    parse(&repeated_component_group_source(
        MAX_COMPONENT_GROUP_MEMBERS,
    ))
    .expect("maximum direct members must compile");
    let excessive = parse(&repeated_component_group_source(
        MAX_COMPONENT_GROUP_MEMBERS + 1,
    ))
    .expect_err("first excessive direct member must reject");

    assert!(matches!(
        excessive,
        ConfigError::ComponentGroupTopology(ComponentGroupTopologyError::MemberBoundExceeded {
            actual,
            maximum,
            ..
        }) if actual == MAX_COMPONENT_GROUP_MEMBERS + 1
            && maximum == MAX_COMPONENT_GROUP_MEMBERS
    ));
}

#[test]
fn graph_count_bounds_reject_their_first_excess_before_graph_validation() {
    let mut excessive_groups = config_model();
    for index in 0..=MAX_COMPONENT_GROUP_SPECS {
        excessive_groups.component_groups.insert(
            group(&format!("g{index:03}")),
            ComponentGroupSpecConfig::default(),
        );
    }
    assert!(matches!(
        excessive_groups.compile_component_group_topology(),
        Err(ComponentGroupTopologyError::GroupBoundExceeded {
            actual,
            maximum: MAX_COMPONENT_GROUP_SPECS,
        }) if actual == MAX_COMPONENT_GROUP_SPECS + 1
    ));

    let mut excessive_members = config_model();
    let full_groups = MAX_COMPONENT_GROUP_DECLARED_MEMBERS / MAX_COMPONENT_GROUP_MEMBERS;
    for index in 0..full_groups {
        excessive_members.component_groups.insert(
            group(&format!("g{index:03}")),
            group_with_component_members(MAX_COMPONENT_GROUP_MEMBERS),
        );
    }
    excessive_members
        .component_groups
        .insert(group("z_excess"), group_with_component_members(1));
    assert!(matches!(
        excessive_members.compile_component_group_topology(),
        Err(ComponentGroupTopologyError::DeclaredMemberBoundExceeded {
            actual,
            maximum: MAX_COMPONENT_GROUP_DECLARED_MEMBERS,
        }) if actual == MAX_COMPONENT_GROUP_DECLARED_MEMBERS + 1
    ));
}

#[test]
fn inclusion_and_flattened_member_bounds_reject_their_first_excess() {
    let target = group("zz_leaf");
    let mut excessive_inclusions = config_model();
    let full_groups = MAX_COMPONENT_GROUP_INCLUSIONS / MAX_COMPONENT_GROUP_MEMBERS;
    for index in 0..full_groups {
        excessive_inclusions.component_groups.insert(
            group(&format!("g{index:03}")),
            group_with_inclusions(MAX_COMPONENT_GROUP_MEMBERS, &target),
        );
    }
    excessive_inclusions
        .component_groups
        .insert(group("z_excess"), group_with_inclusions(1, &target));
    excessive_inclusions
        .component_groups
        .insert(target, group_with_component_members(1));
    assert!(matches!(
        excessive_inclusions.compile_component_group_topology(),
        Err(ComponentGroupTopologyError::InclusionBoundExceeded {
            actual,
            maximum: MAX_COMPONENT_GROUP_INCLUSIONS,
        }) if actual == MAX_COMPONENT_GROUP_INCLUSIONS + 1
    ));

    let mut excessive_flattening = config_model();
    let mut top = ComponentGroupSpecConfig::default();
    let full_groups = MAX_COMPONENT_GROUP_FLATTENED_MEMBERS / MAX_COMPONENT_GROUP_MEMBERS;
    for index in 0..=full_groups {
        let child = group(&format!("g{index:03}"));
        top.groups.insert(
            format!("i{index:03}")
                .parse()
                .expect("Component Group inclusion ID"),
            ComponentGroupIncludeConfig {
                component_group: child.clone(),
                service_purpose: None,
                labels: BTreeMap::new(),
            },
        );
        let member_count = if index == full_groups {
            1
        } else {
            MAX_COMPONENT_GROUP_MEMBERS
        };
        excessive_flattening
            .component_groups
            .insert(child, group_with_component_members(member_count));
    }
    excessive_flattening
        .component_groups
        .insert(group("a_top"), top);
    assert!(matches!(
        excessive_flattening.compile_component_group_topology(),
        Err(ComponentGroupTopologyError::FlattenedMemberBoundExceeded {
            actual,
            maximum: MAX_COMPONENT_GROUP_FLATTENED_MEMBERS,
            ..
        }) if actual == MAX_COMPONENT_GROUP_FLATTENED_MEMBERS + 1
    ));
}
