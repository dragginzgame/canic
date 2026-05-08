use super::ListCommandError;
use crate::args::{default_dfx, parse_matches, value_arg};
use clap::Command as ClapCommand;
use std::ffi::OsString;

const LIST_HELP_AFTER: &str = "\
Examples:
  canic list --fleet demo
  canic list --fleet demo --from user_hub
  canic list --fleet demo --root uzt4z-lp777-77774-qaabq-cai";
const CONFIG_HELP_AFTER: &str = "\
Examples:
  canic config --fleet demo
  canic config --fleet demo --from user_hub";

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
    pub(super) dfx: String,
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
            dfx: optional_string(matches, "dfx").unwrap_or_else(default_dfx),
        }
    }
}

// Read a string option if the subcommand declares it.
fn optional_string(matches: &clap::ArgMatches, id: &str) -> Option<String> {
    matches.try_get_one::<String>(id).ok().flatten().cloned()
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
            value_arg("dfx")
                .long("dfx")
                .value_name("path")
                .help("Path to the dfx executable"),
        )
        .after_help(LIST_HELP_AFTER)
}

// Build the selected fleet config parser.
fn config_command() -> ClapCommand {
    base_list_options(ClapCommand::new("config").bin_name("canic config"))
        .about("List canisters declared by the selected fleet config")
        .arg(
            value_arg("from")
                .long("from")
                .value_name("role")
                .help("Show one declared role"),
        )
        .after_help(CONFIG_HELP_AFTER)
}

// Add options shared by all list subcommands.
fn base_list_options(command: ClapCommand) -> ClapCommand {
    command
        .disable_help_flag(true)
        .arg(
            value_arg("fleet")
                .long("fleet")
                .value_name("name")
                .required(true)
                .help("Fleet name to inspect"),
        )
        .arg(
            value_arg("network")
                .long("network")
                .value_name("name")
                .help("DFX network to inspect"),
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
