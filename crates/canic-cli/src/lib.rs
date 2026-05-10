mod args;
mod backup;
mod build;
mod endpoints;
mod fleets;
mod install;
mod list;
mod manifest;
mod medic;
mod output;
mod replica;
mod restore;
mod scaffold;
mod snapshot;
mod status;
#[cfg(test)]
mod test_support;

use crate::args::{
    INTERNAL_ICP_OPTION, INTERNAL_NETWORK_OPTION, first_arg_is_help, icp_arg, network_arg,
    parse_matches,
};
use clap::{Arg, ArgAction, Command};
use std::ffi::OsString;
use thiserror::Error as ThisError;

const VERSION_TEXT: &str = concat!("canic ", env!("CARGO_PKG_VERSION"));
const TOP_LEVEL_HELP_TEMPLATE: &str = "{name} {version}\n{about-with-newline}\n{usage-heading} {usage}\n\n{before-help}Options:\n{options}{after-help}\n";
const COLOR_RESET: &str = "\x1b[0m";
const COLOR_HEADING: &str = "\x1b[1m";
const COLOR_GROUP: &str = "\x1b[38;5;245m";
const COLOR_COMMAND: &str = "\x1b[38;5;109m";
const COLOR_TIP: &str = "\x1b[38;5;245m";
const DISPATCH_ARGS: &str = "args";

///
/// CommandScope
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CommandScope {
    Global,
    FleetContext,
    WorkspaceFiles,
}

impl CommandScope {
    const fn heading(self) -> &'static str {
        match self {
            Self::Global => "Global commands",
            Self::FleetContext => "Fleet commands",
            Self::WorkspaceFiles => "Workspace and file commands",
        }
    }
}

///
/// CommandSpec
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CommandSpec {
    name: &'static str,
    about: &'static str,
    scope: CommandScope,
}

const COMMAND_SPECS: &[CommandSpec] = &[
    CommandSpec {
        name: "status",
        about: "Show quick Canic project status",
        scope: CommandScope::Global,
    },
    CommandSpec {
        name: "fleet",
        about: "Manage Canic fleets",
        scope: CommandScope::Global,
    },
    CommandSpec {
        name: "replica",
        about: "Manage the local ICP replica",
        scope: CommandScope::Global,
    },
    CommandSpec {
        name: "install",
        about: "Install and bootstrap a Canic fleet",
        scope: CommandScope::FleetContext,
    },
    CommandSpec {
        name: "config",
        about: "Inspect selected fleet config",
        scope: CommandScope::FleetContext,
    },
    CommandSpec {
        name: "list",
        about: "List deployed fleet canisters",
        scope: CommandScope::FleetContext,
    },
    CommandSpec {
        name: "endpoints",
        about: "List canister Candid endpoints",
        scope: CommandScope::FleetContext,
    },
    CommandSpec {
        name: "medic",
        about: "Diagnose local Canic fleet setup",
        scope: CommandScope::FleetContext,
    },
    CommandSpec {
        name: "snapshot",
        about: "Capture and download canister snapshots",
        scope: CommandScope::FleetContext,
    },
    CommandSpec {
        name: "build",
        about: "Build one Canic canister artifact",
        scope: CommandScope::WorkspaceFiles,
    },
    CommandSpec {
        name: "backup",
        about: "Verify backup directories and journal status",
        scope: CommandScope::WorkspaceFiles,
    },
    CommandSpec {
        name: "manifest",
        about: "Validate fleet backup manifests",
        scope: CommandScope::WorkspaceFiles,
    },
    CommandSpec {
        name: "restore",
        about: "Plan or run snapshot restores",
        scope: CommandScope::WorkspaceFiles,
    },
];

///
/// CliError
///

#[derive(Debug, ThisError)]
pub enum CliError {
    #[error("{0}")]
    Usage(String),

    #[error("backup: {0}")]
    Backup(String),

    #[error("build: {0}")]
    Build(String),

    #[error("config: {0}")]
    Config(String),

    #[error("endpoints: {0}")]
    Endpoints(String),

    #[error("install: {0}")]
    Install(String),

    #[error("fleet: {0}")]
    Fleets(String),

    #[error("list: {0}")]
    List(String),

    #[error("manifest: {0}")]
    Manifest(String),

    #[error("medic: {0}")]
    Medic(String),

    #[error("snapshot: {0}")]
    Snapshot(String),

