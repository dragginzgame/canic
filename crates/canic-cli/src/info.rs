//! Module: canic_cli::info
//!
//! Responsibility: dispatch read-only current-Fleet information subcommands.
//! Does not own: Fleet state, registry state, canister lifecycle, or output formats.
//! Boundary: parses the `canic info` group and delegates to leaf command modules.

use crate::{
    cli::{
        clap::{parse_subcommand, passthrough_subcommand},
        help::print_help_or_version,
    },
    cycles, endpoints, info_env, info_subnets, list, metrics, version_text,
};
use clap::Command as ClapCommand;
use std::ffi::OsString;
use thiserror::Error as ThisError;

const INFO_USAGE: &str = "\
Group read-only current-Fleet information commands

Usage: canic info <command> [OPTIONS]

Commands:
  cycles     Summarize current Fleet cycle history
  endpoints  List callable Candid endpoints
  env        Print sourceable canister ID exports
  list       List current Fleet canisters
  metrics    Query Canic runtime telemetry
  subnets    Show live Fleet-owned Canister counts by physical Subnet
  help       Print this message or the help of the given subcommand(s)

Examples:
  canic info endpoints test root
  canic info list test --subtree scale_hub
  canic info metrics test runtime";
const INFO_SUBCOMMANDS: &[&str] = &["cycles", "endpoints", "env", "list", "metrics", "subnets"];

///
/// InfoCommandError
///
/// CLI boundary error for the `canic info` command group and delegated
/// read-only information subcommands.
///

#[derive(Debug, ThisError)]
pub enum InfoCommandError {
    #[error("{0}")]
    Usage(String),

    #[error("endpoints: {0}")]
    Endpoints(#[from] endpoints::EndpointsCommandError),

    #[error("cycles: {0}")]
    Cycles(#[source] Box<cycles::CyclesCommandError>),

    #[error("env: {0}")]
    Env(#[source] Box<info_env::InfoEnvCommandError>),

    #[error("list: {0}")]
    List(#[source] Box<list::ListCommandError>),

    #[error("metrics: {0}")]
    Metrics(#[source] Box<metrics::MetricsCommandError>),

    #[error("subnets: {0}")]
    Subnets(#[source] Box<info_subnets::InfoSubnetsCommandError>),
}

impl From<info_env::InfoEnvCommandError> for InfoCommandError {
    fn from(error: info_env::InfoEnvCommandError) -> Self {
        Self::Env(Box::new(error))
    }
}

impl From<cycles::CyclesCommandError> for InfoCommandError {
    fn from(error: cycles::CyclesCommandError) -> Self {
        Self::Cycles(Box::new(error))
    }
}

impl From<list::ListCommandError> for InfoCommandError {
    fn from(error: list::ListCommandError) -> Self {
        Self::List(Box::new(error))
    }
}

impl From<metrics::MetricsCommandError> for InfoCommandError {
    fn from(error: metrics::MetricsCommandError) -> Self {
        Self::Metrics(Box::new(error))
    }
}

impl From<info_subnets::InfoSubnetsCommandError> for InfoCommandError {
    fn from(error: info_subnets::InfoSubnetsCommandError) -> Self {
        Self::Subnets(Box::new(error))
    }
}

impl InfoCommandError {
    pub const fn exit_code(&self) -> u8 {
        match self {
            Self::Usage(_) => 2,
            Self::Cycles(_)
            | Self::Endpoints(_)
            | Self::Env(_)
            | Self::List(_)
            | Self::Metrics(_)
            | Self::Subnets(_) => 1,
        }
    }
}

/// Run the current-Fleet information command group.
pub fn run<I>(args: I) -> Result<(), InfoCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    let (command, tail) = parse_info_command(args)?;
    match command.as_str() {
        "cycles" => cycles::run_info(tail).map_err(InfoCommandError::from),
        "endpoints" => endpoints::run_info(tail).map_err(InfoCommandError::from),
        "env" => info_env::run(tail).map_err(InfoCommandError::from),
        "list" => list::run_info(tail).map_err(InfoCommandError::from),
        "metrics" => metrics::run_info(tail).map_err(InfoCommandError::from),
        "subnets" => info_subnets::run(tail).map_err(InfoCommandError::from),
        _ => unreachable!("clap restricts info subcommands"),
    }
}

fn parse_info_command(args: Vec<OsString>) -> Result<(String, Vec<OsString>), InfoCommandError> {
    parse_subcommand(command(), args)
        .map_err(|_| InfoCommandError::Usage(usage()))?
        .ok_or_else(|| InfoCommandError::Usage(usage()))
}

fn command() -> ClapCommand {
    let command = ClapCommand::new("info")
        .bin_name("canic info")
        .about("Group read-only current-Fleet information commands")
        .disable_help_flag(true);
    INFO_SUBCOMMANDS.iter().fold(command, |command, name| {
        command.subcommand(passthrough_subcommand(ClapCommand::new(*name)))
    })
}

#[must_use]
fn usage() -> String {
    INFO_USAGE.to_string()
}

// -----------------------------------------------------------------------------
// Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_info_subcommands_with_passthrough_args() {
        let (command, tail) = parse_info_command(vec![
            OsString::from("list"),
            OsString::from("demo"),
            OsString::from("--subtree"),
            OsString::from("app"),
        ])
        .expect("parse info list");

        assert_eq!(command, "list");
        assert_eq!(
            tail,
            vec![
                OsString::from("demo"),
                OsString::from("--subtree"),
                OsString::from("app")
            ]
        );
    }

    #[test]
    fn rejects_missing_or_unknown_info_subcommand() {
        std::assert_matches!(
            parse_info_command(Vec::new()),
            Err(InfoCommandError::Usage(_))
        );
        std::assert_matches!(
            parse_info_command(vec![OsString::from("unknown")]),
            Err(InfoCommandError::Usage(_))
        );
    }

    #[test]
    fn info_usage_includes_representative_examples() {
        let text = usage();

        assert!(text.contains("canic info endpoints test"));
        assert!(text.contains("canic info list test"));
        assert!(text.contains("canic info metrics test"));
    }
}
