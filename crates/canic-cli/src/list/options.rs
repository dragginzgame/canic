use super::ListCommandError;
use crate::{
    cli::clap::{
        defined_string_option, defined_string_or_else, flag_arg, parse_matches, render_usage,
        required_string, string_option, value_arg,
    },
    cli::defaults::default_icp,
    cli::globals::{internal_environment_arg, internal_icp_arg},
};
use clap::Command as ClapCommand;
use std::ffi::OsString;

const INFO_LIST_HELP_AFTER: &str = "\
Examples:
  canic info list demo-local
  canic info list demo-local --subtree user_hub
  canic info list demo-local --verbose";
const CONFIG_HELP_AFTER: &str = "\
Examples:
  canic fleet config test
  canic fleet config test --verbose";

///
/// ListOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ListOptions {
    pub(super) source: ListSource,
    pub(super) target: String,
    pub(super) subtree: Option<String>,
    pub(super) environment: Option<String>,
    pub(super) icp: String,
    pub(super) verbose: bool,
}

impl ListOptions {
    pub(super) fn parse_info_list<I>(args: I) -> Result<Self, ListCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(info_list_command(), args)
            .map_err(|_| ListCommandError::Usage(info_usage()))?;
        Ok(Self::from_matches(
            &matches,
            ListSource::RootRegistry,
            "deployment",
        ))
    }

    pub(super) fn parse_config<I>(args: I) -> Result<Self, ListCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(config_command(), args)
            .map_err(|_| ListCommandError::Usage(config_usage()))?;
        Ok(Self::from_matches(&matches, ListSource::Config, "fleet"))
    }

    fn from_matches(matches: &clap::ArgMatches, source: ListSource, target_arg: &str) -> Self {
        Self {
            source,
            target: required_string(matches, target_arg),
            subtree: defined_string_option(matches, "subtree"),
            environment: string_option(matches, "environment"),
            icp: defined_string_or_else(matches, "icp", default_icp),
            verbose: matches.get_flag("verbose"),
        }
    }
}

///
/// ListSource
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ListSource {
    Config,
    RootRegistry,
}

fn list_command(bin_name: &'static str, help_after: &'static str) -> ClapCommand {
    base_list_options(ClapCommand::new("list").bin_name(bin_name))
        .about("List canisters registered by an installed deployment root")
        .arg(
            value_arg("deployment")
                .value_name("deployment")
                .required(true)
                .help("Installed deployment target name to inspect"),
        )
        .arg(
            value_arg("subtree")
                .long("subtree")
                .value_name("name-or-principal")
                .help("Render a subtree anchored at a unique role name or canister principal"),
        )
        .arg(
            flag_arg("verbose")
                .long("verbose")
                .short('v')
                .help("Show full module hashes instead of 8-character prefixes"),
        )
        .arg(internal_icp_arg())
        .after_help(help_after)
}

fn info_list_command() -> ClapCommand {
    list_command("canic info list", INFO_LIST_HELP_AFTER)
}

fn config_command() -> ClapCommand {
    base_list_options(ClapCommand::new("config").bin_name("canic fleet config"))
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
    command
        .disable_help_flag(true)
        .arg(internal_environment_arg())
}

pub(super) fn info_usage() -> String {
    render_usage(info_list_command)
}

pub(super) fn config_usage() -> String {
    render_usage(config_command)
}
