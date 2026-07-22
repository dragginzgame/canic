use super::super::model::{
    ConfiguredPoolExpectation, DEFAULT_INITIAL_CYCLES, LOCAL_ROOT_MIN_READY_CYCLES,
};
use canic_core::{
    bootstrap::compiled::{ConfigModel, SubnetConfig},
    ids::CanisterRole,
};
use std::collections::{BTreeMap, BTreeSet};

///
/// RootSubnetRoleScope
///
#[derive(Clone, Copy)]
enum RootSubnetRoleScope {
    Release,
    Deployable,
}

impl RootSubnetRoleScope {
    const fn includes_root(self) -> bool {
        matches!(self, Self::Deployable)
    }
}

// Estimate local root create funding from the root subnet bootstrap obligations.
pub(in crate::release_set) fn configured_local_root_create_cycles_from_config(
    config: &ConfigModel,
) -> u128 {
    let subnet = root_subnet(config);

    let mut cycles = subnet
        .get_canister(&CanisterRole::WASM_STORE)
        .map_or(DEFAULT_INITIAL_CYCLES, |cfg| cfg.initial_cycles.to_u128());
    for role in subnet.auto_create_roles() {
        if let Some(cfg) = subnet.get_canister(&role) {
            cycles = cycles.saturating_add(cfg.initial_cycles.to_u128());
        }
    }
    cycles = cycles.saturating_add(
        u128::from(subnet.pool.minimum_size).saturating_mul(DEFAULT_INITIAL_CYCLES),
    );

    cycles.saturating_add(LOCAL_ROOT_MIN_READY_CYCLES)
}

// Enumerate configured pool identities for the single subnet that owns `root`.
pub(in crate::release_set) fn configured_pool_expectations_from_config(
    config: &ConfigModel,
) -> Vec<ConfiguredPoolExpectation> {
    let subnet = root_subnet(config);
    let mut pools = BTreeMap::<String, ConfiguredPoolExpectation>::new();

    for canister in subnet.canisters.values() {
        if let Some(scaling) = &canister.scaling {
            for (pool_name, pool) in &scaling.pools {
                pools.insert(
                    format!("scaling:{pool_name}:{}", pool.canister_role.as_str()),
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
                    format!("sharding:{pool_name}:{}", pool.canister_role.as_str()),
                    ConfiguredPoolExpectation {
                        pool: pool_name.clone(),
                        canister_role: pool.canister_role.as_str().to_string(),
                    },
                );
            }
        }
        if let Some(directory) = &canister.directory {
            for (pool_name, pool) in &directory.pools {
                pools.insert(
                    format!("directory:{pool_name}:{}", pool.canister_role.as_str()),
                    ConfiguredPoolExpectation {
                        pool: pool_name.clone(),
                        canister_role: pool.canister_role.as_str().to_string(),
                    },
                );
            }
        }
    }

    pools.into_values().collect()
}

// Project ordinary release members from one already-validated configuration snapshot.
pub fn configured_release_roles_from_config(config: &ConfigModel) -> Vec<String> {
    configured_root_subnet_roles(config, RootSubnetRoleScope::Release)
}

// Enumerate deployable roles for the single subnet that owns `root`, except the
// implicit `wasm_store` bootstrap canister.
pub(in crate::release_set) fn configured_deployable_roles_from_config(
    config: &ConfigModel,
) -> Vec<String> {
    configured_root_subnet_roles(config, RootSubnetRoleScope::Deployable)
}

// Enumerate roles expected to be present once root bootstrap has completed.
pub(in crate::release_set) fn configured_bootstrap_roles_from_config(
    config: &ConfigModel,
) -> Vec<String> {
    let subnet = root_subnet(config);

    let mut roles = BTreeSet::<String>::new();
    roles.insert(CanisterRole::ROOT.as_str().to_string());
    roles.extend(
        subnet
            .auto_create_roles()
            .iter()
            .map(|role| role.as_str().to_string()),
    );

    for role in subnet.auto_create_roles() {
        let Some(canister) = subnet.get_canister(&role) else {
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

    sort_root_subnet_roles(roles.into_iter().collect())
}

fn configured_root_subnet_roles(config: &ConfigModel, scope: RootSubnetRoleScope) -> Vec<String> {
    let subnet = root_subnet(config);
    let root_subnet_roles = subnet
        .canisters
        .keys()
        .filter(|role| !role.is_wasm_store())
        .filter(|role| scope.includes_root() || !role.is_root())
        .map(|role| role.as_str().to_string())
        .collect::<Vec<_>>();

    sort_root_subnet_roles(root_subnet_roles)
}

fn root_subnet(config: &ConfigModel) -> &SubnetConfig {
    config
        .subnets
        .values()
        .find(|subnet| subnet.canisters.keys().any(CanisterRole::is_root))
        .expect("validated config must contain exactly one root subnet")
}

// Sort display/build roles deterministically, keeping `root` first when present.
fn sort_root_subnet_roles(mut roles: Vec<String>) -> Vec<String> {
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
