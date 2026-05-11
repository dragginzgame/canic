use super::ListCommandError;
use crate::{
    cli::clap::{flag_arg, parse_matches, value_arg},
    cli::defaults::default_icp,
    cli::globals::{internal_icp_arg, internal_network_arg},
};
use clap::Command as ClapCommand;
use std::ffi::OsString;

const LIST_HELP_AFTER: &str = "\
Examples:
  canic list test
  canic list test --subtree user_hub
  canic list test --verbose";
const CONFIG_HELP_AFTER: &str = "\
Examples:
  canic config test
  canic config test --verbose";

///
/// ListOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ListOptions {
    pub(super) source: ListSource,
    pub(super) fleet: String,
    pub(super) subtree: Option<String>,
    pub(super) network: Option<String>,
    pub(super) icp: String,
    pub(super) verbose: bool,
}

impl ListOptions {
    pub(super) fn parse_list<I>(args: I) -> Result<Self, ListCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches =
            parse_matches(list_command(), args).map_err(|_| ListCommandError::Usage(usage()))?;
        Ok(Self::from_matches(&matches, ListSource::RootRegistry))
    }

    pub(super) fn parse_config<I>(args: I) -> Result<Self, ListCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(config_command(), args)
            .map_err(|_| ListCommandError::Usage(config_usage()))?;
        Ok(Self::from_matches(&matches, ListSource::Config))
    }

    fn from_matches(matches: &clap::ArgMatches, source: ListSource) -> Self {
        Self {
            source,
            fleet: optional_string(matches, "fleet").expect("clap requires fleet"),
            subtree: optional_string(matches, "subtree"),
            network: optional_string(matches, "network"),
            icp: optional_string(matches, "icp").unwrap_or_else(default_icp),
            verbose: optional_bool(matches, "verbose"),
        }
    }
}

fn optional_string(matches: &clap::ArgMatches, id: &str) -> Option<String> {
    matches.try_get_one::<String>(id).ok().flatten().cloned()
}

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
            value_arg("subtree")
                .long("subtree")
                .value_name("name-or-principal")
                .help("Render a subtree anchored at one canister"),
        )
        .arg(
            flag_arg("verbose")
                .long("verbose")
                .short('v')
                .help("Show full module hashes instead of 8-character prefixes"),
        )
        .arg(internal_icp_arg())
        .after_help(LIST_HELP_AFTER)
}

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
            flag_arg("verbose")
                .long("verbose")
                .short('v')
                .help("Show indented declared config details under each role"),
        )
        .after_help(CONFIG_HELP_AFTER)
}

fn base_list_options(command: ClapCommand) -> ClapCommand {
    command.disable_help_flag(true).arg(internal_network_arg())
}

pub(super) fn usage() -> String {
    let mut command = list_command();
    command.render_help().to_string()
}

pub(super) fn config_usage() -> String {
    let mut command = config_command();
    command.render_help().to_string()
}
