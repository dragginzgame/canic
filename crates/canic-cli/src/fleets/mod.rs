use crate::{
    args::{
        local_network, parse_matches, parse_subcommand, passthrough_subcommand,
        print_help_or_version, string_option, value_arg,
    },
    scaffold, version_text,
};
use canic_host::{
    install_root::discover_current_canic_config_choices,
    release_set::{
        configured_fleet_name, configured_fleet_roles, display_workspace_path,
        matching_fleet_config_paths, workspace_root,
    },
    table::WhitespaceTable,
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
  canic fleet delete demo";
const FLEET_LIST_HELP_AFTER: &str = "\
Examples:
  canic fleet list
  canic fleet list --network local

Commands that operate on one fleet take the fleet name as a positional argument.";
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

    #[error("refusing to delete fleet {fleet}; target {target} is not under fleets")]
    UnsafeDeleteTarget { fleet: String, target: String },

    #[error("fleet {0} config does not have a parent directory")]
    MissingFleetDirectory(String),

    #[error("fleet create: {0}")]
    Create(String),

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
    let choices = discover_current_canic_config_choices()?;
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
    let target = delete_target_dir(&workspace_root, &options.fleet)?;
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

impl FleetOptions {
    fn parse<I>(args: I) -> Result<Self, FleetCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(fleet_list_command(), args)
            .map_err(|_| FleetCommandError::Usage(list_usage()))?;

        Ok(Self {
            network: string_option(&matches, "network").unwrap_or_else(local_network),
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
        })
    }
}

fn delete_target_dir(workspace_root: &Path, fleet: &str) -> Result<PathBuf, FleetCommandError> {
    let choices = discover_current_canic_config_choices()?;
    delete_target_dir_from_choices(workspace_root, &choices, fleet)
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
        .arg(
            value_arg("network")
                .long("network")
                .value_name("name")
                .help("Network to show in the fleet list"),
        )
        .after_help(FLEET_LIST_HELP_AFTER)
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
        .after_help(FLEET_DELETE_HELP_AFTER)
}

fn render_fleet_list(workspace_root: &Path, choices: &[PathBuf], network: &str) -> String {
    let mut table = WhitespaceTable::new([
        FLEET_HEADER,
        NETWORK_HEADER,
        CONFIG_HEADER,
        CANISTERS_HEADER,
    ]);
    for row in fleet_list_rows(workspace_root, choices, network) {
        table.push_row([row.fleet, row.network, row.config, row.canisters]);
    }
    table.render()
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

fn usage() -> String {
    let mut command = fleet_command();
    command.render_help().to_string()
}

fn list_usage() -> String {
    let mut command = fleet_list_command();
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
                fleet: "demo".to_string(),
                network: "local".to_string(),
                config: "fleets/demo/canic.toml".to_string(),
                canisters: "3 (root, app, user_hub)".to_string(),
            },
            FleetListRow {
                fleet: "staging".to_string(),
                network: "local".to_string(),
                config: "fleets/staging/canic.toml".to_string(),
                canisters: "2 (root, app)".to_string(),
            },
        ]);

        assert_eq!(
            table,
            format!(
                "{:<7}  {:<7}  {:<25}  {}\n{:<7}  {:<7}  {:<25}  {}\n{:<7}  {:<7}  {:<25}  {}",
                "FLEET",
                "NETWORK",
                "CONFIG",
                "CANISTERS",
                "demo",
                "local",
                "fleets/demo/canic.toml",
                "3 (root, app, user_hub)",
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

        assert!(text.contains("Manage Canic fleets"));
        assert!(text.contains("Usage: canic fleet"));
        assert!(text.contains("create"));
        assert!(text.contains("delete"));
        assert!(text.contains("list"));
        assert!(!text.contains("current"));
        assert!(!text.contains("use"));
        assert!(!text.contains("search"));
        assert!(text.contains("Examples:"));
    }

    // Ensure fleet create help explains creation.
    #[test]
    fn fleet_create_usage_lists_options_and_examples() {
        let text = create_usage();

        assert!(text.contains("Create a minimal Canic fleet"));
        assert!(text.contains("Usage: canic fleet create"));
        assert!(!text.contains("--network <name>"));
        assert!(text.contains("--yes"));
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
        assert!(text.contains("<name>"));
        assert!(text.contains("type the"));
    }

    // Render precomputed config rows for focused table tests.
    fn render_fleet_list_from_rows(rows: Vec<FleetListRow>) -> String {
        let mut table = WhitespaceTable::new([
            FLEET_HEADER,
            NETWORK_HEADER,
            CONFIG_HEADER,
            CANISTERS_HEADER,
        ]);
        for row in rows {
            table.push_row([row.fleet, row.network, row.config, row.canisters]);
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
