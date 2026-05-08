use crate::{
    args::{
        default_network, parse_matches, print_help_or_version, string_option, string_values,
        value_arg,
    },
    version_text,
};
use canic_host::{
    install_root::{
        clear_current_fleet_name_if_matches, discover_current_canic_config_choices,
        read_current_fleet_name, read_current_install_state, select_current_fleet_name,
    },
    release_set::{configured_fleet_name, configured_fleet_roles, workspace_root},
    table::WhitespaceTable,
};
use clap::{Arg, Command as ClapCommand};
use std::{
    ffi::OsString,
    fs,
    io::{self, BufRead, Write},
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

const CURRENT_HEADER: &str = "CURRENT";
const FLEET_HEADER: &str = "FLEET";
const NETWORK_HEADER: &str = "NETWORK";
const CONFIG_HEADER: &str = "CONFIG";
const CANISTERS_HEADER: &str = "CANISTERS";
const ROLE_PREVIEW_LIMIT: usize = 6;
const FLEET_HELP_AFTER: &str = "\
Examples:
  canic fleet
  canic fleet list
  canic fleet use demo
  canic fleet delete demo";
const FLEET_LIST_HELP_AFTER: &str = "\
Examples:
  canic fleet list
  canic fleet list --network local

Without --network, this command uses the current default network.";
const FLEET_USE_HELP_AFTER: &str = "\
Examples:
  canic fleet use demo
  canic fleet use staging --network local

Without --network, this command updates the current fleet for the current default network.";
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

    #[error("missing fleet name")]
    MissingFleetName,

    #[error("multiple fleet names provided")]
    ConflictingFleetName,

    #[error(
        "no current Canic fleet is selected for network {0}; run canic fleet list to inspect config-defined fleets, then canic fleet use <name>"
    )]
    NoCurrentFleet(String),

    #[error("no Canic fleet configs found under fleets; run canic scaffold <name>")]
    NoConfigChoices,

    #[error("unknown fleet {0}; run canic fleet list to inspect config-defined fleets")]
    UnknownFleet(String),

    #[error(
        "multiple configs declare fleet {0}; use distinct [fleet].name values before selecting it"
    )]
    DuplicateFleet(String),

    #[error("fleet delete cancelled")]
    DeleteCancelled,

    #[error("refusing to delete fleet {fleet}; target {target} is not under fleets")]
    UnsafeDeleteTarget { fleet: String, target: String },

    #[error("fleet {0} config does not have a parent directory")]
    MissingFleetDirectory(String),

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
}

///
/// UseFleetOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct UseFleetOptions {
    fleet: String,
    network: String,
}

///
/// DeleteFleetOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct DeleteFleetOptions {
    fleet: String,
}

///
/// FleetListRow
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct FleetListRow {
    current: String,
    fleet: String,
    network: String,
    config: String,
    canisters: String,
}

/// Run the fleet default command family.
pub fn run<I>(args: I) -> Result<(), FleetCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    let mut args = args.into_iter();
    match args
        .next()
        .and_then(|arg| arg.into_string().ok())
        .as_deref()
    {
        None => run_current(),
        Some("delete") => run_delete(args),
        Some("list") => run_list(args),
        Some("use") => run_use(args),
        _ => Err(FleetCommandError::Usage(usage())),
    }
}

// Print the current fleet name as a shell-friendly scalar value.
fn run_current() -> Result<(), FleetCommandError> {
    let network = default_network();
    let Some(fleet) = current_fleet_name(&network)? else {
        return Err(FleetCommandError::NoCurrentFleet(network));
    };
    println!("{fleet}");
    Ok(())
}

// Run the config-defined fleet listing subcommand.
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
    let choices = discover_current_canic_config_choices()?;
    if choices.is_empty() {
        return Err(FleetCommandError::NoConfigChoices);
    }
    println!(
        "{}",
        render_fleet_list(&workspace_root, &choices, &options.network)?
    );
    Ok(())
}

