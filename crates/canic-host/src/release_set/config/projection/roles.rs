use super::super::model::ConfiguredRoleLifecycle;
use super::labels::metrics_profile_label;
use crate::format::cycles_tc;
use canic_core::{bootstrap::parse_config_model, ids::CanisterRole};
use std::collections::{BTreeMap, BTreeSet};

// Enumerate configured role kinds from raw config source.
pub(in crate::release_set) fn configured_role_kinds_from_source(
    config_source: &str,
) -> Result<BTreeMap<String, String>, Box<dyn std::error::Error>> {
    let config = parse_config_model(config_source).map_err(|err| err.to_string())?;
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

    Ok(kinds)
}

// Enumerate declared role lifecycle state from raw config source.
pub(in crate::release_set) fn configured_role_lifecycle_from_source(
    config_source: &str,
) -> Result<Vec<ConfiguredRoleLifecycle>, Box<dyn std::error::Error>> {
    let config = parse_config_model(config_source).map_err(|err| err.to_string())?;
    let fleet = config
        .fleet_name()
        .ok_or_else(|| "missing required [fleet].name in canic.toml".to_string())?
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

    Ok(config
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
        .collect())
}

// Enumerate derived auto-created service roles from raw config source.
pub(in crate::release_set) fn configured_role_auto_create_from_source(
    config_source: &str,
) -> Result<BTreeSet<String>, Box<dyn std::error::Error>> {
    let config = parse_config_model(config_source).map_err(|err| err.to_string())?;
    let mut auto_create = BTreeSet::<String>::new();

    for subnet in config.subnets.values() {
        auto_create.extend(
            subnet
                .auto_create_roles()
                .iter()
                .map(|role| role.as_str().to_string()),
        );
    }

    Ok(auto_create)
}

// Enumerate configured top-up policy summaries from raw config source.
pub(in crate::release_set) fn configured_role_topups_from_source(
    config_source: &str,
) -> Result<BTreeMap<String, String>, Box<dyn std::error::Error>> {
    let config = parse_config_model(config_source).map_err(|err| err.to_string())?;
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

    Ok(topups)
}

// Enumerate resolved metrics profiles from raw config source.
pub(in crate::release_set) fn configured_role_metrics_profiles_from_source(
    config_source: &str,
) -> Result<BTreeMap<String, String>, Box<dyn std::error::Error>> {
    let config = parse_config_model(config_source).map_err(|err| err.to_string())?;
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

    Ok(profiles)
}
