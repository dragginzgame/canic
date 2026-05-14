use crate::{
    cli::clap::{
        parse_matches, parse_subcommand, passthrough_subcommand, path_option, string_option,
        value_arg,
    },
    cli::defaults::local_network,
    cli::globals::internal_network_arg,
    cli::help::print_help_or_version,
    scaffold, version_text,
};
use canic_host::{
    icp_config::{IcpConfigError, IcpProjectSyncReport, sync_canic_icp_yaml_with_fleet_root},
    install_root::{
        discover_current_canic_config_choices, discover_project_canic_config_choices_with_root,
        project_fleet_roots_with_override,
    },
    release_set::{
        configured_fleet_name, configured_fleet_roles, display_workspace_path,
        matching_fleet_config_paths, workspace_root,
    },
    table::{ColumnAlign, render_table},
};
use clap::Command as ClapCommand;
use std::{
    ffi::OsString,
    fs,
    io::{self, BufRead, Write},
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

const FLEET_HEADER: &str = "FLEET";
const NETWORK_HEADER: &str = "NETWORK";
const CONFIG_HEADER: &str = "CONFIG";
const CANISTERS_HEADER: &str = "CANISTERS";
const ROLE_PREVIEW_LIMIT: usize = 6;
const FLEET_HELP_AFTER: &str = "\
Examples:
  canic fleet list
  canic fleet create demo
  canic fleet sync
  canic fleet delete demo";
const FLEET_LIST_HELP_AFTER: &str = "\
Examples:
  canic fleet list

Commands that operate on one fleet take the fleet name as a positional argument.";
const FLEET_SYNC_HELP_AFTER: &str = "\
Examples:
  canic fleet sync
  canic fleet sync --fleet test";
const FLEET_DELETE_HELP_AFTER: &str = "\
Examples:
  canic fleet delete demo

This removes the matching config-defined fleet directory after you type the
fleet name exactly.";

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

    #[error("fleet create: {0}")]
    Create(String),

    #[error(transparent)]
    IcpConfig(#[from] IcpConfigError),

    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    Host(#[from] Box<dyn std::error::Error>),
}

///
/// FleetOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct FleetOptions {
    network: String,
    fleets_dir: Option<PathBuf>,
}

///
/// DeleteFleetOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct DeleteFleetOptions {
    fleet: String,
    fleets_dir: Option<PathBuf>,
}

///
/// FleetSyncOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct FleetSyncOptions {
    fleet: Option<String>,
    fleets_dir: Option<PathBuf>,
}

///
/// FleetListRow
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct FleetListRow {
    fleet: String,
    network: String,
    config: String,
    canisters: String,
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
            "delete" => run_delete(args),
            "list" => run_list(args),
            "sync" => run_sync(args),
            _ => unreachable!("fleet dispatch command only defines known commands"),
        },
    }
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
    let workspace_root = workspace_root()?;
    let choices = discover_config_choices(options.fleets_dir.as_deref())?;
    if choices.is_empty() {
        return Err(FleetCommandError::NoConfigChoices);
    }
    println!(
        "{}",
        render_fleet_list(&workspace_root, &choices, &options.network)
    );
    Ok(())
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
    let workspace_root = workspace_root()?;
    let target = delete_target_dir(
        &workspace_root,
        &options.fleet,
        options.fleets_dir.as_deref(),
    )?;
    confirm_delete_fleet(&options.fleet, &target, io::stdin().lock(), io::stdout())?;
    fs::remove_dir_all(&target)?;

    println!("Deleted Canic fleet:");
    println!("  fleet: {}", options.fleet);
    println!(
        "  path:  {}",
        display_workspace_path(&workspace_root, &target)
    );
    Ok(())
}

fn run_sync<I>(args: I) -> Result<(), FleetCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, sync_usage, version_text()) {
        return Ok(());
    }

    let options = FleetSyncOptions::parse(args)?;
    let report = sync_canic_icp_yaml_with_fleet_root(
        options.fleet.as_deref(),
        options.fleets_dir.as_deref(),
    )?;
    print_sync_report(&report);
    Ok(())
}

impl FleetOptions {
    fn parse<I>(args: I) -> Result<Self, FleetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(fleet_list_command(), args)
            .map_err(|_| FleetCommandError::Usage(list_usage()))?;

        Ok(Self {
            network: string_option(&matches, "network").unwrap_or_else(local_network),
            fleets_dir: path_option(&matches, "fleets-dir"),
        })
    }
}

impl DeleteFleetOptions {
    fn parse<I>(args: I) -> Result<Self, FleetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(fleet_delete_command(), args)
            .map_err(|_| FleetCommandError::Usage(delete_usage()))?;

        Ok(Self {
            fleet: string_option(&matches, "fleet").expect("clap requires fleet"),
            fleets_dir: path_option(&matches, "fleets-dir"),
        })
    }
}

impl FleetSyncOptions {
    #[cfg(test)]
    fn parse_test<I>(args: I) -> Result<Self, FleetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        Self::parse(args)
    }

    fn parse<I>(args: I) -> Result<Self, FleetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(fleet_sync_command(), args)
            .map_err(|_| FleetCommandError::Usage(sync_usage()))?;

        Ok(Self {
            fleet: string_option(&matches, "fleet"),
            fleets_dir: path_option(&matches, "fleets-dir"),
        })
    }
}

fn discover_config_choices(
    fleets_dir: Option<&Path>,
) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    if let Some(fleets_dir) = fleets_dir {
        return Ok(discover_project_canic_config_choices_with_root(
            &workspace_root()?,
            Some(fleets_dir),
        )?);
    }
    discover_current_canic_config_choices()
}

