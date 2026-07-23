mod adoption_report;
mod command;
mod options;
mod render;

use crate::{
    cli::clap::parse_subcommand, cli::help::print_help_or_version, scaffold, version_text,
};
use adoption_report::{
    build_adoption_report_from_config_path, current_adoption_report_generated_at,
    write_adoption_report,
};
#[cfg(test)]
use adoption_report::{cargo_metadata_package_path, render_adoption_report};
use canic_host::{
    adoption::AdoptionReportError,
    icp_config::{IcpConfigError, IcpProjectConfigReport, inspect_canic_icp_yaml},
    install_root::{
        ConfigDiscoveryError, current_canic_project_root, discover_current_canic_config_choices,
        project_app_roots, select_discovered_app_config_path,
    },
    release_set::{
        AppConfigError, AppConfigSnapshot, attach_app_role, declare_app_role,
        display_workspace_path, plan_attach_app_role, plan_declare_app_role, plan_rename_app_role,
        rename_app_role,
    },
};
use command::{
    adoption_report_usage, adoption_usage, app_adoption_command, app_command, app_role_command,
    check_usage, create_usage, delete_usage, list_usage, role_attach_usage, role_declare_usage,
    role_inspect_usage, role_list_usage, role_rename_usage, role_usage, usage,
};
#[cfg(test)]
use options::AdoptionReportFormat;
use options::{
    AdoptionReportOptions, AppCheckOptions, AppOptions, DeleteAppOptions, RoleAttachOptions,
    RoleDeclareOptions, RoleInspectOptions, RoleListOptions, RoleRenameOptions,
};
#[cfg(test)]
use render::{AppListRow, render_app_rows};
use render::{
    render_app_list, render_attached_role, render_declared_role, render_planned_attached_role,
    render_planned_declared_role, render_planned_delete, render_planned_renamed_role,
    render_renamed_role, render_role_inspection, render_role_lifecycle_rows,
};
use std::{
    ffi::OsString,
    fs,
    io::{self, BufRead, Write},
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

///
/// AppCommandError
///

#[derive(Debug, ThisError)]
pub enum AppCommandError {
    #[error("{0}")]
    Usage(String),

    #[error("no Canic app configs found under apps; run canic app create <name>")]
    NoConfigChoices,

    #[error("unknown app {0}; run canic app list to inspect config-defined apps")]
    UnknownApp(String),

    #[error("app delete cancelled")]
    DeleteCancelled,

    #[error("refusing to delete app {app}; target {target} is not under an App config root")]
    UnsafeDeleteTarget { app: String, target: String },

    #[error("app {0} config does not have a parent directory")]
    MissingAppDirectory(String),

    #[error("unknown role {role} in app {app}; run canic app role list {app}")]
    UnknownRole { app: String, role: String },

    #[error("app create: {0}")]
    Create(#[from] scaffold::ScaffoldCommandError),

    #[error("config: {0}")]
    Config(#[from] crate::list::ListCommandError),

    #[error(transparent)]
    AdoptionReport(#[from] AdoptionReportError),

    #[error(transparent)]
    IcpConfig(#[from] IcpConfigError),

    #[error("failed to discover Canic project configs: {0}")]
    ConfigDiscovery(#[from] ConfigDiscoveryError),

    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    AppConfig(#[from] AppConfigError),

    #[error(transparent)]
    Host(#[from] Box<dyn std::error::Error>),
}

pub fn run<I>(args: I) -> Result<(), AppCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    match parse_subcommand(app_command(), args).map_err(|_| AppCommandError::Usage(usage()))? {
        None => {
            println!("{}", usage());
            Ok(())
        }
        Some((command, args)) => match command.as_str() {
            "create" => run_create(args),
            "check" => run_check(args),
            "delete" => run_delete(args),
            "list" => run_list(args),
            "config" => run_config(args),
            "adoption" => run_adoption(args),
            "role" => run_role(args),
            _ => unreachable!("app dispatch command only defines known commands"),
        },
    }
}

fn run_adoption<I>(args: I) -> Result<(), AppCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, adoption_usage, version_text()) {
        return Ok(());
    }

    match parse_subcommand(app_adoption_command(), args)
        .map_err(|_| AppCommandError::Usage(adoption_usage()))?
    {
        None => {
            println!("{}", adoption_usage());
            Ok(())
        }
        Some((command, args)) => match command.as_str() {
            "report" => run_adoption_report(args),
            _ => unreachable!("app adoption dispatch command only defines known commands"),
        },
    }
}

fn run_adoption_report<I>(args: I) -> Result<(), AppCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, adoption_report_usage, version_text()) {
        return Ok(());
    }

    let options = AdoptionReportOptions::parse(args)?;
    let config_path = selected_app_config_path(&options.app)?;
    let generated_at = current_adoption_report_generated_at()?;
    let report = build_adoption_report_from_config_path(&config_path, &options, &generated_at)?;
    write_adoption_report(&config_path, &options, &report)
}

fn run_role<I>(args: I) -> Result<(), AppCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, role_usage, version_text()) {
        return Ok(());
    }

    match parse_subcommand(app_role_command(), args)
        .map_err(|_| AppCommandError::Usage(role_usage()))?
    {
        None => {
            println!("{}", role_usage());
            Ok(())
        }
        Some((command, args)) => match command.as_str() {
            "declare" => run_role_declare(args),
            "attach" => run_role_attach(args),
            "rename" => run_role_rename(args),
            "list" => run_role_list(args),
            "inspect" => run_role_inspect(args),
            _ => unreachable!("app role dispatch command only defines known commands"),
        },
    }
}

fn run_role_declare<I>(args: I) -> Result<(), AppCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, role_declare_usage, version_text()) {
        return Ok(());
    }

    let options = RoleDeclareOptions::parse(args)?;
    let config_path = selected_app_config_path(&options.app)?;
    let project_root = current_canic_project_root()?;
    if options.dry_run {
        let declared =
            plan_declare_app_role(&config_path, &options.app, &options.role, &options.package)?;
        println!(
            "{}",
            render_planned_declared_role(&declared, &project_root, &config_path)
        );
    } else {
        let declared =
            declare_app_role(&config_path, &options.app, &options.role, &options.package)?;
        println!(
            "{}",
            render_declared_role(&declared, &project_root, &config_path)
        );
    }
    Ok(())
}

