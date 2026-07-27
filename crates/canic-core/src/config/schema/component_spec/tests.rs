//! Module: config::schema::component_spec::tests
//!
//! Responsibility: verify the flat Component Spec schema and its local invariants.
//! Does not own: complete ConfigModel role ownership or Fleet-wide ceilings.
//! Boundary: focused parsing and validation checks for one Component Spec.

use super::*;
use crate::{
    cdk::types::TC,
    config::schema::{
        MAX_COMPONENT_PROVISIONING_GRANTS, MAX_COMPONENT_SPAWN_GRANTS, NAME_MAX_BYTES, Validate,
    },
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
        index: None,
        auth: CanisterAuthConfig::default(),
        standards: StandardsCanisterConfig::default(),
        diagnostics: DiagnosticsCanisterConfig::default(),
        metrics: MetricsCanisterConfig::default(),
        provisions: BTreeMap::new(),
        children: BTreeMap::new(),
        spawn_grants: BTreeMap::new(),
    }
}

fn child(kind: ComponentChildKind) -> ComponentChildConfig {
    ComponentChildConfig {
        kind,
        initial_cycles: defaults::initial_cycles(),
        topup: None,
        cycles_funding: CyclesFundingPolicyConfig::default(),
        scaling: None,
        sharding: None,
        index: None,
        auth: CanisterAuthConfig::default(),
        standards: StandardsCanisterConfig::default(),
        diagnostics: DiagnosticsCanisterConfig::default(),
        metrics: MetricsCanisterConfig::default(),
    }
}

fn admit_child(
    config: &mut ComponentSpecConfig,
    role: &str,
    kind: ComponentChildKind,
    maximum_instances_per_parent: u32,
) {
    let role = CanisterRole::owned(role.to_string());
    config.children.insert(role.clone(), child(kind));
    config
        .spawn_grants
        .entry(config.component_role.clone())
        .or_default()
        .insert(
            role,
            ComponentSpawnGrantConfig {
                maximum_instances_per_parent,
            },
        );
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
"#,
    )
    .expect("Component Spec should parse");
    let child = &config.children[&CanisterRole::from("user_shard")];

    assert_eq!(config.initial_cycles.to_u128(), 5 * TC);
    assert_eq!(config.cycles_funding.max_per_request.to_u128(), 5 * TC);
    assert_eq!(
        config.limits.maximum_descendants,
        DEFAULT_COMPONENT_MAXIMUM_DESCENDANTS
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
}

#[test]
fn potential_child_roles_and_peer_grants_parse_as_bounded_explicit_policy() {
    let config: ComponentSpecConfig = toml::from_str(
        r#"
component_role = "project_hub"
maximum_instances = 1

[provisions.project_instance]
maximum_instances_per_requester_per_root = 100

[children.project_instance]
kind = "instance"

[children.project_ledger]
kind = "instance"

[children.project_machine]
kind = "instance"

[spawn_grants.project_hub.project_instance]
maximum_instances_per_parent = 100

[spawn_grants.project_instance.project_ledger]
maximum_instances_per_parent = 1

[spawn_grants.project_instance.project_machine]
maximum_instances_per_parent = 1
"#,
    )
    .expect("potential child roles and peer grant should parse");

    assert_eq!(
        config.provisions[&component_spec_id("project_instance")]
            .maximum_instances_per_requester_per_root,
        100,
    );
    assert_eq!(config.children.len(), 3);
    assert_eq!(config.spawn_grants.len(), 2);
    assert_eq!(
        config.spawn_grants[&CanisterRole::from("project_hub")]
            [&CanisterRole::from("project_instance")]
            .maximum_instances_per_parent,
        100,
    );
}

fn component_spec_id(value: &str) -> ComponentSpecId {
    value.parse().expect("valid Component Spec ID")
}

#[test]
fn component_aggregate_limits_are_positive_and_bound_declared_children() {
    let mut config = component_spec("hub");
    admit_child(&mut config, "worker", ComponentChildKind::Replica, 4);
    config.limits.maximum_descendants = 3;
    config
        .validate()
        .expect("aggregate descendant cap may be lower than independent spawn ceilings");

    config.limits.maximum_descendants = 0;
    config
        .validate()
        .expect_err("a Component with children needs positive aggregate capacity");

    config.limits.maximum_descendants = 3;
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

[children.worker.children.nested]
kind = "shard"
"#,
    )
    .expect_err("Component child-role declarations stay flat even though runtime trees do not");
}

#[test]
fn child_kind_rejects_root_service_and_component() {
    for kind in ["root", "service", "component"] {
        let source = format!("kind = \"{kind}\"\n");
        toml::from_str::<ComponentChildConfig>(&source)
            .expect_err("only managed child lifecycle kinds should parse");
    }
}

#[test]
fn component_and_spawn_grant_ceilings_are_positive() {
    let mut config = component_spec("hub");
    config.maximum_instances = 0;
    config
        .validate()
        .expect_err("zero Component ceiling must reject");

    let mut config = component_spec("hub");
    admit_child(&mut config, "worker", ComponentChildKind::Replica, 0);
    config
        .validate()
        .expect_err("zero spawn-grant ceiling must reject");
}

