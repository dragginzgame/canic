use super::{
    ListCommandError,
    options::{ListOptions, ListSource},
    render::ConfigRoleRow,
};
use canic_core::ids::AppId;
use canic_host::{
    config_discovery::{discover_current_canic_config_choices, select_discovered_app_config_path},
    fleet_ensure::CurrentFleetResolution,
    registry::RegistryEntry,
    release_set::AppConfigSnapshot,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

pub(super) fn load_config_role_rows(
    options: &ListOptions,
) -> Result<Vec<ConfigRoleRow>, ListCommandError> {
    let config_path = selected_config_path(options)?;
    let config = AppConfigSnapshot::load(&config_path)?;
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
    fleet: &CurrentFleetResolution,
) -> Result<Vec<String>, ListCommandError> {
    if !matches!(options.source, ListSource::FleetInventory) || options.subtree.is_some() {
        return Ok(Vec::new());
    }

    let app = &fleet
        .initial_active_registry(&options.target)?
        .authority
        .binding
        .fleet
        .app;
    let choices = discover_current_canic_config_choices()?;
    missing_config_roles_for_app(app, &fleet.registry.entries, &choices)
}

fn missing_config_roles_for_app(
    app: &AppId,
    registry: &[RegistryEntry],
    choices: &[PathBuf],
) -> Result<Vec<String>, ListCommandError> {
    let config_path = selected_app_config_path(choices, app.as_ref())?;
    let config = AppConfigSnapshot::load(&config_path)?;
    let expected = config.deployable_roles();
    let deployed = registry
        .iter()
        .filter_map(|entry| entry.role.as_deref())
        .collect::<BTreeSet<_>>();
    Ok(expected
        .into_iter()
        .filter(|role| !deployed.contains(role.as_str()))
        .collect())
}

fn selected_config_path(options: &ListOptions) -> Result<PathBuf, ListCommandError> {
    let choices = discover_current_canic_config_choices()?;
    selected_app_config_path(&choices, &options.target)
}

fn selected_app_config_path(choices: &[PathBuf], app: &str) -> Result<PathBuf, ListCommandError> {
    select_discovered_app_config_path(choices, app)?
        .ok_or_else(|| ListCommandError::UnknownApp(app.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic_host::{
        config_discovery::discover_workspace_canic_config_choices, release_set::AppConfigError,
    };
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    const CONFIG: &str = r#"
[app]
name = "demo"

[roles.root]
kind = "root"

[roles.store]
kind = "canister"
package = "store"

[component_specs.storage]
component_role = "store"
maximum_instances = 1
"#;

    struct WorkspaceFixture(PathBuf);

    impl WorkspaceFixture {
        fn new() -> Self {
            static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);
            let root = std::env::temp_dir().join(format!(
                "canic-list-config-{}-{}",
                std::process::id(),
                NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(root.join("apps/demo")).expect("create App config directory");
            Self(root)
        }

        fn write_config(&self, source: &str) -> PathBuf {
            let path = self.0.join("apps/demo/canic.toml");
            fs::write(&path, source).expect("write App config");
            path
        }
    }

    impl Drop for WorkspaceFixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).expect("remove App config fixture");
        }
    }

    #[test]
    fn missing_roles_select_the_bound_app_for_a_differently_named_fleet() {
        let options = ListOptions::parse_info_list(["demo-local".into()])
            .expect("parse differently named Fleet");
        let app = AppId::from("demo");
        assert_ne!(options.target, app.as_ref());
        let fixture = WorkspaceFixture::new();
        fixture.write_config(CONFIG);
        fs::create_dir_all(fixture.0.join("apps/other")).expect("create unrelated App directory");
        fs::write(
            fixture.0.join("apps/other/canic.toml"),
            CONFIG
                .replace("demo", "other")
                .replace("store", "other_store"),
        )
        .expect("write unrelated App config");
        let choices =
            discover_workspace_canic_config_choices(&fixture.0).expect("discover configs");
        let registry = vec![RegistryEntry {
            pid: "aaaaa-aa".to_string(),
            role: Some("root".to_string()),
            parent_pid: None,
            module_hash: None,
            protocol_binding: None,
        }];

        assert_eq!(
            missing_config_roles_for_app(&app, &registry, &choices)
                .expect("compare configured roles"),
            vec!["store".to_string()]
        );
    }

    #[test]
    fn missing_roles_preserve_unknown_app_and_invalid_config_failures() {
        let app = AppId::from("demo");
        std::assert_matches!(
            missing_config_roles_for_app(&app, &[], &[]),
            Err(ListCommandError::UnknownApp(app)) if app == "demo"
        );

        let fixture = WorkspaceFixture::new();
        let path = fixture.write_config(&CONFIG.replace("maximum_instances = 1", "unknown = true"));
        let choices = discover_workspace_canic_config_choices(&fixture.0).expect("discover config");
        std::assert_matches!(
            missing_config_roles_for_app(&app, &[], &choices),
            Err(ListCommandError::AppConfig(AppConfigError::ConfigInvalid {
                path: actual,
                ..
            })) if actual == path
        );
    }
}
