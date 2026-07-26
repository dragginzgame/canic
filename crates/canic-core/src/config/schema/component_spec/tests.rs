//! Module: config::schema::component_spec::tests
//!
//! Responsibility: verify the flat Component Spec schema and its local invariants.
//! Does not own: complete ConfigModel role ownership or Fleet-wide ceilings.
//! Boundary: focused parsing and validation checks for one Component Spec.

use super::*;
use crate::{
    cdk::types::TC,
    config::schema::{MAX_COMPONENT_PROVISIONING_GRANTS, NAME_MAX_BYTES, Validate},
};

fn component_spec(role: &str) -> ComponentSpecConfig {
    ComponentSpecConfig {
        component_role: CanisterRole::owned(role.to_string()),
        maximum_instances: 1,
        limits: ComponentLimitsConfig::default(),
        initial_cycles: defaults::initial_cycles(),
        topup: None,
        cycles_funding: CyclesFundingPolicyConfig::default(),
        scaling: None,
        sharding: None,
        binding: None,
        auth: CanisterAuthConfig::default(),
        standards: StandardsCanisterConfig::default(),
        diagnostics: DiagnosticsCanisterConfig::default(),
        metrics: MetricsCanisterConfig::default(),
        provisions: BTreeMap::new(),
        children: BTreeMap::new(),
    }
}

fn child(kind: ComponentChildKind, maximum_instances: u32) -> ComponentChildConfig {
    ComponentChildConfig {
        kind,
        initial_instances: 0,
        maximum_instances,
        initial_cycles: defaults::initial_cycles(),
        topup: None,
        cycles_funding: CyclesFundingPolicyConfig::default(),
        auth: CanisterAuthConfig::default(),
        standards: StandardsCanisterConfig::default(),
        diagnostics: DiagnosticsCanisterConfig::default(),
        metrics: MetricsCanisterConfig::default(),
    }
}

#[test]
fn minimal_component_spec_parses_with_one_component_role() {
    let config: ComponentSpecConfig = toml::from_str(
        r#"
component_role = "user_hub"
maximum_instances = 12
"#,
    )
    .expect("minimal Component Spec should parse");

    assert_eq!(config.component_role, CanisterRole::from("user_hub"));
    assert_eq!(config.maximum_instances, 12);
    assert!(config.provisions.is_empty());
    assert!(config.children.is_empty());
}

#[test]
fn component_and_child_defaults_are_finite() {
    let config: ComponentSpecConfig = toml::from_str(
        r#"
component_role = "user_hub"
maximum_instances = 1

[children.user_shard]
kind = "shard"
maximum_instances = 4
"#,
    )
    .expect("Component Spec should parse");
    let child = &config.children[&CanisterRole::from("user_shard")];

    assert_eq!(config.initial_cycles.to_u128(), 5 * TC);
    assert_eq!(config.cycles_funding.max_per_request.to_u128(), 5 * TC);
    assert_eq!(
        config.limits.maximum_children,
        DEFAULT_COMPONENT_MAXIMUM_CHILDREN
    );
    assert_eq!(
        config.limits.maximum_registry_bytes,
        DEFAULT_COMPONENT_MAXIMUM_REGISTRY_BYTES
    );
    assert_eq!(
        config.limits.cycles_funding.window_secs,
        DEFAULT_COMPONENT_CYCLES_FUNDING_WINDOW_SECS
    );
    assert_eq!(
        config.limits.cycles_funding.maximum_cycles.to_u128(),
        DEFAULT_COMPONENT_CYCLES_FUNDING_MAXIMUM_CYCLES
    );
    assert_eq!(child.initial_cycles.to_u128(), 5 * TC);
    assert_eq!(child.cycles_funding.max_per_child.to_u128(), 100 * TC);
    assert_eq!(child.initial_instances, 0);
}

