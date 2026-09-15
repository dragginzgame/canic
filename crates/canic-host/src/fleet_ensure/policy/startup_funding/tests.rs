use super::*;
use canic_core::control_plane_support::config::ComponentDeploymentConfiguration;

const T: u128 = 1_000_000_000_000;

#[test]
fn startup_preserves_deployment_reserve_below_the_automatic_funding_threshold() {
    use canic_core::control_plane_support::policy::deployment::MINIMUM_DEPLOYMENT_RESERVE_CYCLES as RESERVE;
    assert_eq!(minimum_root_cycles(0, 2 * T).unwrap(), RESERVE + 2 * T);
    assert_eq!(
        minimum_root_cycles(RESERVE, 2 * T).unwrap(),
        RESERVE + 1 + 2 * T
    );
    assert_eq!(minimum_root_cycles(10 * T, 2 * T).unwrap(), 12 * T + 1);
    for (threshold, grants) in [(u128::MAX, 0), (0, u128::MAX)] {
        assert!(matches!(
            minimum_root_cycles(threshold, grants),
            Err(EnsurePolicyError::ArithmeticOverflow { .. })
        ));
    }
}

#[test]
fn startup_grants_include_every_increment_and_threshold_equality() {
    assert_eq!(
        grant_demand(49 * T / 10, 0, Some(10 * T), 5 * T, 100 * T).unwrap(),
        GrantDemand {
            count: 2,
            cycles: 10 * T,
            unfunded: 0
        }
    );
    assert_eq!(
        grant_demand(5 * T, 0, Some(10 * T), 5 * T, 100 * T).unwrap(),
        GrantDemand {
            count: 2,
            cycles: 10 * T,
            unfunded: 0
        }
    );
    assert_eq!(
        grant_demand(10 * T, 0, Some(10 * T), 5 * T, 100 * T).unwrap(),
        GrantDemand {
            count: 1,
            cycles: 5 * T,
            unfunded: 0
        }
    );
    assert_eq!(
        grant_demand(10 * T + 1, 0, Some(10 * T), 5 * T, 100 * T).unwrap(),
        GrantDemand {
            count: 0,
            cycles: 0,
            unfunded: 0
        }
    );
}

#[test]
fn startup_grants_preserve_lifetime_caps_and_disabled_parent_shortfalls() {
    assert_eq!(
        grant_demand(49 * T / 10, 0, Some(10 * T), 2 * T, 5 * T).unwrap(),
        GrantDemand {
            count: 3,
            cycles: 5 * T,
            unfunded: T / 10 + 1
        }
    );
    assert_eq!(
        grant_demand(5 * T, 10 * T, None, 0, 100 * T).unwrap(),
        GrantDemand {
            count: 0,
            cycles: 0,
            unfunded: 5 * T
        }
    );
    assert_eq!(
        grant_demand(5 * T, 0, None, 0, 100 * T).unwrap(),
        GrantDemand {
            count: 0,
            cycles: 0,
            unfunded: 0
        }
    );
    assert!(matches!(
        grant_demand(0, 0, Some(u128::MAX), 1, u128::MAX),
        Err(EnsurePolicyError::ArithmeticOverflow { .. })
    ));
}

pub(super) fn hub_config() -> ConfigModel {
    toml::from_str(
        r#"
[app]
name = "startup"
[roles.hub]
kind = "canister"
package = "hub"
[roles.shard]
kind = "canister"
package = "shard"
[component_specs.hubs]
component_role = "hub"
maximum_instances = 1
initial_cycles = "1.9T"
[component_specs.hubs.topup]
threshold = "10T"
amount = "5T"
[component_specs.hubs.sharding.pools.shards]
canister_role = "shard"
policy.capacity = 100
policy.initial_shards = 3
policy.max_shards = 3
[component_specs.hubs.children.shard]
kind = "shard"
initial_cycles = "1.9T"
[component_specs.hubs.children.shard.topup]
threshold = "10T"
amount = "5T"
[component_specs.hubs.spawn_grants.hub.shard]
maximum_instances_per_parent = 3
[component_specs.hubs.limits.cycles_funding]
window_secs = 3600
maximum_cycles = "25T"
[component_groups.app.components.hub]
component_spec = "hubs"
[component_group_deployments.app]
component_group = "app"
initial_placements = 1
maximum_placements = 1
placement.maximum_per_root = 1
placement.minimum_distinct_roots = 1
"#,
    )
    .unwrap()
}