fn run_role_attach<I>(args: I) -> Result<(), AppCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, role_attach_usage, version_text()) {
        return Ok(());
    }

    let options = RoleAttachOptions::parse(args)?;
    let config_path = selected_app_config_path(&options.app)?;
    let project_root = current_canic_project_root()?;
    if options.dry_run {
        let attached = plan_attach_app_role(
            &config_path,
            &options.app,
            &options.role,
            &options.subnet,
            &options.kind,
        )?;
        println!(
            "{}",
            render_planned_attached_role(&attached, &project_root, &config_path)
        );
    } else {
        let attached = attach_app_role(
            &config_path,
            &options.app,
            &options.role,
            &options.subnet,
            &options.kind,
        )?;
        println!(
            "{}",
            render_attached_role(&attached, &project_root, &config_path)
        );
    }
    Ok(())
}

fn run_role_rename<I>(args: I) -> Result<(), AppCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, role_rename_usage, version_text()) {
        return Ok(());
    }

    let options = RoleRenameOptions::parse(args)?;
    let config_path = selected_app_config_path(&options.app)?;
    let project_root = current_canic_project_root()?;
    if options.dry_run {
        let renamed = plan_rename_app_role(
            &config_path,
            &options.app,
            &options.old_role,
            &options.new_role,
        )?;
        println!(
            "{}",
            render_planned_renamed_role(&renamed, &project_root, &config_path)
        );
    } else {
        let renamed = rename_app_role(
            &config_path,
            &options.app,
            &options.old_role,
            &options.new_role,
        )?;
        println!(
            "{}",
            render_renamed_role(&renamed, &project_root, &config_path)
        );
    }
    Ok(())
}

fn run_role_list<I>(args: I) -> Result<(), AppCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, role_list_usage, version_text()) {
        return Ok(());
    }

    let options = RoleListOptions::parse(args)?;
    let config_path = selected_app_config_path(&options.app)?;
    let rows = AppConfigSnapshot::load(&config_path)?.role_lifecycle();
    println!("{}", render_role_lifecycle_rows(&rows));
    Ok(())
}

fn run_role_inspect<I>(args: I) -> Result<(), AppCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, role_inspect_usage, version_text()) {
        return Ok(());
    }

    let options = RoleInspectOptions::parse(args)?;
    let config_path = selected_app_config_path(&options.app)?;
    let rows = AppConfigSnapshot::load(&config_path)?.role_lifecycle();
    let row = rows
        .iter()
        .find(|row| row.role == options.role)
        .ok_or_else(|| AppCommandError::UnknownRole {
            app: options.app.clone(),
            role: options.role.clone(),
        })?;
    println!("{}", render_role_inspection(row));
    Ok(())
}

fn run_check<I>(args: I) -> Result<(), AppCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, check_usage, version_text()) {
        return Ok(());
    }

    let options = AppCheckOptions::parse(args)?;
    let report = inspect_canic_icp_yaml(Some(&options.app))?;
    print_config_report(&report);
    Ok(())
}

fn run_create<I>(args: I) -> Result<(), AppCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, create_usage, version_text()) {
        return Ok(());
    }

    scaffold::run_app_create(args).map_err(AppCommandError::from)
}