#[test]
fn initial_children_and_peer_grants_parse_as_bounded_explicit_policy() {
    let config: ComponentSpecConfig = toml::from_str(
        r#"
component_role = "project_hub"
maximum_instances = 1

[provisions.project_instance]
maximum_instances_per_requester_per_root = 100

[children.required_ledger]
kind = "singleton"
initial_instances = 1
maximum_instances = 1

[children.optional_machine]
kind = "singleton"
initial_instances = 0
maximum_instances = 1
"#,
    )
    .expect("initial children and peer grant should parse");

    assert_eq!(
        config.provisions[&component_spec_id("project_instance")]
            .maximum_instances_per_requester_per_root,
        100,
    );
    assert_eq!(
        config.children[&CanisterRole::from("required_ledger")].initial_instances,
        1,
    );
    assert_eq!(
        config.children[&CanisterRole::from("optional_machine")].initial_instances,
        0,
    );
}

fn component_spec_id(value: &str) -> ComponentSpecId {
    value.parse().expect("valid Component Spec ID")
}

#[test]
fn component_aggregate_limits_are_positive_and_bound_declared_children() {
    let mut config = component_spec("hub");
    config.children.insert(
        CanisterRole::from("worker"),
        child(ComponentChildKind::Replica, 4),
    );
    config.limits.maximum_children = 3;
    config
        .validate()
        .expect("aggregate child cap may be lower than independent role ceilings");

    config.limits.maximum_children = 0;
    config
        .validate()
        .expect_err("a Component with children needs positive aggregate capacity");

    config.limits.maximum_children = 3;
    config.limits.maximum_registry_bytes = 0;
    config
        .validate()
        .expect_err("Component Registry bytes must be positive");

    config.limits.maximum_registry_bytes = 1;
    config.limits.cycles_funding.window_secs = 0;
    config
        .validate()
        .expect_err("Component funding window must be positive");

    config.limits.cycles_funding.window_secs = 1;
    config.limits.cycles_funding.maximum_cycles = Cycles::new(0);
    config
        .validate()
        .expect_err("Component funding budget must be positive");
}

#[test]
fn component_and_child_topup_tables_enable_defaults() {
    let config: ComponentSpecConfig = toml::from_str(
        r#"
component_role = "user_hub"
maximum_instances = 1
topup = {}

[children.user_shard]
kind = "shard"
maximum_instances = 4
topup = {}
"#,
    )
    .expect("empty topup tables should parse");

    for topup in [
        config.topup.as_ref().expect("Component topup"),
        config.children[&CanisterRole::from("user_shard")]
            .topup
            .as_ref()
            .expect("child topup"),
    ] {
        assert_eq!(topup.threshold.to_u128(), 10 * TC);
        assert_eq!(topup.amount.to_u128(), 5 * TC);
    }
}

#[test]
fn removed_recursive_and_tree_fields_do_not_parse() {
    for field in [
        "kind = \"service\"",
        "owner_component = \"other\"",
        "initial_trees = 1",
        "maximum_trees = 1",
    ] {
        let source = format!("component_role = \"hub\"\nmaximum_instances = 1\n{field}\n");
        toml::from_str::<ComponentSpecConfig>(&source)
            .expect_err("removed Component Spec field must reject");
    }

    toml::from_str::<ComponentSpecConfig>(
        r#"
component_role = "hub"
maximum_instances = 1

[children.worker]
kind = "replica"
maximum_instances = 2

[children.worker.children.nested]
kind = "shard"
maximum_instances = 1
"#,
    )
    .expect_err("a Component Child cannot own children");
}

#[test]
fn child_kind_rejects_root_service_and_component() {
    for kind in ["root", "service", "component"] {
        let source = format!("kind = \"{kind}\"\nmaximum_instances = 1\n");
        toml::from_str::<ComponentChildConfig>(&source)
            .expect_err("only direct child lifecycle kinds should parse");
    }
}

#[test]
fn component_and_child_instance_ceilings_are_positive() {
    let mut config = component_spec("hub");
    config.maximum_instances = 0;
    config
        .validate()
        .expect_err("zero Component ceiling must reject");

    let mut config = component_spec("hub");
    config.children.insert(
        CanisterRole::from("worker"),
        child(ComponentChildKind::Replica, 0),
    );
    config
        .validate()
        .expect_err("zero child ceiling must reject");
}

