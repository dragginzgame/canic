use crate::{
    cli::clap::{
        flag_arg, parse_matches, parse_positive_u64, path_option, render_usage, required_string,
        required_typed, string_option, string_option_or_else, value_arg,
    },
    cli::defaults::{default_icp, local_network},
    cli::globals::{internal_icp_arg, internal_network_arg},
    cycles::CyclesCommandError,
};
use clap::Command as ClapCommand;
use std::{ffi::OsString, path::PathBuf};

const DEFAULT_SINCE: &str = "24h";
const DEFAULT_LIMIT: &str = "1000";

const COMMAND_NAME: &str = "cycles";
const DEPLOYMENT_ARG: &str = "deployment";
const JSON_ARG: &str = "json";
const LIMIT_ARG: &str = "limit";
const OUT_ARG: &str = "out";
const SINCE_ARG: &str = "since";
const SUBTREE_ARG: &str = "subtree";
const VERBOSE_ARG: &str = "verbose";

///
/// CyclesOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct CyclesOptions {
    pub(super) deployment: String,
    pub(super) subtree: Option<String>,
    pub(super) since_seconds: u64,
    pub(super) limit: u64,
    pub(super) json: bool,
    pub(super) verbose: bool,
    pub(super) out: Option<PathBuf>,
    pub(super) network: String,
    pub(super) icp: String,
}

impl CyclesOptions {
    pub(super) fn parse_info<I>(args: I) -> Result<Self, CyclesCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(info_cycles_command(), args)
            .map_err(|_| CyclesCommandError::Usage(info_usage()))?;
        Ok(Self::from_matches(&matches))
    }

    fn from_matches(matches: &clap::ArgMatches) -> Self {
        Self {
            deployment: required_string(matches, DEPLOYMENT_ARG),
            subtree: string_option(matches, SUBTREE_ARG),
            since_seconds: required_typed(matches, SINCE_ARG),
            limit: required_typed(matches, LIMIT_ARG),
            json: matches.get_flag(JSON_ARG),
            verbose: matches.get_flag(VERBOSE_ARG),
            out: path_option(matches, OUT_ARG),
            network: string_option_or_else(matches, "network", local_network),
            icp: string_option_or_else(matches, "icp", default_icp),
        }
    }
}

fn parse_duration(value: &str) -> Result<u64, String> {
    let value = value.trim();
    let digits = value
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>();
    let suffix = value[digits.len()..].trim();
    let amount = digits.parse::<u64>().map_err(|_| invalid_duration(value))?;
    let multiplier = match suffix {
        "s" | "" => 1,
        "m" => 60,
        "h" => 60 * 60,
        "d" => 24 * 60 * 60,
        _ => return Err(invalid_duration(value)),
    };
    amount
        .checked_mul(multiplier)
        .filter(|seconds| *seconds > 0)
        .ok_or_else(|| invalid_duration(value))
}

fn invalid_duration(value: &str) -> String {
    format!("invalid duration {value}; use values like 1h, 6h, 24h, 7d, or 30m")
}

pub(super) fn info_usage() -> String {
    render_usage(info_cycles_command)
}

fn info_cycles_command() -> ClapCommand {
    cycles_command_with_bin_name("canic info cycles")
}

fn cycles_command_with_bin_name(bin_name: &'static str) -> ClapCommand {
    ClapCommand::new(COMMAND_NAME)
        .bin_name(bin_name)
        .about("Summarize installed deployment cycle history")
        .disable_help_flag(true)
        .arg(
            value_arg(DEPLOYMENT_ARG)
                .value_name(DEPLOYMENT_ARG)
                .required(true)
                .help("Installed deployment target name to inspect"),
        )
        .arg(
            value_arg(SINCE_ARG)
                .long(SINCE_ARG)
                .value_name("duration")
                .default_value(DEFAULT_SINCE)
                .value_parser(clap::builder::ValueParser::new(parse_duration))
                .help("Cycle history window; defaults to 24h"),
        )
        .arg(
            value_arg(SUBTREE_ARG)
                .long(SUBTREE_ARG)
                .value_name("name-or-principal")
                .help("Summarize one subtree anchored at a unique role name or canister principal"),
        )
        .arg(
            value_arg(LIMIT_ARG)
                .long(LIMIT_ARG)
                .value_name("entries")
                .default_value(DEFAULT_LIMIT)
                .value_parser(clap::builder::ValueParser::new(parse_positive_u64))
                .help("Maximum tracker samples to fetch per canister; defaults to 1000"),
        )
        .arg(flag_arg(JSON_ARG).long(JSON_ARG))
        .arg(
            flag_arg(VERBOSE_ARG).long(VERBOSE_ARG).short('v').help(
                "Show diagnostic columns such as canister id, history, topups, and net total",
            ),
        )
        .arg(value_arg(OUT_ARG).long(OUT_ARG).value_name("file"))
        .arg(internal_network_arg())
        .arg(internal_icp_arg())
}