    #[error("restore: {0}")]
    Restore(String),

    #[error("replica: {0}")]
    Replica(String),

    #[error("status: {0}")]
    Status(String),
}

impl From<backup::BackupCommandError> for CliError {
    fn from(err: backup::BackupCommandError) -> Self {
        Self::Backup(err.to_string())
    }
}

impl From<build::BuildCommandError> for CliError {
    fn from(err: build::BuildCommandError) -> Self {
        Self::Build(err.to_string())
    }
}

impl From<endpoints::EndpointsCommandError> for CliError {
    fn from(err: endpoints::EndpointsCommandError) -> Self {
        Self::Endpoints(err.to_string())
    }
}

impl From<install::InstallCommandError> for CliError {
    fn from(err: install::InstallCommandError) -> Self {
        Self::Install(err.to_string())
    }
}

impl From<fleets::FleetCommandError> for CliError {
    fn from(err: fleets::FleetCommandError) -> Self {
        Self::Fleets(err.to_string())
    }
}

impl From<list::ListCommandError> for CliError {
    fn from(err: list::ListCommandError) -> Self {
        Self::List(err.to_string())
    }
}

impl From<manifest::ManifestCommandError> for CliError {
    fn from(err: manifest::ManifestCommandError) -> Self {
        Self::Manifest(err.to_string())
    }
}

impl From<medic::MedicCommandError> for CliError {
    fn from(err: medic::MedicCommandError) -> Self {
        Self::Medic(err.to_string())
    }
}

impl From<snapshot::SnapshotCommandError> for CliError {
    fn from(err: snapshot::SnapshotCommandError) -> Self {
        Self::Snapshot(err.to_string())
    }
}

impl From<restore::RestoreCommandError> for CliError {
    fn from(err: restore::RestoreCommandError) -> Self {
        Self::Restore(err.to_string())
    }
}

impl From<replica::ReplicaCommandError> for CliError {
    fn from(err: replica::ReplicaCommandError) -> Self {
        Self::Replica(err.to_string())
    }
}

impl From<status::StatusCommandError> for CliError {
    fn from(err: status::StatusCommandError) -> Self {
        Self::Status(err.to_string())
    }
}

/// Run the CLI from process arguments.
pub fn run_from_env() -> Result<(), CliError> {
    run(std::env::args_os().skip(1))
}

/// Run the CLI from an argument iterator.
pub fn run<I>(args: I) -> Result<(), CliError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if first_arg_is_help(&args) {
        println!("{}", usage());
        return Ok(());
    }
    if let Some(option) = command_local_global_option(&args) {
        return Err(CliError::Usage(format!(
            "{option} is a top-level option; put it before the command\n\n{}",
            usage()
        )));
    }

    let matches =
        parse_matches(top_level_dispatch_command(), args).map_err(|_| CliError::Usage(usage()))?;
    if matches.get_flag("version") {
        println!("{}", version_text());
        return Ok(());
    }
    let global_icp = matches.get_one::<String>("icp").cloned();
    let global_network = matches.get_one::<String>("network").cloned();

    let Some((command, subcommand_matches)) = matches.subcommand() else {
        return Err(CliError::Usage(usage()));
    };
    let mut tail = subcommand_matches
        .get_many::<OsString>(DISPATCH_ARGS)
        .map(|values| values.cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    apply_global_icp(command, &mut tail, global_icp);
    apply_global_network(command, &mut tail, global_network);
    let tail = tail.into_iter();

    match command {
        "backup" => backup::run(tail).map_err(CliError::from),
        "build" => build::run(tail).map_err(CliError::from),
        "config" => list::run_config(tail).map_err(|err| CliError::Config(err.to_string())),
        "endpoints" => endpoints::run(tail).map_err(CliError::from),
        "fleet" => fleets::run(tail).map_err(CliError::from),
        "install" => install::run(tail).map_err(CliError::from),
        "list" => list::run(tail).map_err(CliError::from),
        "manifest" => manifest::run(tail).map_err(CliError::from),
        "medic" => medic::run(tail).map_err(CliError::from),
        "replica" => replica::run(tail).map_err(CliError::from),
        "snapshot" => snapshot::run(tail).map_err(CliError::from),
        "status" => status::run(tail).map_err(CliError::from),
        "restore" => restore::run(tail).map_err(CliError::from),
        _ => unreachable!("top-level dispatch command only defines known commands"),
    }
}

