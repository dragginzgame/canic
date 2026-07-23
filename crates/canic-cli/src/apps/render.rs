//! Module: apps::render
//! Responsibility: render list and role-management output for `canic app`.
//! Does not own: command dispatch, option parsing, filesystem mutation, or reports.
//! Boundary: deterministic text/table formatting for app listing and role lifecycle commands.

use crate::cli::render::append_dry_run_footer;
use canic_host::{
    release_set::{
        AppConfigSnapshot, AttachedAppRole, ConfiguredRoleLifecycle, DeclaredAppRole,
        RenamedAppRole, display_workspace_path,
    },
    table::{ColumnAlign, render_table},
};
use std::path::{Path, PathBuf};

const APP_HEADER: &str = "APP";
const ENVIRONMENT_HEADER: &str = "ENVIRONMENT";
const CONFIG_HEADER: &str = "CONFIG";
const CANISTERS_HEADER: &str = "CANISTERS";
const ROLE_PREVIEW_LIMIT: usize = 6;

///
/// AppListRow
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct AppListRow {
    pub(super) app: String,
    pub(super) environment: String,
    pub(super) config: String,
    pub(super) canisters: String,
}

pub(super) fn render_app_list(
    workspace_root: &Path,
    choices: &[PathBuf],
    environment: &str,
) -> String {
    render_app_rows(app_list_rows(workspace_root, choices, environment))
}

pub(super) fn render_app_rows(rows: Vec<AppListRow>) -> String {
    let rows = rows
        .into_iter()
        .map(|row| [row.app, row.environment, row.config, row.canisters])
        .collect::<Vec<_>>();
    render_table(
        &[
            APP_HEADER,
            ENVIRONMENT_HEADER,
            CONFIG_HEADER,
            CANISTERS_HEADER,
        ],
        &rows,
        &[ColumnAlign::Left; 4],
    )
}

fn app_list_rows(workspace_root: &Path, choices: &[PathBuf], environment: &str) -> Vec<AppListRow> {
    choices
        .iter()
        .map(|path| app_list_row(workspace_root, path, environment))
        .collect()
}

fn app_list_row(workspace_root: &Path, path: &Path, environment: &str) -> AppListRow {
    let Ok(config) = AppConfigSnapshot::load(path) else {
        return AppListRow {
            environment: environment.to_string(),
            app: "invalid config".to_string(),
            config: display_workspace_path(workspace_root, path),
            canisters: "invalid config".to_string(),
        };
    };
    AppListRow {
        environment: environment.to_string(),
        app: config.app_id().to_string(),
        config: display_workspace_path(workspace_root, path),
        canisters: format_canister_summary(&config.deployable_roles()),
    }
}

fn format_canister_summary(roles: &[String]) -> String {
    if roles.is_empty() {
        return "0".to_string();
    }

    let preview = roles
        .iter()
        .take(ROLE_PREVIEW_LIMIT)
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(", ");
    let suffix = if roles.len() > ROLE_PREVIEW_LIMIT {
        ", ..."
    } else {
        ""
    };

    format!("{} ({preview}{suffix})", roles.len())
}

pub(super) fn render_role_lifecycle_rows(rows: &[ConfiguredRoleLifecycle]) -> String {
    let rows = rows
        .iter()
        .map(|row| {
            [
                row.display.clone(),
                row.package.clone(),
                row.state.clone(),
                row.topology.clone().unwrap_or_else(|| "-".to_string()),
            ]
        })
        .collect::<Vec<_>>();
    render_table(
        &["ROLE", "PACKAGE", "STATE", "TOPOLOGY"],
        &rows,
        &[ColumnAlign::Left; 4],
    )
}

pub(super) fn render_role_inspection(row: &ConfiguredRoleLifecycle) -> String {
    let topology = row.topology.as_deref().unwrap_or("-");
    let package = row.package.as_str();
    let deploy = if row.attached {
        "eligible"
    } else {
        "blocked: role is declared-only"
    };
    let next_action = if row.attached {
        format!("canic build {} {}", row.app, row.role)
    } else {
        format!(
            "canic app role attach {} {} --subnet <subnet>",
            row.app, row.role
        )
    };

    [
        "App role:".to_string(),
        format!("  role: {}", row.display),
        format!("  declaration: {}", row.declaration_kind),
        format!("  package: {package}"),
        format!("  state: {}", row.state),
        format!("  topology: {topology}"),
        "  cargo check: allowed".to_string(),
        format!("  deploy artifact: {deploy}"),
        format!("  next action: {next_action}"),
    ]
    .join("\n")
}