#[test]
fn singleton_child_ceiling_is_exactly_one() {
    let mut config = component_spec("hub");
    config.children.insert(
        CanisterRole::from("settings"),
        child(ComponentChildKind::Singleton, 2),
    );

    config
        .validate()
        .expect_err("singleton multiplicity above one must reject");
}

#[test]
fn initial_child_cardinality_is_bounded_by_role_and_component_limits() {
    let mut config = component_spec("hub");
    config.children.insert(
        CanisterRole::from("worker"),
        child(ComponentChildKind::Replica, 2),
    );
    config
        .children
        .get_mut("worker")
        .expect("worker")
        .initial_instances = 3;
    config
        .validate()
        .expect_err("initial count above role maximum must reject");

    config
        .children
        .get_mut("worker")
        .expect("worker")
        .initial_instances = 2;
    config.limits.maximum_children = 1;
    config
        .validate()
        .expect_err("aggregate initial count above Component limit must reject");
}

#[test]
fn provisioning_grants_require_positive_bounded_outbound_policy() {
    let mut config = component_spec("hub");
    config.provisions.insert(
        component_spec_id("worker"),
        ComponentProvisioningGrantConfig {
            maximum_instances_per_requester_per_root: 0,
        },
    );
    config
        .validate()
        .expect_err("zero per-requester/root grant limit must reject");

    config.provisions.clear();
    for ordinal in 0..=MAX_COMPONENT_PROVISIONING_GRANTS {
        config.provisions.insert(
            component_spec_id(&format!("target_{ordinal}")),
            ComponentProvisioningGrantConfig {
                maximum_instances_per_requester_per_root: 1,
            },
        );
    }
    config
        .validate()
        .expect_err("outbound grant count above the bound must reject");
}

#[test]
fn infrastructure_and_self_roles_are_rejected() {
    for role in [CanisterRole::ROOT, CanisterRole::WASM_STORE] {
        component_spec(role.as_str())
            .validate()
            .expect_err("infrastructure cannot be a Component role");
    }

    let mut config = component_spec("hub");
    config.children.insert(
        CanisterRole::from("hub"),
        child(ComponentChildKind::Replica, 1),
    );
    config
        .validate()
        .expect_err("a Component cannot be its own child");
}

#[test]
fn component_roles_and_pool_names_use_bounded_canonical_names() {
    component_spec(&"a".repeat(NAME_MAX_BYTES + 1))
        .validate()
        .expect_err("oversized Component role must reject");

    let mut config = component_spec("hub");
    config.children.insert(
        CanisterRole::from("worker"),
        child(ComponentChildKind::Replica, 2),
    );
    let mut scaling = ScalingConfig::default();
    scaling.pools.insert(
        "a".repeat(NAME_MAX_BYTES + 1),
        ScalePool {
            canister_role: CanisterRole::from("worker"),
            policy: ScalePoolPolicy {
                initial_workers: 1,
                min_workers: 1,
                max_workers: 2,
            },
        },
    );
    config.scaling = Some(scaling);

    config
        .validate()
        .expect_err("oversized pool name must reject");
}

#[test]
fn scaling_targets_only_a_declared_replica_child_within_its_ceiling() {
    let mut config = component_spec("hub");
    config.children.insert(
        CanisterRole::from("worker"),
        child(ComponentChildKind::Replica, 4),
    );
    config.scaling = Some(ScalingConfig {
        pools: BTreeMap::from([(
            "workers".to_string(),
            ScalePool {
                canister_role: CanisterRole::from("worker"),
                policy: ScalePoolPolicy {
                    initial_workers: 1,
                    min_workers: 1,
                    max_workers: 4,
                },
            },
        )]),
    });
    config.validate().expect("valid scaling edge should pass");

    config
        .scaling
        .as_mut()
        .expect("scaling")
        .pools
        .get_mut("workers")
        .expect("workers")
        .policy
        .max_workers = 5;
    config
        .validate()
        .expect_err("pool ceiling above direct child ceiling must reject");
}