fn run_list<I>(args: I) -> Result<(), AppCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, list_usage, version_text()) {
        return Ok(());
    }

    let options = AppOptions::parse(args)?;
    let project_root = current_canic_project_root()?;
    let choices = discover_config_choices()?;
    if choices.is_empty() {
        return Err(AppCommandError::NoConfigChoices);
    }
    println!(
        "{}",
        render_app_list(&project_root, &choices, &options.environment)
    );
    Ok(())
}

fn run_config<I>(args: I) -> Result<(), AppCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    crate::list::run_config(args).map_err(AppCommandError::from)
}

fn run_delete<I>(args: I) -> Result<(), AppCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, delete_usage, version_text()) {
        return Ok(());
    }

    let options = DeleteAppOptions::parse(args)?;
    let project_root = current_canic_project_root()?;
    let target = delete_target_dir(&project_root, &options.app)?;
    if options.dry_run {
        println!(
            "{}",
            render_planned_delete(&project_root, &options.app, &target)
        );
        return Ok(());
    }
    confirm_delete_app(&options.app, &target, io::stdin().lock(), io::stdout())?;
    fs::remove_dir_all(&target)?;

    println!("Deleted Canic app:");
    println!("  app: {}", options.app);
    println!(
        "  path:  {}",
        display_workspace_path(&project_root, &target)
    );
    Ok(())
}

fn discover_config_choices() -> Result<Vec<PathBuf>, ConfigDiscoveryError> {
    discover_current_canic_config_choices()
}

fn delete_target_dir(workspace_root: &Path, app: &str) -> Result<PathBuf, AppCommandError> {
    let choices = discover_config_choices()?;
    delete_target_dir_from_choices(workspace_root, &choices, app)
}

fn selected_app_config_path(app: &str) -> Result<PathBuf, AppCommandError> {
    let choices = discover_config_choices()?;
    if choices.is_empty() {
        return Err(AppCommandError::NoConfigChoices);
    }
    selected_app_config_path_from_choices(&choices, app)
}

fn selected_app_config_path_from_choices(
    choices: &[PathBuf],
    app: &str,
) -> Result<PathBuf, AppCommandError> {
    select_discovered_app_config_path(choices, app)?
        .ok_or_else(|| AppCommandError::UnknownApp(app.to_string()))
}

fn delete_target_dir_from_choices(
    workspace_root: &Path,
    choices: &[PathBuf],
    app: &str,
) -> Result<PathBuf, AppCommandError> {
    let config_path = select_discovered_app_config_path(choices, app)?
        .ok_or_else(|| AppCommandError::UnknownApp(app.to_string()))?;
    let target = config_path
        .parent()
        .ok_or_else(|| AppCommandError::MissingAppDirectory(app.to_string()))?
        .to_path_buf();
    if !is_safe_delete_target(workspace_root, &target) {
        return Err(AppCommandError::UnsafeDeleteTarget {
            app: app.to_string(),
            target: target.display().to_string(),
        });
    }

    Ok(target)
}

// Restrict destructive delete targets to one app directory, never the app root.
fn is_safe_delete_target(workspace_root: &Path, target: &Path) -> bool {
    let Ok(metadata) = fs::symlink_metadata(target) else {
        return false;
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return false;
    }

    let Ok(target) = target.canonicalize() else {
        return false;
    };
    project_app_roots(workspace_root)
        .into_iter()
        .filter_map(|root| root.canonicalize().ok())
        .any(|root| target != root && target.starts_with(root))
}

// Confirm destructive app directory deletion by requiring the exact app name.
fn confirm_delete_app<R, W>(
    app: &str,
    target: &Path,
    mut reader: R,
    mut writer: W,
) -> Result<(), AppCommandError>
where
    R: BufRead,
    W: Write,
{
    writeln!(writer, "Delete Canic app?")?;
    writeln!(writer, "  app: {app}")?;
    writeln!(writer, "  target: {}", target.display())?;
    writeln!(writer, "This will permanently remove the app directory.")?;
    write!(writer, "Type the app name to confirm: ")?;
    writer.flush()?;

    let mut answer = String::new();
    reader.read_line(&mut answer)?;
    if answer.trim() == app {
        return Ok(());
    }

    Err(AppCommandError::DeleteCancelled)
}

fn print_config_report(report: &IcpProjectConfigReport) {
    println!("Checked ICP project config:");
    println!("  path: {}", report.path.display());
    println!("  canisters: {}", report.canisters.len());
    println!("  environments: {}", report.environments.len());
    println!(
        "  status: {}",
        if report.is_ready() {
            "ok"
        } else {
            "incomplete"
        }
    );
    for issue in report.issues() {
        println!("  issue: {issue}");
    }
}

#[cfg(test)]
mod tests;