pub(super) fn render_declared_role(
    role: &DeclaredAppRole,
    workspace_root: &Path,
    config_path: &Path,
) -> String {
    [
        "Declared app role:".to_string(),
        format!("  role: {}", role.display),
        format!("  package: {}", role.package),
        format!(
            "  config: {}",
            display_workspace_path(workspace_root, config_path)
        ),
        "  state: declared".to_string(),
        format!(
            "  next action: canic app role attach {} {} --subnet <subnet>",
            role.app, role.role
        ),
    ]
    .join("\n")
}

pub(super) fn render_planned_declared_role(
    role: &DeclaredAppRole,
    workspace_root: &Path,
    config_path: &Path,
) -> String {
    let mut lines = vec![
        "Planned app role declaration:".to_string(),
        format!("  role: {}", role.display),
        format!("  package: {}", role.package),
        format!(
            "  would_write: {}",
            display_workspace_path(workspace_root, config_path)
        ),
    ];
    append_dry_run_footer(&mut lines);
    lines.join("\n")
}

pub(super) fn render_attached_role(
    role: &AttachedAppRole,
    workspace_root: &Path,
    config_path: &Path,
) -> String {
    [
        "Attached app role:".to_string(),
        format!("  role: {}", role.display),
        format!("  kind: {}", role.kind),
        format!("  topology: {}", role.topology),
        format!(
            "  config: {}",
            display_workspace_path(workspace_root, config_path)
        ),
        "  state: attached".to_string(),
        format!("  next action: canic build {} {}", role.app, role.role),
    ]
    .join("\n")
}

pub(super) fn render_planned_attached_role(
    role: &AttachedAppRole,
    workspace_root: &Path,
    config_path: &Path,
) -> String {
    let mut lines = vec![
        "Planned app role attachment:".to_string(),
        format!("  role: {}", role.display),
        format!("  kind: {}", role.kind),
        format!("  topology: {}", role.topology),
        format!(
            "  would_write: {}",
            display_workspace_path(workspace_root, config_path)
        ),
    ];
    append_dry_run_footer(&mut lines);
    lines.join("\n")
}

pub(super) fn render_renamed_role(
    role: &RenamedAppRole,
    workspace_root: &Path,
    config_path: &Path,
) -> String {
    let package = role.package_manifest.as_ref().map_or_else(
        || {
            role.package_manifest_note
                .as_deref()
                .unwrap_or("not updated")
                .to_string()
        },
        |path| display_workspace_path(workspace_root, path),
    );

    [
        "Renamed app role:".to_string(),
        format!("  old: {}", role.old_display),
        format!("  new: {}", role.new_display),
        format!(
            "  config: {}",
            display_workspace_path(workspace_root, config_path)
        ),
        format!("  package_manifest: {package}"),
        format!(
            "  next action: canic app role inspect {} {}",
            role.app, role.new_role
        ),
    ]
    .join("\n")
}

pub(super) fn render_planned_renamed_role(
    role: &RenamedAppRole,
    workspace_root: &Path,
    config_path: &Path,
) -> String {
    let package = role.package_manifest.as_ref().map_or_else(
        || {
            role.package_manifest_note
                .as_deref()
                .unwrap_or("not updated")
                .to_string()
        },
        |path| display_workspace_path(workspace_root, path),
    );

    let mut lines = vec![
        "Planned app role rename:".to_string(),
        format!("  old: {}", role.old_display),
        format!("  new: {}", role.new_display),
        format!(
            "  would_write: {}",
            display_workspace_path(workspace_root, config_path)
        ),
        format!("  would_write_package_manifest: {package}"),
    ];
    append_dry_run_footer(&mut lines);
    lines.join("\n")
}

pub(super) fn render_planned_delete(workspace_root: &Path, app: &str, target: &Path) -> String {
    let mut lines = vec![
        "Planned app delete:".to_string(),
        format!("  app: {app}"),
        format!(
            "  would_remove: {}",
            display_workspace_path(workspace_root, target)
        ),
    ];
    append_dry_run_footer(&mut lines);
    lines.join("\n")
}