#[must_use]
pub fn top_level_command() -> Command {
    let command = Command::new("canic")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Operator CLI for Canic install, backup, and restore workflows")
        .disable_version_flag(true)
        .arg(
            Arg::new("version")
                .short('V')
                .long("version")
                .action(ArgAction::SetTrue)
                .help("Print version"),
        )
        .arg(icp_arg().global(true))
        .arg(network_arg().global(true))
        .subcommand_help_heading("Commands")
        .help_template(TOP_LEVEL_HELP_TEMPLATE)
        .before_help(grouped_command_section(COMMAND_SPECS).join("\n"))
        .after_help("Run `canic <command> help` for command-specific help.");

    COMMAND_SPECS.iter().fold(command, |command, spec| {
        command.subcommand(Command::new(spec.name).about(spec.about))
    })
}

fn top_level_dispatch_command() -> Command {
    let command = Command::new("canic")
        .disable_help_flag(true)
        .disable_version_flag(true)
        .arg(
            Arg::new("version")
                .short('V')
                .long("version")
                .action(ArgAction::SetTrue),
        );
    let command = command
        .arg(icp_arg().global(true))
        .arg(network_arg().global(true));

    COMMAND_SPECS.iter().fold(command, |command, spec| {
        command.subcommand(
            Command::new(spec.name).arg(
                Arg::new(DISPATCH_ARGS)
                    .num_args(0..)
                    .allow_hyphen_values(true)
                    .trailing_var_arg(true)
                    .value_parser(clap::value_parser!(OsString)),
            ),
        )
    })
}

fn command_local_global_option(args: &[OsString]) -> Option<&'static str> {
    let mut index = 0;
    while index < args.len() {
        let arg = args[index].to_str()?;
        if COMMAND_SPECS.iter().any(|spec| spec.name == arg) {
            return args[index + 1..]
                .iter()
                .filter_map(|arg| arg.to_str())
                .find_map(global_option_name);
        }
        index += if matches!(arg, "--icp" | "--network") {
            2
        } else {
            1
        };
    }
    None
}

fn global_option_name(arg: &str) -> Option<&'static str> {
    match arg {
        "--icp" => Some("--icp"),
        "--network" => Some("--network"),
        _ if arg.starts_with("--icp=") => Some("--icp"),
        _ if arg.starts_with("--network=") => Some("--network"),
        _ => None,
    }
}

fn apply_global_icp(command: &str, tail: &mut Vec<OsString>, global_icp: Option<String>) {
    let Some(global_icp) = global_icp else {
        return;
    };
    if tail_has_option(tail, INTERNAL_ICP_OPTION) {
        return;
    }
    if !command_accepts_global_icp(command, tail) {
        return;
    }

    tail.push(OsString::from(INTERNAL_ICP_OPTION));
    tail.push(OsString::from(global_icp));
}

fn apply_global_network(command: &str, tail: &mut Vec<OsString>, global_network: Option<String>) {
    let Some(global_network) = global_network else {
        return;
    };
    if tail_has_option(tail, INTERNAL_NETWORK_OPTION) {
        return;
    }
    if !command_accepts_global_network(command, tail) {
        return;
    }

    tail.push(OsString::from(INTERNAL_NETWORK_OPTION));
    tail.push(OsString::from(global_network));
}

fn command_accepts_global_icp(command: &str, tail: &[OsString]) -> bool {
    match command {
        "endpoints" | "list" | "medic" | "status" => true,
        "replica" => matches!(
            tail.first().and_then(|arg| arg.to_str()),
            Some("start" | "status" | "stop")
        ),
        "snapshot" => tail.first().and_then(|arg| arg.to_str()) == Some("download"),
        "restore" => tail.first().and_then(|arg| arg.to_str()) == Some("run"),
        _ => false,
    }
}

fn command_accepts_global_network(command: &str, tail: &[OsString]) -> bool {
    match command {
        "endpoints" | "install" | "list" | "medic" | "status" => true,
        "fleet" => tail.first().and_then(|arg| arg.to_str()) == Some("list"),
        "snapshot" => tail.first().and_then(|arg| arg.to_str()) == Some("download"),
        "restore" => tail.first().and_then(|arg| arg.to_str()) == Some("run"),
        _ => false,
    }
}

