use super::super::model::{
    ConfiguredPoolExpectation, DEFAULT_INITIAL_CYCLES, LOCAL_ROOT_MIN_READY_CYCLES,
};
use canic_core::{
    bootstrap::compiled::{ComponentSpecConfig, ConfigModel},
    ids::CanisterRole,
};
use std::collections::{BTreeMap, BTreeSet};

///
/// ComponentRoleScope
///
#[derive(Clone, Copy)]
enum ComponentRoleScope {
    Release,
    Deployable,
}

impl ComponentRoleScope {
    const fn includes_root(self) -> bool {
        matches!(self, Self::Deployable)
    }
}

// Estimate local root create funding from all configured Component obligations.
pub(in crate::release_set) fn configured_local_root_create_cycles_from_config(
    config: &ConfigModel,
) -> Option<u128> {
    if !config.roles.contains_key(&CanisterRole::ROOT) {
        return None;
    }

    let mut cycles = DEFAULT_INITIAL_CYCLES;
    for component_spec in config.component_specs.values() {
        cycles = cycles.saturating_add(component_spec.initial_cycles.to_u128());
    }

    Some(cycles.saturating_add(LOCAL_ROOT_MIN_READY_CYCLES))
}

// Enumerate configured pool identities across all Component Specs.
pub(in crate::release_set) fn configured_pool_expectations_from_config(
    config: &ConfigModel,
) -> Vec<ConfiguredPoolExpectation> {
    let mut pools = BTreeMap::<String, ConfiguredPoolExpectation>::new();

    for (component_spec_id, component_spec) in &config.component_specs {
        for (parent_role, canister) in component_spec.canister_configs() {
            if let Some(scaling) = &canister.scaling {
                for (pool_name, pool) in &scaling.pools {
                    pools.insert(
                        format!(
                            "{component_spec_id}:{parent_role}:scaling:{pool_name}:{}",
                            pool.canister_role.as_str()
                        ),
                        ConfiguredPoolExpectation {
                            pool: pool_name.clone(),
                            canister_role: pool.canister_role.as_str().to_string(),
                        },
                    );
                }
            }
            if let Some(sharding) = &canister.sharding {
                for (pool_name, pool) in &sharding.pools {
                    pools.insert(
                        format!(
                            "{component_spec_id}:{parent_role}:sharding:{pool_name}:{}",
                            pool.canister_role.as_str()
                        ),
                        ConfiguredPoolExpectation {
                            pool: pool_name.clone(),
                            canister_role: pool.canister_role.as_str().to_string(),
                        },
                    );
                }
            }
            if let Some(index) = &canister.index {
                for (pool_name, pool) in &index.pools {
                    pools.insert(
                        format!(
                            "{component_spec_id}:{parent_role}:index:{pool_name}:{}",
                            pool.canister_role.as_str()
                        ),
                        ConfiguredPoolExpectation {
                            pool: pool_name.clone(),
                            canister_role: pool.canister_role.as_str().to_string(),
                        },
                    );
                }
            }
        }
    }

    pools.into_values().collect()
}

// Project ordinary release members from one already-validated configuration snapshot.
pub fn configured_release_roles_from_config(config: &ConfigModel) -> Vec<String> {
    configured_component_roles(config, ComponentRoleScope::Release)
}

// Enumerate deployable roles across all Component Specs except implicit Wasm stores.
pub(in crate::release_set) fn configured_deployable_roles_from_config(
    config: &ConfigModel,
) -> Vec<String> {
    configured_component_roles(config, ComponentRoleScope::Deployable)
}

// Enumerate roles expected after the configured Components have bootstrapped.
pub(in crate::release_set) fn configured_bootstrap_roles_from_config(
    config: &ConfigModel,
) -> Vec<String> {
    let mut roles = BTreeSet::<String>::new();
    if config.roles.contains_key(&CanisterRole::ROOT) {
        roles.insert(CanisterRole::ROOT.as_str().to_string());
    }
    for component_spec in config.component_specs.values() {
        roles.extend(
            component_spec
                .auto_create_roles()
                .iter()
                .map(|role| role.as_str().to_string()),
        );

        for role in component_spec.auto_create_roles() {
            let Some(canister) = component_spec.get_canister(&role) else {
                continue;
            };

            if let Some(sharding) = &canister.sharding {
                for pool in sharding.pools.values() {
                    if pool.policy.initial_shards > 0 {
                        roles.insert(pool.canister_role.as_str().to_string());
                    }
                }
            }

            if let Some(scaling) = &canister.scaling {
                for pool in scaling.pools.values() {
                    if pool.policy.initial_workers > 0 {
                        roles.insert(pool.canister_role.as_str().to_string());
                    }
                }
            }
        }
    }

    sort_component_roles(roles.into_iter().collect())
}

fn configured_component_roles(config: &ConfigModel, scope: ComponentRoleScope) -> Vec<String> {
    let roles = if scope.includes_root() {
        config.deployable_roles()
    } else {
        config
            .component_specs
            .values()
            .flat_map(ComponentSpecConfig::roles)
            .cloned()
            .collect::<BTreeSet<_>>()
    }
    .into_iter()
    .map(|role| role.as_str().to_string())
    .collect::<Vec<_>>();

    sort_component_roles(roles)
}

// Sort display/build roles deterministically, keeping `root` first when present.
fn sort_component_roles(mut roles: Vec<String>) -> Vec<String> {
    roles.sort_by(|left, right| {
        match (
            left == CanisterRole::ROOT.as_str(),
            right == CanisterRole::ROOT.as_str(),
        ) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => left.cmp(right),
        }
    });
    roles
}