#[test]
fn startup_tree_charges_root_only_for_direct_child_and_exposes_window_limits() {
    let config = hub_config();
    let compiled = ComponentDeploymentConfiguration::compile(&config).unwrap();
    let spec = &compiled.component_topology.component_specs[0];
    let forecast = component(&config, spec, 49 * T / 10, "app", 0).unwrap();
    // Three descendants need 10T each. Their Hub needs 40T in total, including
    // those outgoing 30T; the Root does not need 40T + 30T.
    assert_eq!(forecast.descendant_grants_cycles, 30 * T);
    assert_eq!(forecast.root_grant_cycles, 40 * T);
    assert!(forecast.exceeds_window_budget);
    let hub = forecast
        .roles
        .iter()
        .find(|role| role.role.as_str() == "hub")
        .unwrap();
    let shard = forecast
        .roles
        .iter()
        .find(|role| role.role.as_str() == "shard")
        .unwrap();
    assert_eq!(hub.grants_per_instance, 8);
    assert_eq!(shard.instances, 3);
    assert_eq!(shard.grants_per_instance, 2);
    assert_eq!(hub.unfunded_per_instance_cycles, 0);

    let mut clamped = config;
    clamped
        .component_specs
        .get_mut("hubs")
        .unwrap()
        .cycles_funding
        .max_per_request = canic_core::cdk::types::Cycles::new(2 * T);
    let clamped = component(&clamped, spec, 49 * T / 10, "app", 0).unwrap();
    assert_eq!(clamped.root_grant_cycles, 36 * T);
    assert_eq!(
        clamped
            .roles
            .iter()
            .find(|role| role.role.as_str() == "hub")
            .unwrap()
            .grants_per_instance,
        18
    );
}

#[test]
fn startup_counts_same_role_under_distinct_parents_and_rejects_initial_cycles() {
    let mut config = hub_config();
    let mut compiled = ComponentDeploymentConfiguration::compile(&config).unwrap();
    let spec = &mut compiled.component_topology.component_specs[0];
    let mut branch = spec.children[0].clone();
    branch.role = "branch".into();
    spec.children.push(branch);
    let mut hub_branch = spec.spawn_grants[0].clone();
    hub_branch.child_role = "branch".into();
    hub_branch.initial_instances_per_parent = 2;
    let mut branch_shard = spec.spawn_grants[0].clone();
    branch_shard.parent_role = "branch".into();
    branch_shard.initial_instances_per_parent = 2;
    spec.spawn_grants.extend([hub_branch, branch_shard]);
    let configured = config.component_specs.get_mut("hubs").unwrap();
    configured.children.insert(
        "branch".into(),
        configured.children.get("shard").unwrap().clone(),
    );
    let forecast = component(&config, spec, 49 * T / 10, "app", 0).unwrap();
    assert_eq!(
        forecast
            .roles
            .iter()
            .find(|role| role.role.as_str() == "shard")
            .unwrap()
            .instances,
        7
    );
    assert_eq!(super::super::initial_role_tree_size(spec).unwrap(), 10);
    assert_eq!(forecast.root_grant_cycles, 100 * T);
    assert_eq!(forecast.descendant_grants_cycles, 130 * T);

    // Positive initial edges must terminate even though later dynamic spawn
    // permissions may legitimately form a cycle.
    let mut recursive = spec.spawn_grants[0].clone();
    recursive.parent_role = "shard".into();
    recursive.child_role = "shard".into();
    recursive.initial_instances_per_parent = 1;
    spec.spawn_grants.push(recursive);
    assert!(matches!(
        initial_role_instances(spec),
        Err(EnsurePolicyError::EstateFundingTopology { .. })
    ));
    spec.spawn_grants
        .last_mut()
        .unwrap()
        .initial_instances_per_parent = 0;
    assert_eq!(super::super::initial_role_tree_size(spec).unwrap(), 10);
}
