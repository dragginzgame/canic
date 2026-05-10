use super::{
    ListCommandError,
    options::{ListOptions, ListSource},
    read_selected_install_state,
    render::ConfigRoleRow,
};
use canic_backup::discovery::RegistryEntry;
use canic_host::{
    install_root::discover_current_canic_config_choices,
    release_set::{
        config_path as default_config_path, configured_fleet_roles, configured_role_auto_create,
        configured_role_capabilities, configured_role_details, configured_role_kinds,
        configured_role_metrics_profiles, configured_role_topups, matching_fleet_config_paths,
    },
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

pub(super) fn resolve_role_kinds(options: &ListOptions) -> BTreeMap<String, String> {
    role_kind_config_candidates(options)
        .into_iter()
        .find_map(|path| configured_role_kinds(&path).ok())
        .unwrap_or_default()
}

fn role_kind_config_candidates(options: &ListOptions) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Ok(path) = selected_config_path(options) {
        paths.push(path);
    }

    if let Ok(Some(state)) = read_selected_install_state(options) {
        paths.push(PathBuf::from(state.config_path));
    }

    if let Ok(workspace_root) = std::env::current_dir() {
        paths.push(default_config_path(&workspace_root));
    }

    paths
}

pub(super) fn load_config_role_rows(
    options: &ListOptions,
) -> Result<Vec<ConfigRoleRow>, ListCommandError> {
    let config_path = selected_config_path(options)?;
    let roles = configured_fleet_roles(&config_path)
        .map_err(|err| ListCommandError::InstallState(err.to_string()))?;
    let kinds = configured_role_kinds(&config_path)
        .map_err(|err| ListCommandError::InstallState(err.to_string()))?;
    let capabilities = configured_role_capabilities(&config_path)
        .map_err(|err| ListCommandError::InstallState(err.to_string()))?;
    let auto_create = configured_role_auto_create(&config_path)
        .map_err(|err| ListCommandError::InstallState(err.to_string()))?;
    let topups = configured_role_topups(&config_path)
        .map_err(|err| ListCommandError::InstallState(err.to_string()))?;
    let metrics = configured_role_metrics_profiles(&config_path)
        .map_err(|err| ListCommandError::InstallState(err.to_string()))?;
    let details = if options.verbose {
        configured_role_details(&config_path)
            .map_err(|err| ListCommandError::InstallState(err.to_string()))?
    } else {
        BTreeMap::new()
    };
    let anchor = options.anchor.as_deref();
    if let Some(anchor) = anchor
        && !roles.iter().any(|role| role == anchor)
    {
        return Err(ListCommandError::CanisterNotInRegistry(anchor.to_string()));
    }

    Ok(roles
        .into_iter()
        .filter(|role| anchor.is_none_or(|anchor| anchor == role))
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
    if !matches!(options.source, ListSource::RootRegistry) || options.anchor.is_some() {
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
    let choices = discover_current_canic_config_choices()
        .map_err(|err| ListCommandError::InstallState(err.to_string()))?;
    let matches = matching_fleet_config_paths(&choices, fleet);

    match matches.as_slice() {
        [] => Err(ListCommandError::UnknownFleet(fleet.clone())),
        [path] => Ok(path.clone()),
        _ => Err(ListCommandError::InstallState(format!(
            "multiple configs declare fleet {fleet}"
        ))),
    }
}
