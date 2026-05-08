use super::ListCommandError;
use crate::args::{default_icp, flag_arg, parse_matches, value_arg};
use clap::Command as ClapCommand;
use std::ffi::OsString;

const LIST_HELP_AFTER: &str = "\
Examples:
  canic list test
  canic list test --from user_hub
  canic list test --root uzt4z-lp777-77774-qaabq-cai";
const CONFIG_HELP_AFTER: &str = "\
Examples:
  canic config test
  canic config test --from user_hub
  canic config test --verbose";

///
/// ListOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ListOptions {
    pub(super) source: ListSource,
    pub(super) fleet: String,
    pub(super) root: Option<String>,
    pub(super) anchor: Option<String>,
    pub(super) network: Option<String>,
    pub(super) icp: String,
    pub(super) verbose: bool,
}

impl ListOptions {
    /// Parse deployed root registry listing options from CLI arguments.
    pub(super) fn parse_list<I>(args: I) -> Result<Self, ListCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches =
            parse_matches(list_command(), args).map_err(|_| ListCommandError::Usage(usage()))?;
        Ok(Self::from_matches(&matches, ListSource::RootRegistry))
    }

    /// Parse declared fleet config listing options from CLI arguments.
    pub(super) fn parse_config<I>(args: I) -> Result<Self, ListCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(config_command(), args)
            .map_err(|_| ListCommandError::Usage(config_usage()))?;
        Ok(Self::from_matches(&matches, ListSource::Config))
    }

    // Build one option set from subcommand-specific matches.
    fn from_matches(matches: &clap::ArgMatches, source: ListSource) -> Self {
        Self {
            source,
            fleet: optional_string(matches, "fleet").expect("clap requires fleet"),
            root: optional_string(matches, "root"),
            anchor: optional_string(matches, "from"),
            network: optional_string(matches, "network"),
            icp: optional_string(matches, "icp").unwrap_or_else(default_icp),
            verbose: optional_bool(matches, "verbose"),
        }
    }
}

// Read a string option if the subcommand declares it.
fn optional_string(matches: &clap::ArgMatches, id: &str) -> Option<String> {
    matches.try_get_one::<String>(id).ok().flatten().cloned()
}

// Read a boolean flag if the subcommand declares it.
fn optional_bool(matches: &clap::ArgMatches, id: &str) -> bool {
    matches
        .try_get_one::<bool>(id)
        .ok()
        .flatten()
        .copied()
        .unwrap_or(false)
}

///
/// ListSource
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ListSource {
    Config,
    RootRegistry,
}

// Build the list parser.
fn list_command() -> ClapCommand {
    base_list_options(ClapCommand::new("list").bin_name("canic list"))
        .about("List canisters registered by the deployed root")
        .arg(
            value_arg("fleet")
                .value_name("fleet")
                .required(true)
                .help("Fleet name to inspect"),
        )
        .arg(
            value_arg("root")
                .long("root")
                .value_name("name-or-principal")
                .help("Query a specific root canister registry"),
        )
        .arg(
            value_arg("from")
                .long("from")
                .value_name("name-or-principal")
                .help("Render a subtree anchored at one canister"),
        )
        .arg(
            value_arg("icp")
                .long("icp")
                .value_name("path")
                .help("Path to the icp executable"),
        )
        .after_help(LIST_HELP_AFTER)
}

// Build the selected fleet config parser.
fn config_command() -> ClapCommand {
    base_list_options(ClapCommand::new("config").bin_name("canic config"))
        .about("List canisters declared by the selected fleet config")
        .arg(
            value_arg("fleet")
                .value_name("fleet")
                .required(true)
                .help("Fleet name to inspect"),
        )
        .arg(
            value_arg("from")
                .long("from")
                .value_name("role")
                .help("Show one declared role"),
        )
        .arg(
            flag_arg("verbose")
                .long("verbose")
                .help("Show indented declared config details under each role"),
        )
        .after_help(CONFIG_HELP_AFTER)
}

// Add options shared by all list subcommands.
fn base_list_options(command: ClapCommand) -> ClapCommand {
    command.disable_help_flag(true).arg(
        value_arg("network")
            .long("network")
            .value_name("name")
            .help("ICP CLI network to inspect"),
    )
}

// Return list command usage text.
pub(super) fn usage() -> String {
    let mut command = list_command();
    command.render_help().to_string()
}

// Return selected fleet config usage text.
pub(super) fn config_usage() -> String {
    let mut command = config_command();
    command.render_help().to_string()
}