fn tail_has_option(tail: &[OsString], name: &str) -> bool {
    tail.iter().any(|arg| arg.to_str() == Some(name))
}

#[must_use]
pub const fn version_text() -> &'static str {
    VERSION_TEXT
}

fn usage() -> String {
    let mut lines = vec![
        color(
            COLOR_HEADING,
            &format!("Canic Operator CLI v{}", env!("CARGO_PKG_VERSION")),
        ),
        String::new(),
        "Usage: canic [OPTIONS] <COMMAND>".to_string(),
        String::new(),
        color(COLOR_HEADING, "Commands:"),
    ];
    lines.extend(grouped_command_section(COMMAND_SPECS));
    lines.extend([
        String::new(),
        color(COLOR_HEADING, "Options:"),
        "      --icp <path>      Path to the icp executable for ICP-backed commands".to_string(),
        "      --network <name>  ICP CLI network for networked commands".to_string(),
        "  -V, --version  Print version".to_string(),
        "  -h, --help     Print help".to_string(),
        String::new(),
        format!(
            "{}Tip:{} Run {} for command-specific help.",
            COLOR_TIP,
            COLOR_RESET,
            color(COLOR_COMMAND, "`canic <command> help`")
        ),
    ]);
    lines.join("\n")
}

fn grouped_command_section(specs: &[CommandSpec]) -> Vec<String> {
    let mut lines = Vec::new();
    let scopes = [
        CommandScope::Global,
        CommandScope::FleetContext,
        CommandScope::WorkspaceFiles,
    ];
    for (index, scope) in scopes.into_iter().enumerate() {
        lines.push(format!("  {}", color(COLOR_GROUP, scope.heading())));
        for spec in specs.iter().filter(|spec| spec.scope == scope) {
            let command = format!("{:<12}", spec.name);
            lines.push(format!(
                "    {} {}",
                color(COLOR_COMMAND, &command),
                spec.about
            ));
        }
        if index + 1 < scopes.len() {
            lines.push(String::new());
        }
    }
    lines
}

fn color(code: &str, text: &str) -> String {
    format!("{code}{text}{COLOR_RESET}")
}

#[cfg(test)]
mod tests {
    use super::*;

    // Ensure top-level help stays compact as command surfaces grow.
    #[test]
    fn usage_lists_command_families() {
        let text = usage();
        let plain = strip_ansi(&text);

        assert!(plain.contains(&format!(
            "Canic Operator CLI v{}",
            env!("CARGO_PKG_VERSION")
        )));
        assert!(plain.contains("Usage: canic [OPTIONS] <COMMAND>"));
        assert!(plain.contains("\nCommands:\n"));
        assert!(plain.contains("Global commands"));
        assert!(plain.contains("Fleet commands"));
        assert!(plain.contains("Workspace and file commands"));
        assert!(plain.find("    status") < plain.find("    fleet"));
        assert!(plain.find("    fleet") < plain.find("    replica"));
        assert!(plain.find("    replica") < plain.find("    install"));
        assert!(plain.find("    install") < plain.find("    config"));
        assert!(plain.find("    config") < plain.find("    list"));
        assert!(plain.find("    list") < plain.find("    endpoints"));
        assert!(plain.contains("Options:"));
        assert!(plain.contains("--icp <path>"));
        assert!(plain.contains("--network <name>"));
        assert!(!plain.contains("    scaffold"));
        assert!(plain.contains("config"));
        assert!(plain.contains("list"));
        assert!(plain.contains("endpoints"));
        assert!(plain.contains("build"));
        assert!(!plain.contains("    network"));
        assert!(!plain.contains("    defaults"));
        assert!(plain.contains("    status"));
        assert!(plain.contains("fleet"));
        assert!(plain.contains("replica"));
        assert!(plain.contains("install"));
        assert!(plain.contains("snapshot"));
        assert!(plain.contains("backup"));
        assert!(plain.contains("manifest"));
        assert!(plain.contains("medic"));
        assert!(plain.contains("restore"));
        assert!(plain.contains("Tip: Run `canic <command> help`"));
        assert!(text.contains(COLOR_HEADING));
        assert!(text.contains(COLOR_GROUP));
        assert!(text.contains(COLOR_COMMAND));
    }

