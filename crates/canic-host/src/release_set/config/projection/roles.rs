use super::super::model::ConfiguredRoleLifecycle;
use super::labels::metrics_profile_label;
use crate::format::cycles_tc;
use canic_core::bootstrap::compiled::ConfigModel;
use canic_core::ids::CanisterRole;
use std::collections::{BTreeMap, BTreeSet};

// Enumerate configured role kinds from one validated snapshot.
pub(in crate::release_set) fn configured_role_kinds_from_config(
    config: &ConfigModel,
) -> BTreeMap<String, String> {
    let mut kinds = BTreeMap::<String, String>::new();

    for subnet in config.subnets.values() {
        for (role, canister) in &subnet.canisters {
            let role = role.as_str().to_string();
            let kind = canister.kind.to_string();
            match kinds.get(&role) {
                Some(existing) if existing != &kind => {
                    kinds.insert(role, "mixed".to_string());
                }
                Some(_) => {}
                None => {
                    kinds.insert(role, kind);
                }
            }
        }
    }

    kinds
}

// Enumerate declared role lifecycle state from one validated snapshot.
pub(in crate::release_set) fn configured_role_lifecycle_from_config(
    config: &ConfigModel,
) -> Vec<ConfiguredRoleLifecycle> {
    let fleet = config
        .fleet_name()
        .expect("validated config must declare [fleet].name")
        .to_string();
    let attached_roles = config.attached_roles();
    let mut topology = BTreeMap::<CanisterRole, Vec<String>>::new();

    for (subnet_role, subnet) in &config.subnets {
        for (role, canister) in &subnet.canisters {
            topology
                .entry(role.clone())
                .or_default()
                .push(format!("{subnet_role}/{role}"));

            if let Some(scaling) = &canister.scaling {
                for (pool, scale_pool) in &scaling.pools {
                    topology
                        .entry(scale_pool.canister_role.clone())
                        .or_default()
                        .push(format!("{subnet_role}/{role}/scaling/{pool}"));
                }
            }

            if let Some(sharding) = &canister.sharding {
                for (pool, shard_pool) in &sharding.pools {
                    topology
                        .entry(shard_pool.canister_role.clone())
                        .or_default()
                        .push(format!("{subnet_role}/{role}/sharding/{pool}"));
                }
            }

            if let Some(directory) = &canister.directory {
                for (pool, directory_pool) in &directory.pools {
                    topology
                        .entry(directory_pool.canister_role.clone())
                        .or_default()
                        .push(format!("{subnet_role}/{role}/directory/{pool}"));
                }
            }
        }
    }

    config
        .roles
        .iter()
        .map(|(role, declaration)| {
            let role_name = role.as_str().to_string();
            let attached = attached_roles.contains(role);
            ConfiguredRoleLifecycle {
                fleet: fleet.clone(),
                display: format!("{fleet}.{role}"),
                role: role_name,
                declaration_kind: if role.is_root() { "root" } else { "canister" }.to_string(),
                package: declaration.package.clone(),
                attached,
                state: if attached { "attached" } else { "declared" }.to_string(),
                topology: topology.get(role).map(|labels| labels.join(",")),
            }
        })
        .collect()
}

// Enumerate derived auto-created service roles from one validated snapshot.
pub(in crate::release_set) fn configured_role_auto_create_from_config(
    config: &ConfigModel,
) -> BTreeSet<String> {
    let mut auto_create = BTreeSet::<String>::new();

    for subnet in config.subnets.values() {
        auto_create.extend(
            subnet
                .auto_create_roles()
                .iter()
                .map(|role| role.as_str().to_string()),
        );
    }

    auto_create
}

// Enumerate configured top-up policy summaries from one validated snapshot.
pub(in crate::release_set) fn configured_role_topups_from_config(
    config: &ConfigModel,
) -> BTreeMap<String, String> {
    let mut topups = BTreeMap::<String, String>::new();

    for subnet in config.subnets.values() {
        for (role, canister) in &subnet.canisters {
            if let Some(policy) = &canister.topup {
                topups.insert(
                    role.as_str().to_string(),
                    format!(
                        "{} @ {}",
                        cycles_tc(policy.amount.to_u128()),
                        cycles_tc(policy.threshold.to_u128())
                    ),
                );
            }
        }
    }

    topups
}

// Enumerate resolved metrics profiles from one validated snapshot.
pub(in crate::release_set) fn configured_role_metrics_profiles_from_config(
    config: &ConfigModel,
) -> BTreeMap<String, String> {
    let mut profiles = BTreeMap::<String, String>::new();

    for subnet in config.subnets.values() {
        for (role, canister) in &subnet.canisters {
            let role_name = role.as_str().to_string();
            let profile = metrics_profile_label(canister.resolved_metrics_profile(role));
            match profiles.get(&role_name) {
                Some(existing) if existing != profile => {
                    profiles.insert(role_name, "mixed".to_string());
                }
                Some(_) => {}
                None => {
                    profiles.insert(role_name, profile.to_string());
                }
            }
        }
    }

    profiles
}
