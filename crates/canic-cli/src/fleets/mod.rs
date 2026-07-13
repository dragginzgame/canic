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
        current_canic_project_root, discover_current_canic_config_choices, project_fleet_roots,
    },
    release_set::{
        FleetConfigError, attach_fleet_role, configured_role_lifecycle, declare_fleet_role,
        display_workspace_path, matching_fleet_config_paths, plan_attach_fleet_role,
        plan_declare_fleet_role, plan_rename_fleet_role, rename_fleet_role,
    },
};
use command::{
    adoption_report_usage, adoption_usage, check_usage, create_usage, delete_usage,
    fleet_adoption_command, fleet_command, fleet_role_command, list_usage, role_attach_usage,
    role_declare_usage, role_inspect_usage, role_list_usage, role_rename_usage, role_usage, usage,
};
#[cfg(test)]
use options::AdoptionReportFormat;
use options::{
    AdoptionReportOptions, DeleteFleetOptions, FleetCheckOptions, FleetOptions, RoleAttachOptions,
    RoleDeclareOptions, RoleInspectOptions, RoleListOptions, RoleRenameOptions,
};
#[cfg(test)]
use render::{FleetListRow, render_fleet_rows};
use render::{
    render_attached_role, render_declared_role, render_fleet_list, render_planned_attached_role,
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
/// FleetCommandError
///

#[derive(Debug, ThisError)]
pub enum FleetCommandError {
    #[error("{0}")]
    Usage(String),

    #[error("no Canic fleet configs found under fleets; run canic fleet create <name>")]
    NoConfigChoices,

    #[error("unknown fleet {0}; run canic fleet list to inspect config-defined fleets")]
    UnknownFleet(String),

    #[error(
        "multiple configs declare fleet {0}; use distinct [fleet].name values before selecting it"
    )]
    DuplicateFleet(String),

    #[error("fleet delete cancelled")]
    DeleteCancelled,

    #[error("refusing to delete fleet {fleet}; target {target} is not under a fleet config root")]
    UnsafeDeleteTarget { fleet: String, target: String },

    #[error("fleet {0} config does not have a parent directory")]
    MissingFleetDirectory(String),

    #[error("unknown role {role} in fleet {fleet}; run canic fleet role list {fleet}")]
    UnknownRole { fleet: String, role: String },

    #[error("fleet create: {0}")]
    Create(String),

    #[error("config: {0}")]
    Config(String),

    #[error(transparent)]
    AdoptionReport(#[from] AdoptionReportError),

    #[error(transparent)]
    IcpConfig(#[from] IcpConfigError),

    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    FleetConfig(#[from] FleetConfigError),

    #[error(transparent)]
    Host(#[from] Box<dyn std::error::Error>),
}

pub fn run<I>(args: I) -> Result<(), FleetCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    match parse_subcommand(fleet_command(), args).map_err(|_| FleetCommandError::Usage(usage()))? {
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
            _ => unreachable!("fleet dispatch command only defines known commands"),
        },
    }
}

fn run_adoption<I>(args: I) -> Result<(), FleetCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, adoption_usage, version_text()) {
        return Ok(());
    }

    match parse_subcommand(fleet_adoption_command(), args)
        .map_err(|_| FleetCommandError::Usage(adoption_usage()))?
    {
        None => {
            println!("{}", adoption_usage());
            Ok(())
        }
        Some((command, args)) => match command.as_str() {
            "report" => run_adoption_report(args),
            _ => unreachable!("fleet adoption dispatch command only defines known commands"),
        },
    }
}

fn run_adoption_report<I>(args: I) -> Result<(), FleetCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, adoption_report_usage, version_text()) {
        return Ok(());
    }

    let options = AdoptionReportOptions::parse(args)?;
    let config_path = selected_fleet_config_path(&options.fleet)?;
    let generated_at = current_adoption_report_generated_at()?;
    let report = build_adoption_report_from_config_path(&config_path, &options, &generated_at)?;
    write_adoption_report(&config_path, &options, &report)
}

fn run_role<I>(args: I) -> Result<(), FleetCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, role_usage, version_text()) {
        return Ok(());
    }

    match parse_subcommand(fleet_role_command(), args)
        .map_err(|_| FleetCommandError::Usage(role_usage()))?
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
            _ => unreachable!("fleet role dispatch command only defines known commands"),
        },
    }
}