#[test]
fn singleton_spawn_grant_ceiling_is_exactly_one() {
    let mut config = component_spec("hub");
    admit_child(&mut config, "settings", ComponentChildKind::Singleton, 2);

    config
        .validate()
        .expect_err("singleton multiplicity above one must reject");
}

#[test]
fn spawn_grants_require_declared_roles_complete_coverage_and_a_finite_bound() {
    let mut missing = component_spec("hub");
    missing.children.insert(
        CanisterRole::from("worker"),
        child(ComponentChildKind::Replica),
    );
    missing
        .validate()
        .expect_err("every potential child role needs an incoming spawn grant");

    let mut unknown_parent = component_spec("hub");
    admit_child(
        &mut unknown_parent,
        "worker",
        ComponentChildKind::Replica,
        1,
    );
    let worker_grant = unknown_parent
        .spawn_grants
        .remove(&CanisterRole::from("hub"))
        .expect("hub grants");
    unknown_parent
        .spawn_grants
        .insert(CanisterRole::from("missing"), worker_grant);
    unknown_parent
        .validate()
        .expect_err("spawn-grant parent must be a declared role");

    let mut excessive = component_spec("hub");
    excessive.children.insert(
        CanisterRole::from("worker"),
        child(ComponentChildKind::Replica),
    );
    let grants = (0..=MAX_COMPONENT_SPAWN_GRANTS)
        .map(|ordinal| {
            (
                CanisterRole::owned(format!("worker_{ordinal}")),
                ComponentSpawnGrantConfig {
                    maximum_instances_per_parent: 1,
                },
            )
        })
        .collect();
    excessive
        .spawn_grants
        .insert(CanisterRole::from("hub"), grants);
    excessive
        .validate()
        .expect_err("spawn-grant count above the bound must reject");
}

#[test]
fn descendant_roles_may_own_placement_pools_through_their_own_spawn_grants() {
    let mut config = component_spec("project_hub");
    admit_child(
        &mut config,
        "project_instance",
        ComponentChildKind::Instance,
        10_000,
    );
    admit_child(
        &mut config,
        "project_machine",
        ComponentChildKind::Replica,
        4,
    );
    config
        .spawn_grants
        .entry(CanisterRole::from("project_instance"))
        .or_default()
        .insert(
            CanisterRole::from("project_machine"),
            ComponentSpawnGrantConfig {
                maximum_instances_per_parent: 4,
            },
        );
    config
        .children
        .get_mut("project_instance")
        .expect("project instance")
        .scaling = Some(ScalingConfig {
        pools: BTreeMap::from([(
            "machines".to_string(),
            ScalePool {
                canister_role: CanisterRole::from("project_machine"),
                policy: ScalePoolPolicy {
                    initial_workers: 1,
                    min_workers: 1,
                    max_workers: 4,
                },
            },
        )]),
    });

    config
        .validate()
        .expect("a descendant manager may use a placement pool");
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
    for role in [
        CanisterRole::FLEET_COORDINATOR,
        CanisterRole::ROOT,
        CanisterRole::WASM_STORE,
    ] {
        component_spec(role.as_str())
            .validate()
            .expect_err("infrastructure cannot be a Component role");
    }

    let mut config = component_spec("hub");
    admit_child(&mut config, "hub", ComponentChildKind::Replica, 1);
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
    admit_child(&mut config, "worker", ComponentChildKind::Replica, 2);
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
    admit_child(&mut config, "worker", ComponentChildKind::Replica, 4);
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
        .expect_err("pool ceiling above its spawn-grant ceiling must reject");
}

#[test]
fn sharding_targets_only_a_declared_shard_child_within_its_ceiling() {
    let mut config = component_spec("hub");
    admit_child(&mut config, "shard", ComponentChildKind::Shard, 8);
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
fn index_targets_only_a_declared_instance_child() {
    let mut config = component_spec("hub");
    admit_child(&mut config, "instance", ComponentChildKind::Instance, 100);
    config.index = Some(IndexConfig {
        pools: BTreeMap::from([(
            "projects".to_string(),
            IndexPool {
                canister_role: CanisterRole::from("instance"),
                key_name: "project".to_string(),
            },
        )]),
    });
    config.validate().expect("valid index edge should pass");

    config
        .index
        .as_mut()
        .expect("index")
        .pools
        .get_mut("projects")
        .expect("projects")
        .key_name = String::new();
    config.validate().expect_err("empty index key must reject");
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
    let mut worker = child(ComponentChildKind::Replica);
    worker.topup = Some(TopupPolicy {
        threshold: Cycles::new(10),
        amount: Cycles::new(6),
    });
    config.children.insert(CanisterRole::from("worker"), worker);
    config.spawn_grants.insert(
        CanisterRole::from("hub"),
        BTreeMap::from([(
            CanisterRole::from("worker"),
            ComponentSpawnGrantConfig {
                maximum_instances_per_parent: 1,
            },
        )]),
    );
    config
        .validate()
        .expect_err("child topup above half its threshold must reject");
}

#[test]
fn structural_role_lookup_returns_only_the_component_and_its_children() {
    let mut config = component_spec("hub");
    config.children.insert(
        CanisterRole::from("worker"),
        child(ComponentChildKind::Replica),
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
        child(ComponentChildKind::Replica),
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
    let child = child(ComponentChildKind::Shard).canister_config();

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
