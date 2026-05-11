use crate::{
    cli::clap::{flag_arg, parse_matches, path_option, string_option, value_arg},
    cli::defaults::{default_icp, local_network},
    cli::globals::{internal_icp_arg, internal_network_arg},
    cycles::CyclesCommandError,
};
use clap::Command as ClapCommand;
use std::{ffi::OsString, path::PathBuf};

const DEFAULT_SINCE_SECONDS: u64 = 24 * 60 * 60;
const DEFAULT_LIMIT: u64 = 1_000;

///
/// CyclesOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CyclesOptions {
    pub fleet: String,
    pub since_seconds: u64,
    pub limit: u64,
    pub json: bool,
    pub out: Option<PathBuf>,
    pub network: String,
    pub icp: String,
}

impl CyclesOptions {
    pub fn parse<I>(args: I) -> Result<Self, CyclesCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(cycles_command(), args)
            .map_err(|_| CyclesCommandError::Usage(usage()))?;
        let since_seconds = string_option(&matches, "since")
            .map(|value| parse_duration(&value))
            .transpose()?
            .unwrap_or(DEFAULT_SINCE_SECONDS);
        let limit = string_option(&matches, "limit")
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|limit| *limit > 0)
            .unwrap_or(DEFAULT_LIMIT);

        Ok(Self {
            fleet: string_option(&matches, "fleet").expect("clap requires fleet"),
            since_seconds,
            limit,
            json: matches.get_flag("json"),
            out: path_option(&matches, "out"),
            network: string_option(&matches, "network").unwrap_or_else(local_network),
            icp: string_option(&matches, "icp").unwrap_or_else(default_icp),
        })
    }
}

pub(super) fn parse_duration(value: &str) -> Result<u64, CyclesCommandError> {
    let value = value.trim();
    let digits = value
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>();
    let suffix = value[digits.len()..].trim();
    let amount = digits
        .parse::<u64>()
        .map_err(|_| CyclesCommandError::InvalidDuration(value.to_string()))?;
    let multiplier = match suffix {
        "s" | "" => 1,
        "m" => 60,
        "h" => 60 * 60,
        "d" => 24 * 60 * 60,
        _ => return Err(CyclesCommandError::InvalidDuration(value.to_string())),
    };
    amount
        .checked_mul(multiplier)
        .filter(|seconds| *seconds > 0)
        .ok_or_else(|| CyclesCommandError::InvalidDuration(value.to_string()))
}

pub(super) fn usage() -> String {
    let mut command = cycles_command();
    command.render_help().to_string()
}

fn cycles_command() -> ClapCommand {
    ClapCommand::new("cycles")
        .bin_name("canic cycles")
        .about("Summarize fleet cycle history")
        .disable_help_flag(true)
        .arg(
            value_arg("fleet")
                .value_name("fleet")
                .required(true)
                .help("Installed fleet name to inspect"),
        )
        .arg(
            value_arg("since")
                .long("since")
                .value_name("duration")
                .help("Cycle history window; defaults to 24h"),
        )
        .arg(
            value_arg("limit")
                .long("limit")
                .value_name("entries")
                .help("Maximum tracker samples to fetch per canister; defaults to 1000"),
        )
        .arg(flag_arg("json").long("json"))
        .arg(value_arg("out").long("out").value_name("file"))
        .arg(internal_network_arg())
        .arg(internal_icp_arg())
}