#[test]
fn sharding_targets_only_a_declared_shard_child_within_its_ceiling() {
    let mut config = component_spec("hub");
    config.children.insert(
        CanisterRole::from("shard"),
        child(ComponentChildKind::Shard, 8),
    );
    config.sharding = Some(ShardingConfig {
        pools: BTreeMap::from([(
            "shards".to_string(),
            ShardPool {
                canister_role: CanisterRole::from("shard"),
                policy: ShardPoolPolicy {
                    capacity: 100,
                    initial_shards: 1,
                    max_shards: 8,
                },
            },
        )]),
    });
    config.validate().expect("valid sharding edge should pass");

    config.children.get_mut("shard").expect("shard").kind = ComponentChildKind::Replica;
    config
        .validate()
        .expect_err("sharding must not target a Replica child");
}

#[test]
fn binding_targets_only_a_declared_instance_child() {
    let mut config = component_spec("hub");
    config.children.insert(
        CanisterRole::from("instance"),
        child(ComponentChildKind::Instance, 100),
    );
    config.binding = Some(BindingConfig {
        pools: BTreeMap::from([(
            "projects".to_string(),
            BindingPool {
                canister_role: CanisterRole::from("instance"),
                key_name: "project".to_string(),
            },
        )]),
    });
    config.validate().expect("valid binding edge should pass");

    config
        .binding
        .as_mut()
        .expect("binding")
        .pools
        .get_mut("projects")
        .expect("projects")
        .key_name = String::new();
    config
        .validate()
        .expect_err("empty binding key must reject");
}

#[test]
fn cycles_funding_and_topup_limits_validate_for_components_and_children() {
    let mut config = component_spec("hub");
    config.cycles_funding.max_per_request = Cycles::new(2);
    config.cycles_funding.max_per_child = Cycles::new(1);
    config
        .validate()
        .expect_err("Component request limit above child limit must reject");

    let mut config = component_spec("hub");
    let mut worker = child(ComponentChildKind::Replica, 1);
    worker.topup = Some(TopupPolicy {
        threshold: Cycles::new(10),
        amount: Cycles::new(6),
    });
    config.children.insert(CanisterRole::from("worker"), worker);
    config
        .validate()
        .expect_err("child topup above half its threshold must reject");
}

#[test]
fn structural_role_lookup_returns_only_the_component_and_its_children() {
    let mut config = component_spec("hub");
    config.children.insert(
        CanisterRole::from("worker"),
        child(ComponentChildKind::Replica, 2),
    );

    assert_eq!(
        config
            .get_canister(&CanisterRole::from("hub"))
            .expect("Component")
            .kind,
        CanisterKind::Service,
    );
    assert_eq!(
        config
            .get_canister(&CanisterRole::from("worker"))
            .expect("child")
            .kind,
        CanisterKind::Replica,
    );
    assert!(config.get_canister(&CanisterRole::WASM_STORE).is_none());
    assert!(config.get_canister(&CanisterRole::from("other")).is_none());
}

#[test]
fn only_the_component_role_is_root_auto_created_and_directory_visible() {
    let mut config = component_spec("hub");
    config.children.insert(
        CanisterRole::from("worker"),
        child(ComponentChildKind::Replica, 2),
    );

    assert_eq!(
        config.auto_create_roles(),
        BTreeSet::from([CanisterRole::from("hub")]),
    );
    assert_eq!(
        config.component_directory_roles(),
        BTreeSet::from([CanisterRole::from("hub")]),
    );
}

#[test]
fn metrics_profiles_distinguish_component_child_root_and_store() {
    let component = component_spec("hub").component_canister_config();
    let child = child(ComponentChildKind::Shard, 1).canister_config();

    assert_eq!(
        component.resolved_metrics_profile(&CanisterRole::from("hub")),
        MetricsProfile::Leaf,
    );
    assert_eq!(
        child.resolved_metrics_profile(&CanisterRole::from("shard")),
        MetricsProfile::Leaf,
    );
    assert_eq!(
        implicit_root_canister_config().resolved_metrics_profile(&CanisterRole::ROOT),
        MetricsProfile::Root,
    );
    assert_eq!(
        implicit_wasm_store_canister_config().resolved_metrics_profile(&CanisterRole::WASM_STORE),
        MetricsProfile::Storage,
    );
}