fn delete_target_dir(
    workspace_root: &Path,
    fleet: &str,
    fleets_dir: Option<&Path>,
) -> Result<PathBuf, FleetCommandError> {
    let choices = discover_config_choices(fleets_dir)?;
    delete_target_dir_from_choices(workspace_root, &choices, fleet, fleets_dir)
}

fn delete_target_dir_from_choices(
    workspace_root: &Path,
    choices: &[PathBuf],
    fleet: &str,
    fleets_dir: Option<&Path>,
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
    if !is_safe_delete_target(workspace_root, &target, fleets_dir) {
        return Err(FleetCommandError::UnsafeDeleteTarget {
            fleet: fleet.to_string(),
            target: target.display().to_string(),
        });
    }

    Ok(target)
}

// Restrict destructive delete targets to one fleet directory, never the fleet root.
fn is_safe_delete_target(workspace_root: &Path, target: &Path, fleets_dir: Option<&Path>) -> bool {
    let Ok(metadata) = fs::symlink_metadata(target) else {
        return false;
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return false;
    }

    let Ok(target) = target.canonicalize() else {
        return false;
    };
    project_fleet_roots_with_override(workspace_root, fleets_dir)
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

fn fleet_command() -> ClapCommand {
    ClapCommand::new("fleet")
        .bin_name("canic fleet")
        .about("Manage Canic fleets")
        .disable_help_flag(true)
        .subcommand(passthrough_subcommand(
            ClapCommand::new("create")
                .about("Create a minimal Canic fleet")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("list")
                .about("List config-defined Canic fleets")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("sync")
                .about("Sync fleet configs into icp.yaml")
                .disable_help_flag(true),
        ))
        .subcommand(passthrough_subcommand(
            ClapCommand::new("delete")
                .about("Delete a config-defined Canic fleet")
                .disable_help_flag(true),
        ))
        .after_help(FLEET_HELP_AFTER)
}

fn fleet_list_command() -> ClapCommand {
    ClapCommand::new("list")
        .bin_name("canic fleet list")
        .about("List config-defined Canic fleets")
        .disable_help_flag(true)
        .arg(internal_network_arg())
        .arg(fleets_dir_arg())
        .after_help(FLEET_LIST_HELP_AFTER)
}

fn fleet_sync_command() -> ClapCommand {
    ClapCommand::new("sync")
        .bin_name("canic fleet sync")
        .about("Sync fleet configs into icp.yaml")
        .disable_help_flag(true)
        .arg(
            value_arg("fleet")
                .long("fleet")
                .value_name("name")
                .help("Require this fleet to exist before syncing"),
        )
        .arg(fleets_dir_arg())
        .after_help(FLEET_SYNC_HELP_AFTER)
}

fn fleet_delete_command() -> ClapCommand {
    ClapCommand::new("delete")
        .bin_name("canic fleet delete")
        .about("Delete a config-defined Canic fleet directory")
        .disable_help_flag(true)
        .arg(
            value_arg("fleet")
                .value_name("name")
                .required(true)
                .help("Config-defined fleet name to delete"),
        )
        .arg(fleets_dir_arg())
        .after_help(FLEET_DELETE_HELP_AFTER)
}

fn fleets_dir_arg() -> clap::Arg {
    value_arg("fleets-dir")
        .long("fleets-dir")
        .value_name("DIR")
        .help("Read fleet configs from this directory")
}

fn render_fleet_list(workspace_root: &Path, choices: &[PathBuf], network: &str) -> String {
    render_fleet_rows(fleet_list_rows(workspace_root, choices, network))
}

fn render_fleet_rows(rows: Vec<FleetListRow>) -> String {
    let rows = rows
        .into_iter()
        .map(|row| [row.fleet, row.network, row.config, row.canisters])
        .collect::<Vec<_>>();
    render_table(
        &[
            FLEET_HEADER,
            NETWORK_HEADER,
            CONFIG_HEADER,
            CANISTERS_HEADER,
        ],
        &rows,
        &[ColumnAlign::Left; 4],
    )
}

fn fleet_list_rows(workspace_root: &Path, choices: &[PathBuf], network: &str) -> Vec<FleetListRow> {
    choices
        .iter()
        .map(|path| fleet_list_row(workspace_root, path, network))
        .collect()
}

fn fleet_list_row(workspace_root: &Path, path: &Path, network: &str) -> FleetListRow {
    let fleet = configured_fleet_name(path).unwrap_or_else(|_| "invalid config".to_string());
    FleetListRow {
        network: network.to_string(),
        fleet,
        config: display_workspace_path(workspace_root, path),
        canisters: configured_fleet_roles(path).map_or_else(
            |_| "invalid config".to_string(),
            |roles| format_canister_summary(&roles),
        ),
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

fn print_sync_report(report: &IcpProjectSyncReport) {
    println!("Synced ICP project config:");
    println!("  path: {}", report.path.display());
    println!("  canisters: {}", report.canisters.len());
    println!("  environments: {}", report.environments.len());
    println!("  changed: {}", if report.changed { "yes" } else { "no" });
}

fn usage() -> String {
    let mut command = fleet_command();
    command.render_help().to_string()
}

fn list_usage() -> String {
    let mut command = fleet_list_command();
    command.render_help().to_string()
}

fn sync_usage() -> String {
    let mut command = fleet_sync_command();
    command.render_help().to_string()
}

fn create_usage() -> String {
    scaffold::fleet_create_usage()
}

fn delete_usage() -> String {
    let mut command = fleet_delete_command();
    command.render_help().to_string()
}

#[cfg(test)]
mod tests;