// Run the current fleet selection subcommand.
fn run_use<I>(args: I) -> Result<(), FleetCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, use_usage, version_text()) {
        return Ok(());
    }

    let options = UseFleetOptions::parse(args)?;
    ensure_config_fleet_exists(&options.fleet)?;
    select_current_fleet_name(&options.network, &options.fleet)?;
    println!("{}", options.fleet);
    Ok(())
}

// Run the destructive config-defined fleet deletion subcommand.
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
    let target = delete_target_dir(&workspace_root, &options.fleet)?;
    confirm_delete_fleet(&options.fleet, &target, io::stdin().lock(), io::stdout())?;
    fs::remove_dir_all(&target)?;
    let cleared_networks = clear_current_fleet_name_if_matches(&options.fleet)?;

    println!("Deleted Canic fleet:");
    println!("  fleet: {}", options.fleet);
    println!(
        "  path:  {}",
        display_workspace_path(&workspace_root, &target)
    );
    if !cleared_networks.is_empty() {
        println!(
            "  cleared current fleet for: {}",
            cleared_networks.join(", ")
        );
    }
    Ok(())
}

impl FleetOptions {
    // Parse fleet listing options.
    fn parse<I>(args: I) -> Result<Self, FleetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(fleet_list_command(), args)
            .map_err(|_| FleetCommandError::Usage(list_usage()))?;

        Ok(Self {
            network: string_option(&matches, "network").unwrap_or_else(default_network),
        })
    }
}

impl UseFleetOptions {
    // Parse current fleet selection options.
    fn parse<I>(args: I) -> Result<Self, FleetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(fleet_use_command(), args)
            .map_err(|_| FleetCommandError::Usage(use_usage()))?;
        let fleet_names = string_values(&matches, "fleet");
        let fleet = match fleet_names.as_slice() {
            [] => return Err(FleetCommandError::MissingFleetName),
            [fleet] => fleet.clone(),
            _ => return Err(FleetCommandError::ConflictingFleetName),
        };

        Ok(Self {
            fleet,
            network: string_option(&matches, "network").unwrap_or_else(default_network),
        })
    }
}

impl DeleteFleetOptions {
    // Parse fleet deletion options.
    fn parse<I>(args: I) -> Result<Self, FleetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(fleet_delete_command(), args)
            .map_err(|_| FleetCommandError::Usage(delete_usage()))?;
        let fleet_names = string_values(&matches, "fleet");
        let fleet = match fleet_names.as_slice() {
            [] => return Err(FleetCommandError::MissingFleetName),
            [fleet] => fleet.clone(),
            _ => return Err(FleetCommandError::ConflictingFleetName),
        };

        Ok(Self { fleet })
    }
}

// Return the current fleet name from installed state or an explicit selection.
fn current_fleet_name(network: &str) -> Result<Option<String>, FleetCommandError> {
    if let Some(state) = read_current_install_state(network)? {
        return Ok(Some(state.fleet));
    }

    Ok(read_current_fleet_name(network)?)
}

// Ensure a requested fleet is declared by exactly one install config.
fn ensure_config_fleet_exists(fleet: &str) -> Result<(), FleetCommandError> {
    let matches = discover_current_canic_config_choices()?
        .into_iter()
        .filter_map(|path| configured_fleet_name(&path).ok())
        .filter(|name| name == fleet)
        .count();

    match matches {
        0 => Err(FleetCommandError::UnknownFleet(fleet.to_string())),
        1 => Ok(()),
        _ => Err(FleetCommandError::DuplicateFleet(fleet.to_string())),
    }
}

// Resolve the directory that owns the selected fleet config.
fn delete_target_dir(workspace_root: &Path, fleet: &str) -> Result<PathBuf, FleetCommandError> {
    let choices = discover_current_canic_config_choices()?;
    delete_target_dir_from_choices(workspace_root, &choices, fleet)
}

