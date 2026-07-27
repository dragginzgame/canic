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

    if config.roles.contains_key(&CanisterRole::ROOT) {
        kinds.insert(CanisterRole::ROOT.as_str().to_string(), "root".to_string());
    }

    for component_spec in config.component_specs.values() {
        merge_role_label(
            &mut kinds,
            component_spec.component_role.as_str().to_string(),
            "component".to_string(),
        );
        for (role, child) in &component_spec.children {
            merge_role_label(
                &mut kinds,
                role.as_str().to_string(),
                child.kind.to_string(),
            );
        }
    }

    kinds
}

fn merge_role_label(labels: &mut BTreeMap<String, String>, role: String, label: String) {
    match labels.get(&role) {
        Some(existing) if existing != &label => {
            labels.insert(role, "mixed".to_string());
        }
        Some(_) => {}
        None => {
            labels.insert(role, label);
        }
    }
}

// Enumerate declared role lifecycle state from one validated snapshot.
pub(in crate::release_set) fn configured_role_lifecycle_from_config(
    config: &ConfigModel,
) -> Vec<ConfiguredRoleLifecycle> {
    let app = config.app_id().to_string();
    let attached_roles = config.attached_roles();
    let mut topology = BTreeMap::<CanisterRole, Vec<String>>::new();

    if config.roles.contains_key(&CanisterRole::ROOT) {
        topology.insert(CanisterRole::ROOT, vec!["fleet-subnet-root".to_string()]);
    }

    for (component_spec_id, component_spec) in &config.component_specs {
        let component_role = &component_spec.component_role;
        topology
            .entry(component_role.clone())
            .or_default()
            .push(format!("{component_spec_id}/{component_role}"));
        for child_role in component_spec.children.keys() {
            topology
                .entry(child_role.clone())
                .or_default()
                .push(format!(
                    "{component_spec_id}/{component_role}/children/{child_role}"
                ));
        }

        for (parent_role, canister) in component_spec.canister_configs() {
            if let Some(scaling) = &canister.scaling {
                for (pool, scale_pool) in &scaling.pools {
                    topology
                        .entry(scale_pool.canister_role.clone())
                        .or_default()
                        .push(format!("{component_spec_id}/{parent_role}/scaling/{pool}"));
                }
            }

            if let Some(sharding) = &canister.sharding {
                for (pool, shard_pool) in &sharding.pools {
                    topology
                        .entry(shard_pool.canister_role.clone())
                        .or_default()
                        .push(format!("{component_spec_id}/{parent_role}/sharding/{pool}"));
                }
            }

            if let Some(index) = &canister.index {
                for (pool, index_pool) in &index.pools {
                    topology
                        .entry(index_pool.canister_role.clone())
                        .or_default()
                        .push(format!("{component_spec_id}/{parent_role}/index/{pool}"));
                }
            }
        }
    }

    config
        .roles
        .iter()
        .map(|(role, declaration)| {
            let role_name = role.as_str().to_string();
            let attached = declaration.kind
                == canic_core::bootstrap::compiled::RoleDeclarationKind::Root
                || attached_roles.contains(role);
            ConfiguredRoleLifecycle {
                app: app.clone(),
                display: format!("{app}.{role}"),
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

// Enumerate auto-created Component roles from one validated snapshot.
pub(in crate::release_set) fn configured_role_auto_create_from_config(
    config: &ConfigModel,
) -> BTreeSet<String> {
    let mut auto_create = BTreeSet::<String>::new();

    for component_spec in config.component_specs.values() {
        auto_create.extend(
            component_spec
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

    for component_spec in config.component_specs.values() {
        for (role, canister) in component_spec.canister_configs() {
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

    if config.roles.contains_key(&CanisterRole::ROOT) {
        profiles.insert(CanisterRole::ROOT.as_str().to_string(), "root".to_string());
    }

    for component_spec in config.component_specs.values() {
        for (role, canister) in component_spec.canister_configs() {
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