fn run_role_declare<I>(args: I) -> Result<(), FleetCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, role_declare_usage, version_text()) {
        return Ok(());
    }

    let options = RoleDeclareOptions::parse(args)?;
    let config_path = selected_fleet_config_path(&options.fleet)?;
    let project_root = current_canic_project_root()?;
    if options.dry_run {
        let declared = plan_declare_fleet_role(
            &config_path,
            &options.fleet,
            &options.role,
            &options.package,
        )?;
        println!(
            "{}",
            render_planned_declared_role(&declared, &project_root, &config_path)
        );
    } else {
        let declared = declare_fleet_role(
            &config_path,
            &options.fleet,
            &options.role,
            &options.package,
        )?;
        println!(
            "{}",
            render_declared_role(&declared, &project_root, &config_path)
        );
    }
    Ok(())
}

fn run_role_attach<I>(args: I) -> Result<(), FleetCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, role_attach_usage, version_text()) {
        return Ok(());
    }

    let options = RoleAttachOptions::parse(args)?;
    let config_path = selected_fleet_config_path(&options.fleet)?;
    let project_root = current_canic_project_root()?;
    if options.dry_run {
        let attached = plan_attach_fleet_role(
            &config_path,
            &options.fleet,
            &options.role,
            &options.subnet,
            &options.kind,
        )?;
        println!(
            "{}",
            render_planned_attached_role(&attached, &project_root, &config_path)
        );
    } else {
        let attached = attach_fleet_role(
            &config_path,
            &options.fleet,
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

fn run_role_rename<I>(args: I) -> Result<(), FleetCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, role_rename_usage, version_text()) {
        return Ok(());
    }

    let options = RoleRenameOptions::parse(args)?;
    let config_path = selected_fleet_config_path(&options.fleet)?;
    let project_root = current_canic_project_root()?;
    if options.dry_run {
        let renamed = plan_rename_fleet_role(
            &config_path,
            &options.fleet,
            &options.old_role,
            &options.new_role,
        )?;
        println!(
            "{}",
            render_planned_renamed_role(&renamed, &project_root, &config_path)
        );
    } else {
        let renamed = rename_fleet_role(
            &config_path,
            &options.fleet,
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

fn run_role_list<I>(args: I) -> Result<(), FleetCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, role_list_usage, version_text()) {
        return Ok(());
    }

    let options = RoleListOptions::parse(args)?;
    let config_path = selected_fleet_config_path(&options.fleet)?;
    let rows = configured_role_lifecycle(&config_path)?;
    println!("{}", render_role_lifecycle_rows(&rows));
    Ok(())
}

fn run_role_inspect<I>(args: I) -> Result<(), FleetCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, role_inspect_usage, version_text()) {
        return Ok(());
    }

    let options = RoleInspectOptions::parse(args)?;
    let config_path = selected_fleet_config_path(&options.fleet)?;
    let rows = configured_role_lifecycle(&config_path)?;
    let row = rows
        .iter()
        .find(|row| row.role == options.role)
        .ok_or_else(|| FleetCommandError::UnknownRole {
            fleet: options.fleet.clone(),
            role: options.role.clone(),
        })?;
    println!("{}", render_role_inspection(row));
    Ok(())
}

fn run_check<I>(args: I) -> Result<(), FleetCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, check_usage, version_text()) {
        return Ok(());
    }

    let options = FleetCheckOptions::parse(args)?;
    let report = inspect_canic_icp_yaml(Some(&options.fleet))?;
    print_config_report(&report);
    Ok(())
}

fn run_create<I>(args: I) -> Result<(), FleetCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, create_usage, version_text()) {
        return Ok(());
    }

    scaffold::run_fleet_create(args).map_err(|err| FleetCommandError::Create(err.to_string()))
}

fn run_list<I>(args: I) -> Result<(), FleetCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, list_usage, version_text()) {
        return Ok(());
    }

    let options = FleetOptions::parse(args)?;
    let project_root = current_canic_project_root()?;
    let choices = discover_config_choices()?;
    if choices.is_empty() {
        return Err(FleetCommandError::NoConfigChoices);
    }
    println!(
        "{}",
        render_fleet_list(&project_root, &choices, &options.network)
    );
    Ok(())
}

