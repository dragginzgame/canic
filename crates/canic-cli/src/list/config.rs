use super::{
    ListCommandError,
    options::{ListOptions, ListSource},
    render::ConfigRoleRow,
};
use canic_host::{
    install_root::{discover_current_canic_config_choices, select_discovered_fleet_config_path},
    registry::RegistryEntry,
    release_set::FleetConfigSnapshot,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

pub(super) fn load_config_role_rows(
    options: &ListOptions,
) -> Result<Vec<ConfigRoleRow>, ListCommandError> {
    let config_path = selected_config_path(options)?;
    let config = FleetConfigSnapshot::load(&config_path)?;
    let roles = config.deployable_roles();
    let kinds = config.role_kinds();
    let capabilities = config.role_capabilities()?;
    let auto_create = config.role_auto_create();
    let topups = config.role_topups();
    let metrics = config.role_metrics_profiles();
    let details = if options.verbose {
        config.role_details()
    } else {
        BTreeMap::new()
    };
    Ok(roles
        .into_iter()
        .map(|role| ConfigRoleRow {
            capabilities: capabilities
                .get(&role)
                .filter(|capabilities| !capabilities.is_empty())
                .map_or_else(|| "-".to_string(), |capabilities| capabilities.join(", ")),
            auto_create: auto_create_label(&role, &auto_create),
            topup: topups
                .get(&role)
                .cloned()
                .unwrap_or_else(|| "-".to_string()),
            metrics: metrics
                .get(&role)
                .cloned()
                .unwrap_or_else(|| "-".to_string()),
            details: details.get(&role).cloned().unwrap_or_default(),
            kind: kinds
                .get(&role)
                .cloned()
                .unwrap_or_else(|| "unknown".to_string()),
            role,
        })
        .collect())
}

fn auto_create_label(role: &str, auto_create: &BTreeSet<String>) -> String {
    if role == "root" {
        "-".to_string()
    } else if auto_create.contains(role) {
        "yes".to_string()
    } else {
        "no".to_string()
    }
}

pub(super) fn missing_config_roles(
    options: &ListOptions,
    registry: &[RegistryEntry],
) -> Vec<String> {
    if !matches!(options.source, ListSource::RootRegistry) || options.subtree.is_some() {
        return Vec::new();
    }

    let Ok(config_path) = selected_config_path(options) else {
        return Vec::new();
    };
    let Ok(config) = FleetConfigSnapshot::load(&config_path) else {
        return Vec::new();
    };
    let expected = config.deployable_roles();
    let deployed = registry
        .iter()
        .filter_map(|entry| entry.role.as_deref())
        .collect::<BTreeSet<_>>();
    expected
        .into_iter()
        .filter(|role| !deployed.contains(role.as_str()))
        .collect()
}

pub(super) fn selected_config_path(options: &ListOptions) -> Result<PathBuf, ListCommandError> {
    let fleet = &options.target;
    let choices = discover_current_canic_config_choices()?;
    select_discovered_fleet_config_path(&choices, fleet)?
        .ok_or_else(|| ListCommandError::UnknownFleet(fleet.clone()))
}