// Resolve the target directory from pre-discovered config choices.
fn delete_target_dir_from_choices(
    workspace_root: &Path,
    choices: &[PathBuf],
    fleet: &str,
) -> Result<PathBuf, FleetCommandError> {
    let matches = choices
        .iter()
        .cloned()
        .filter_map(|path| match configured_fleet_name(&path) {
            Ok(name) if name == fleet => Some(path),
            Ok(_) | Err(_) => None,
        })
        .collect::<Vec<_>>();

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

    let Ok(root) = workspace_root.join("fleets").canonicalize() else {
        return false;
    };
    let Ok(target) = target.canonicalize() else {
        return false;
    };
    target != root && target.starts_with(root)
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

// Build the fleet command-family parser for help rendering.
fn fleet_command() -> ClapCommand {
    ClapCommand::new("fleet")
        .bin_name("canic fleet")
        .about("Show, list, select, or delete Canic fleets")
        .disable_help_flag(true)
        .subcommand(ClapCommand::new("list").about("List config-defined Canic fleets"))
        .subcommand(ClapCommand::new("use").about("Select the current Canic fleet"))
        .subcommand(ClapCommand::new("delete").about("Delete a config-defined Canic fleet"))
        .after_help(FLEET_HELP_AFTER)
}

// Build the fleet list parser.
fn fleet_list_command() -> ClapCommand {
    ClapCommand::new("list")
        .bin_name("canic fleet list")
        .about("List config-defined Canic fleets")
        .disable_help_flag(true)
        .arg(
            value_arg("network")
                .long("network")
                .value_name("name")
                .help("Network whose current fleet marker should be shown"),
        )
        .after_help(FLEET_LIST_HELP_AFTER)
}

// Build the current-fleet selection parser.
fn fleet_use_command() -> ClapCommand {
    ClapCommand::new("use")
        .bin_name("canic fleet use")
        .about("Select the current Canic fleet")
        .disable_help_flag(true)
        .arg(
            Arg::new("fleet")
                .num_args(0..=1)
                .value_name("name")
                .help("Config-defined fleet name to make current"),
        )
        .arg(
            value_arg("network")
                .long("network")
                .value_name("name")
                .help("Temporarily select a fleet for another DFX network"),
        )
        .after_help(FLEET_USE_HELP_AFTER)
}

// Build the fleet delete parser.
fn fleet_delete_command() -> ClapCommand {
    ClapCommand::new("delete")
        .bin_name("canic fleet delete")
        .about("Delete a config-defined Canic fleet directory")
        .disable_help_flag(true)
        .arg(
            Arg::new("fleet")
                .num_args(0..=1)
                .value_name("name")
                .help("Config-defined fleet name to delete"),
        )
        .after_help(FLEET_DELETE_HELP_AFTER)
}

// Render config-defined fleets as a compact whitespace table.
fn render_fleet_list(
    workspace_root: &Path,
    choices: &[PathBuf],
    network: &str,
) -> Result<String, FleetCommandError> {
    let current = current_fleet_name(network)?;
    let mut table = WhitespaceTable::new([
        CURRENT_HEADER,
        FLEET_HEADER,
        NETWORK_HEADER,
        CONFIG_HEADER,
        CANISTERS_HEADER,
    ]);
    for row in fleet_list_rows(workspace_root, choices, network, current.as_deref()) {
        table.push_row([
            row.current,
            row.fleet,
            row.network,
            row.config,
            row.canisters,
        ]);
    }
    Ok(table.render())
}

// Build operator-facing rows for config-defined fleets.
fn fleet_list_rows(
    workspace_root: &Path,
    choices: &[PathBuf],
    network: &str,
    current: Option<&str>,
) -> Vec<FleetListRow> {
    choices
        .iter()
        .map(|path| fleet_list_row(workspace_root, path, network, current))
        .collect()
}

// Build one operator-facing row for an installable config.
fn fleet_list_row(
    workspace_root: &Path,
    path: &Path,
    network: &str,
    current: Option<&str>,
) -> FleetListRow {
    let fleet = configured_fleet_name(path).unwrap_or_else(|_| "invalid config".to_string());
    FleetListRow {
        current: if current == Some(fleet.as_str()) {
            "*".to_string()
        } else {
            String::new()
        },
        network: network.to_string(),
        fleet,
        config: display_workspace_path(workspace_root, path),
        canisters: configured_fleet_roles(path).map_or_else(
            |_| "invalid config".to_string(),
            |roles| format_canister_summary(&roles),
        ),
    }
}

// Format the root-subnet canister count with a bounded role preview.
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

// Render a workspace-relative path where possible for concise output.
fn display_workspace_path(workspace_root: &Path, path: &Path) -> String {
    path.strip_prefix(workspace_root)
        .unwrap_or(path)
        .display()
        .to_string()
}

// Return fleet command-family usage text.
fn usage() -> String {
    let mut command = fleet_command();
    command.render_help().to_string()
}

// Return fleet list usage text.
fn list_usage() -> String {
    let mut command = fleet_list_command();
    command.render_help().to_string()
}

// Return fleet delete usage text.
fn delete_usage() -> String {
    let mut command = fleet_delete_command();
    command.render_help().to_string()
}

// Return fleet selection usage text.
fn use_usage() -> String {
    let mut command = fleet_use_command();
    command.render_help().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_dir;
    use std::fs;

    // Ensure fleet listing options accept network selection.
    #[test]
    fn parses_fleet_options() {
        let options = FleetOptions::parse([OsString::from("--network"), OsString::from("ic")])
            .expect("parse fleet options");

        assert_eq!(options.network, "ic");
    }

    // Ensure fleet use options require exactly one fleet name.
    #[test]
    fn parses_use_fleet_options() {
        let options = UseFleetOptions::parse([
            OsString::from("demo"),
            OsString::from("--network"),
            OsString::from("local"),
        ])
        .expect("parse use options");

        assert_eq!(options.fleet, "demo");
        assert_eq!(options.network, "local");
    }

    // Ensure fleet delete options require exactly one fleet name.
    #[test]
    fn parses_delete_fleet_options() {
        let options =
            DeleteFleetOptions::parse([OsString::from("demo")]).expect("parse delete options");

        assert_eq!(options.fleet, "demo");
    }

    // Ensure fleet deletion requires the exact fleet name as confirmation.
    #[test]
    fn confirm_delete_fleet_requires_exact_name() {
        let target = Path::new("/tmp/canic/fleets/demo");
        let mut output = Vec::new();

        confirm_delete_fleet("demo", target, io::Cursor::new(b"demo\n"), &mut output)
            .expect("confirm delete");

        let output = String::from_utf8(output).expect("utf8 prompt");
        assert!(output.contains("Delete Canic fleet?"));
        assert!(output.contains("fleet: demo"));
        assert!(output.contains("Type the fleet name to confirm"));

        let err = confirm_delete_fleet("demo", target, io::Cursor::new(b"yes\n"), Vec::new())
            .expect_err("wrong confirmation should cancel");
        assert!(matches!(err, FleetCommandError::DeleteCancelled));
    }

    // Ensure delete resolves the fleet config parent, not an arbitrary path.
    #[test]
    fn delete_target_resolves_config_parent() {
        let root = temp_dir("canic-fleet-delete-target");
        let demo = write_fleet_config(&root, "demo");
        let staging = write_fleet_config(&root, "staging");
        let choices = vec![demo.join("canic.toml"), staging.join("canic.toml")];

        let target =
            delete_target_dir_from_choices(&root, &choices, "staging").expect("delete target");

        fs::remove_dir_all(&root).expect("remove temp root");
        assert_eq!(target, staging);
    }

    // Ensure fleet listing renders deterministic config-defined rows.
    #[test]
    fn renders_fleet_list_table() {
        let table = render_fleet_list_from_rows(vec![
            FleetListRow {
                current: "*".to_string(),
                fleet: "demo".to_string(),
                network: "local".to_string(),
                config: "fleets/demo/canic.toml".to_string(),
                canisters: "3 (root, app, user_hub)".to_string(),
            },
            FleetListRow {
                current: String::new(),
                fleet: "staging".to_string(),
                network: "local".to_string(),
                config: "fleets/staging/canic.toml".to_string(),
                canisters: "2 (root, app)".to_string(),
            },
        ]);

        assert_eq!(
            table,
            format!(
                "{:<7}  {:<7}  {:<7}  {:<25}  {}\n{:<7}  {:<7}  {:<7}  {:<25}  {}\n{:<7}  {:<7}  {:<7}  {:<25}  {}",
                "CURRENT",
                "FLEET",
                "NETWORK",
                "CONFIG",
                "CANISTERS",
                "*",
                "demo",
                "local",
                "fleets/demo/canic.toml",
                "3 (root, app, user_hub)",
                "",
                "staging",
                "local",
                "fleets/staging/canic.toml",
                "2 (root, app)",
            )
        );
    }

    // Ensure fleet command help lists the command family without search.
    #[test]
    fn fleet_usage_lists_subcommands_and_examples() {
        let text = usage();

        assert!(text.contains("Show, list, select, or delete Canic fleets"));
        assert!(text.contains("Usage: canic fleet"));
        assert!(text.contains("delete"));
        assert!(text.contains("list"));
        assert!(text.contains("use"));
        assert!(!text.contains("search"));
        assert!(text.contains("Examples:"));
    }

    // Ensure fleet list help explains network selection.
    #[test]
    fn fleet_list_usage_lists_options_and_examples() {
        let text = list_usage();

        assert!(text.contains("List config-defined Canic fleets"));
        assert!(text.contains("Usage: canic fleet list"));
        assert!(text.contains("--network <name>"));
        assert!(text.contains("Examples:"));
    }

    // Ensure fleet delete help explains the destructive confirmation.
    #[test]
    fn delete_usage_lists_confirmation() {
        let text = delete_usage();

        assert!(text.contains("Delete a config-defined Canic fleet directory"));
        assert!(text.contains("Usage: canic fleet delete"));
        assert!(text.contains("[name]"));
        assert!(text.contains("type the"));
    }

    // Ensure current-fleet help renders the singular fleet argument.
    #[test]
    fn use_usage_lists_singular_fleet_argument() {
        let text = use_usage();

        assert!(text.contains("Select the current Canic fleet"));
        assert!(text.contains("Usage: canic fleet use"));
        assert!(text.contains("[name]"));
        assert!(!text.contains("[name]..."));
    }

    // Render precomputed config rows for focused table tests.
    fn render_fleet_list_from_rows(rows: Vec<FleetListRow>) -> String {
        let mut table = WhitespaceTable::new([
            CURRENT_HEADER,
            FLEET_HEADER,
            NETWORK_HEADER,
            CONFIG_HEADER,
            CANISTERS_HEADER,
        ]);
        for row in rows {
            table.push_row([
                row.current,
                row.fleet,
                row.network,
                row.config,
                row.canisters,
            ]);
        }
        table.render()
    }

    fn write_fleet_config(root: &Path, name: &str) -> PathBuf {
        let dir = root.join("fleets").join(name);
        fs::create_dir_all(dir.join("root")).expect("create root dir");
        fs::write(dir.join("root/Cargo.toml"), "").expect("write root manifest");
        fs::write(
            dir.join("canic.toml"),
            format!(
                r#"
[fleet]
name = "{name}"

[subnets.prime.canisters.root]
kind = "root"
"#
            ),
        )
        .expect("write canic config");
        dir
    }
}