fn run_config<I>(args: I) -> Result<(), FleetCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    crate::list::run_config(args).map_err(|err| FleetCommandError::Config(err.to_string()))
}

fn run_delete<I>(args: I) -> Result<(), FleetCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, delete_usage, version_text()) {
        return Ok(());
    }

    let options = DeleteFleetOptions::parse(args)?;
    let project_root = current_canic_project_root()?;
    let target = delete_target_dir(&project_root, &options.fleet)?;
    if options.dry_run {
        println!(
            "{}",
            render_planned_delete(&project_root, &options.fleet, &target)
        );
        return Ok(());
    }
    confirm_delete_fleet(&options.fleet, &target, io::stdin().lock(), io::stdout())?;
    fs::remove_dir_all(&target)?;

    println!("Deleted Canic fleet:");
    println!("  fleet: {}", options.fleet);
    println!(
        "  path:  {}",
        display_workspace_path(&project_root, &target)
    );
    Ok(())
}

fn discover_config_choices() -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    discover_current_canic_config_choices()
}

fn delete_target_dir(workspace_root: &Path, fleet: &str) -> Result<PathBuf, FleetCommandError> {
    let choices = discover_config_choices()?;
    delete_target_dir_from_choices(workspace_root, &choices, fleet)
}

fn selected_fleet_config_path(fleet: &str) -> Result<PathBuf, FleetCommandError> {
    let choices = discover_config_choices()?;
    if choices.is_empty() {
        return Err(FleetCommandError::NoConfigChoices);
    }
    selected_fleet_config_path_from_choices(&choices, fleet)
}

fn selected_fleet_config_path_from_choices(
    choices: &[PathBuf],
    fleet: &str,
) -> Result<PathBuf, FleetCommandError> {
    let matches = matching_fleet_config_paths(choices, fleet);

    match matches.as_slice() {
        [] => Err(FleetCommandError::UnknownFleet(fleet.to_string())),
        [path] => Ok(path.clone()),
        _ => Err(FleetCommandError::DuplicateFleet(fleet.to_string())),
    }
}

fn delete_target_dir_from_choices(
    workspace_root: &Path,
    choices: &[PathBuf],
    fleet: &str,
) -> Result<PathBuf, FleetCommandError> {
    let matches = matching_fleet_config_paths(choices, fleet);

    let config_path = match matches.as_slice() {
        [] => return Err(FleetCommandError::UnknownFleet(fleet.to_string())),
        [path] => path,
        _ => return Err(FleetCommandError::DuplicateFleet(fleet.to_string())),
    };
    let target = config_path
        .parent()
        .ok_or_else(|| FleetCommandError::MissingFleetDirectory(fleet.to_string()))?
        .to_path_buf();
    if !is_safe_delete_target(workspace_root, &target) {
        return Err(FleetCommandError::UnsafeDeleteTarget {
            fleet: fleet.to_string(),
            target: target.display().to_string(),
        });
    }

    Ok(target)
}

// Restrict destructive delete targets to one fleet directory, never the fleet root.
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
    project_fleet_roots(workspace_root)
        .into_iter()
        .filter_map(|root| root.canonicalize().ok())
        .any(|root| target != root && target.starts_with(root))
}

// Confirm destructive fleet directory deletion by requiring the exact fleet name.
fn confirm_delete_fleet<R, W>(
    fleet: &str,
    target: &Path,
    mut reader: R,
    mut writer: W,
) -> Result<(), FleetCommandError>
where
    R: BufRead,
    W: Write,
{
    writeln!(writer, "Delete Canic fleet?")?;
    writeln!(writer, "  fleet: {fleet}")?;
    writeln!(writer, "  target: {}", target.display())?;
    writeln!(writer, "This will permanently remove the fleet directory.")?;
    write!(writer, "Type the fleet name to confirm: ")?;
    writer.flush()?;

    let mut answer = String::new();
    reader.read_line(&mut answer)?;
    if answer.trim() == fleet {
        return Ok(());
    }

    Err(FleetCommandError::DeleteCancelled)
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
