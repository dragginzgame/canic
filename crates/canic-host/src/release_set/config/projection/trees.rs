use super::super::model::{
    ConfiguredPoolExpectation, DEFAULT_INITIAL_CYCLES, LOCAL_ROOT_MIN_READY_CYCLES,
};
use canic_core::{
    bootstrap::compiled::{ConfigModel, TreeSpecConfig},
    ids::CanisterRole,
};
use std::collections::{BTreeMap, BTreeSet};

///
/// TreeRoleScope
///
#[derive(Clone, Copy)]
enum TreeRoleScope {
    Release,
    Deployable,
}

impl TreeRoleScope {
    const fn includes_root(self) -> bool {
        matches!(self, Self::Deployable)
    }
}

// Estimate local root create funding from the initial Tree bootstrap obligations.
pub(in crate::release_set) fn configured_local_root_create_cycles_from_config(
    config: &ConfigModel,
) -> Option<u128> {
    let tree_spec = sole_initial_tree_spec(config)?;

    let mut cycles = tree_spec
        .get_canister(&CanisterRole::WASM_STORE)
        .map_or(DEFAULT_INITIAL_CYCLES, |cfg| cfg.initial_cycles.to_u128());
    for role in tree_spec.auto_create_roles() {
        if let Some(cfg) = tree_spec.get_canister(&role) {
            cycles = cycles.saturating_add(cfg.initial_cycles.to_u128());
        }
    }
    cycles = cycles.saturating_add(
        u128::from(tree_spec.pool.minimum_size).saturating_mul(DEFAULT_INITIAL_CYCLES),
    );

    Some(cycles.saturating_add(LOCAL_ROOT_MIN_READY_CYCLES))
}

// Enumerate configured pool identities across all initial Tree Specs.
pub(in crate::release_set) fn configured_pool_expectations_from_config(
    config: &ConfigModel,
) -> Vec<ConfiguredPoolExpectation> {
    let mut pools = BTreeMap::<String, ConfiguredPoolExpectation>::new();

    for (tree_spec_id, tree_spec) in initial_tree_specs(config) {
        for canister in tree_spec.canisters.values() {
            if let Some(scaling) = &canister.scaling {
                for (pool_name, pool) in &scaling.pools {
                    pools.insert(
                        format!(
                            "{tree_spec_id}:scaling:{pool_name}:{}",
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
                            "{tree_spec_id}:sharding:{pool_name}:{}",
                            pool.canister_role.as_str()
                        ),
                        ConfiguredPoolExpectation {
                            pool: pool_name.clone(),
                            canister_role: pool.canister_role.as_str().to_string(),
                        },
                    );
                }
            }
            if let Some(binding) = &canister.binding {
                for (pool_name, pool) in &binding.pools {
                    pools.insert(
                        format!(
                            "{tree_spec_id}:binding:{pool_name}:{}",
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
    configured_tree_roles(config, TreeRoleScope::Release)
}

// Enumerate deployable roles across all Tree Specs except implicit Wasm stores.
pub(in crate::release_set) fn configured_deployable_roles_from_config(
    config: &ConfigModel,
) -> Vec<String> {
    configured_tree_roles(config, TreeRoleScope::Deployable)
}

// Enumerate roles expected after all configured initial Trees have bootstrapped.
pub(in crate::release_set) fn configured_bootstrap_roles_from_config(
    config: &ConfigModel,
) -> Vec<String> {
    let mut roles = BTreeSet::<String>::new();
    for (_tree_spec_id, tree_spec) in initial_tree_specs(config) {
        roles.insert(CanisterRole::ROOT.as_str().to_string());
        roles.extend(
            tree_spec
                .auto_create_roles()
                .iter()
                .map(|role| role.as_str().to_string()),
        );

        for role in tree_spec.auto_create_roles() {
            let Some(canister) = tree_spec.get_canister(&role) else {
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

    sort_tree_roles(roles.into_iter().collect())
}

fn configured_tree_roles(config: &ConfigModel, scope: TreeRoleScope) -> Vec<String> {
    let roles = config
        .tree_specs
        .values()
        .flat_map(|tree_spec| tree_spec.canisters.keys())
        .filter(|role| !role.is_wasm_store())
        .filter(|role| scope.includes_root() || !role.is_root())
        .map(|role| role.as_str().to_string())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    sort_tree_roles(roles)
}

fn sole_initial_tree_spec(config: &ConfigModel) -> Option<&TreeSpecConfig> {
    let tree_spec_id = config.sole_initial_tree_spec_id()?;
    config.tree_specs.get(tree_spec_id)
}

fn initial_tree_specs(config: &ConfigModel) -> Vec<(&str, &TreeSpecConfig)> {
    config
        .tree_groups
        .values()
        .filter(|group| group.initial_trees > 0)
        .filter_map(|group| {
            config
                .tree_specs
                .get(&group.tree_spec)
                .map(|tree_spec| (group.tree_spec.as_str(), tree_spec))
        })
        .collect()
}

// Sort display/build roles deterministically, keeping `root` first when present.
fn sort_tree_roles(mut roles: Vec<String>) -> Vec<String> {
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