    // Ensure command-family help paths return successfully instead of erroring.
    #[test]
    fn command_family_help_returns_ok() {
        for args in [
            &["backup", "help"][..],
            &["backup", "list", "help"],
            &["backup", "status", "help"],
            &["backup", "verify", "help"],
            &["build", "help"],
            &["config", "help"],
            &["endpoints", "help"],
            &["install", "help"],
            &["fleet"],
            &["fleet", "help"],
            &["fleet", "create", "help"],
            &["fleet", "list", "help"],
            &["fleet", "delete", "help"],
            &["replica"],
            &["replica", "help"],
            &["replica", "start", "help"],
            &["replica", "status", "help"],
            &["replica", "stop", "help"],
            &["list", "help"],
            &["restore", "help"],
            &["restore", "plan", "help"],
            &["restore", "apply", "help"],
            &["restore", "run", "help"],
            &["manifest", "help"],
            &["manifest", "validate", "help"],
            &["medic", "help"],
            &["snapshot", "help"],
            &["snapshot", "download", "help"],
            &["status", "help"],
        ] {
            assert_run_ok(args);
        }
    }

    // Ensure version flags are accepted at the top level and command-family level.
    #[test]
    fn version_flags_return_ok() {
        assert_eq!(version_text(), concat!("canic ", env!("CARGO_PKG_VERSION")));
        assert!(run([OsString::from("--version")]).is_ok());
        assert!(
            run([
                OsString::from("backup"),
                OsString::from("list"),
                OsString::from("--dir"),
                OsString::from("version")
            ])
            .is_ok()
        );
        assert!(run([OsString::from("backup"), OsString::from("--version")]).is_ok());
        assert!(
            run([
                OsString::from("backup"),
                OsString::from("list"),
                OsString::from("--version")
            ])
            .is_ok()
        );
        assert!(run([OsString::from("build"), OsString::from("--version")]).is_ok());
        assert!(run([OsString::from("config"), OsString::from("--version")]).is_ok());
        assert!(run([OsString::from("endpoints"), OsString::from("--version")]).is_ok());
        assert!(run([OsString::from("install"), OsString::from("--version")]).is_ok());
        assert!(run([OsString::from("fleet"), OsString::from("--version")]).is_ok());
        assert!(run([OsString::from("replica"), OsString::from("--version")]).is_ok());
        assert!(run([OsString::from("status"), OsString::from("--version")]).is_ok());
        assert!(
            run([
                OsString::from("fleet"),
                OsString::from("create"),
                OsString::from("--version")
            ])
            .is_ok()
        );
        assert!(
            run([
                OsString::from("replica"),
                OsString::from("start"),
                OsString::from("--version")
            ])
            .is_ok()
        );
        assert!(run([OsString::from("list"), OsString::from("--version")]).is_ok());
        assert!(run([OsString::from("restore"), OsString::from("--version")]).is_ok());
        assert!(run([OsString::from("manifest"), OsString::from("--version")]).is_ok());
        assert!(run([OsString::from("medic"), OsString::from("--version")]).is_ok());
        assert!(run([OsString::from("snapshot"), OsString::from("--version")]).is_ok());
        assert!(
            run([
                OsString::from("snapshot"),
                OsString::from("download"),
                OsString::from("--version")
            ])
            .is_ok()
        );
    }

    #[test]
    fn global_icp_is_forwarded_to_commands_that_use_icp() {
        let mut tail = vec![OsString::from("test")];

        apply_global_icp("medic", &mut tail, Some("/tmp/icp".to_string()));

        assert_eq!(
            tail,
            vec![
                OsString::from("test"),
                OsString::from(INTERNAL_ICP_OPTION),
                OsString::from("/tmp/icp")
            ]
        );
    }

    #[test]
    fn global_icp_does_not_override_internal_forwarded_icp() {
        let mut tail = vec![
            OsString::from("test"),
            OsString::from(INTERNAL_ICP_OPTION),
            OsString::from("/bin/icp"),
        ];

        apply_global_icp("medic", &mut tail, Some("/tmp/icp".to_string()));

        assert_eq!(
            tail,
            vec![
                OsString::from("test"),
                OsString::from(INTERNAL_ICP_OPTION),
                OsString::from("/bin/icp")
            ]
        );
    }

