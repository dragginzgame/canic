use super::{
    ListCommandError,
    options::{ListOptions, ListSource},
    render::ConfigRoleRow,
};
use canic_backup::discovery::RegistryEntry;
use canic_host::{
    install_root::discover_current_canic_config_choices,
    release_set::{
        configured_fleet_roles, configured_role_auto_create, configured_role_capabilities,
        configured_role_details, configured_role_kinds, configured_role_metrics_profiles,
        configured_role_topups, matching_fleet_config_paths,
    },
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Display,
    path::PathBuf,
};

pub(super) fn load_config_role_rows(
    options: &ListOptions,
) -> Result<Vec<ConfigRoleRow>, ListCommandError> {
    let config_path = selected_config_path(options)?;
    let roles = load_config_value(|| configured_fleet_roles(&config_path))?;
    let kinds = load_config_value(|| configured_role_kinds(&config_path))?;
    let capabilities = load_config_value(|| configured_role_capabilities(&config_path))?;
    let auto_create = load_config_value(|| configured_role_auto_create(&config_path))?;
    let topups = load_config_value(|| configured_role_topups(&config_path))?;
    let metrics = load_config_value(|| configured_role_metrics_profiles(&config_path))?;
    let details = if options.verbose {
        load_config_value(|| configured_role_details(&config_path))?
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

fn load_config_value<T, E>(load: impl FnOnce() -> Result<T, E>) -> Result<T, ListCommandError>
where
    E: Display,
{
    load().map_err(config_error)
}

fn config_error(error: impl Display) -> ListCommandError {
    ListCommandError::InstallState(error.to_string())
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
    let Ok(expected) = configured_fleet_roles(&config_path) else {
        return Vec::new();
    };
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
    let fleet = &options.fleet;
    let choices = load_config_value(discover_current_canic_config_choices)?;
    let matches = matching_fleet_config_paths(&choices, fleet);

    match matches.as_slice() {
        [] => Err(ListCommandError::UnknownFleet(fleet.clone())),
        [path] => Ok(path.clone()),
        _ => Err(ListCommandError::InstallState(format!(
            "multiple configs declare fleet {fleet}"
        ))),
    }
}