    #[test]
    fn global_icp_is_forwarded_only_to_restore_run() {
        let mut plan_tail = vec![OsString::from("plan")];
        let mut run_tail = vec![OsString::from("run")];

        apply_global_icp("restore", &mut plan_tail, Some("/tmp/icp".to_string()));
        apply_global_icp("restore", &mut run_tail, Some("/tmp/icp".to_string()));

        assert_eq!(plan_tail, vec![OsString::from("plan")]);
        assert_eq!(
            run_tail,
            vec![
                OsString::from("run"),
                OsString::from(INTERNAL_ICP_OPTION),
                OsString::from("/tmp/icp")
            ]
        );
    }

    #[test]
    fn global_icp_is_forwarded_only_to_replica_leaf_commands() {
        let mut family_tail = Vec::new();
        let mut start_tail = vec![OsString::from("start")];

        apply_global_icp("replica", &mut family_tail, Some("/tmp/icp".to_string()));
        apply_global_icp("replica", &mut start_tail, Some("/tmp/icp".to_string()));

        assert!(family_tail.is_empty());
        assert_eq!(
            start_tail,
            vec![
                OsString::from("start"),
                OsString::from(INTERNAL_ICP_OPTION),
                OsString::from("/tmp/icp")
            ]
        );
    }

    #[test]
    fn global_network_is_forwarded_to_commands_that_use_network() {
        let mut tail = vec![OsString::from("test")];

        apply_global_network("install", &mut tail, Some("ic".to_string()));

        assert_eq!(
            tail,
            vec![
                OsString::from("test"),
                OsString::from(INTERNAL_NETWORK_OPTION),
                OsString::from("ic")
            ]
        );
    }

    #[test]
    fn global_network_does_not_override_internal_forwarded_network() {
        let mut tail = vec![
            OsString::from("test"),
            OsString::from(INTERNAL_NETWORK_OPTION),
            OsString::from("local"),
        ];

        apply_global_network("install", &mut tail, Some("ic".to_string()));

        assert_eq!(
            tail,
            vec![
                OsString::from("test"),
                OsString::from(INTERNAL_NETWORK_OPTION),
                OsString::from("local")
            ]
        );
    }

    #[test]
    fn global_network_is_forwarded_only_to_restore_run() {
        let mut plan_tail = vec![OsString::from("plan")];
        let mut run_tail = vec![OsString::from("run")];

        apply_global_network("restore", &mut plan_tail, Some("ic".to_string()));
        apply_global_network("restore", &mut run_tail, Some("ic".to_string()));

        assert_eq!(plan_tail, vec![OsString::from("plan")]);
        assert_eq!(
            run_tail,
            vec![
                OsString::from("run"),
                OsString::from(INTERNAL_NETWORK_OPTION),
                OsString::from("ic")
            ]
        );
    }

    #[test]
    fn global_network_is_forwarded_only_to_fleet_list() {
        let mut create_tail = vec![OsString::from("create")];
        let mut list_tail = vec![OsString::from("list")];

        apply_global_network("fleet", &mut create_tail, Some("local".to_string()));
        apply_global_network("fleet", &mut list_tail, Some("local".to_string()));

        assert_eq!(create_tail, vec![OsString::from("create")]);
        assert_eq!(
            list_tail,
            vec![
                OsString::from("list"),
                OsString::from(INTERNAL_NETWORK_OPTION),
                OsString::from("local")
            ]
        );
    }

    #[test]
    fn command_local_global_options_are_hard_rejected() {
        assert!(matches!(
            run([
                OsString::from("status"),
                OsString::from("--network"),
                OsString::from("local")
            ]),
            Err(CliError::Usage(_))
        ));
        assert!(matches!(
            run([
                OsString::from("medic"),
                OsString::from("test"),
                OsString::from("--icp"),
                OsString::from("icp")
            ]),
            Err(CliError::Usage(_))
        ));
    }

    // Remove ANSI color sequences so tests can assert help structure.
    fn strip_ansi(text: &str) -> String {
        let mut plain = String::new();
        let mut chars = text.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '\x1b' && chars.peek() == Some(&'[') {
                chars.next();
                for ch in chars.by_ref() {
                    if ch == 'm' {
                        break;
                    }
                }
                continue;
            }
            plain.push(ch);
        }
        plain
    }

    // Assert that a CLI argv slice returns successfully.
    fn assert_run_ok(raw_args: &[&str]) {
        let args = raw_args.iter().map(OsString::from).collect::<Vec<_>>();
        assert!(
            run(args).is_ok(),
            "expected successful run for {raw_args:?}"
        );
    }
}
