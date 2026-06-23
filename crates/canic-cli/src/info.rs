//! Module: canic_cli::info
//!
//! Responsibility: dispatch read-only installed-deployment information subcommands.
//! Does not own: deployment state, registry state, canister lifecycle, or output formats.
//! Boundary: parses the `canic info` group and delegates to leaf command modules.

use crate::{
    cli::{
        clap::{parse_subcommand, passthrough_subcommand},
        help::print_help_or_version,
    },
    cycles, endpoints, info_env, list, medic, metrics, version_text,
};
use clap::Command as ClapCommand;
use std::ffi::OsString;
use thiserror::Error as ThisError;

const INFO_USAGE: &str = "\
Group read-only installed-deployment information commands

Usage: canic info <command> [OPTIONS]

Commands:
  list       List installed deployment canisters
  cycles     Summarize deployment cycle history
  metrics    Query Canic runtime telemetry
  endpoints  List callable Candid endpoints
  medic      Diagnose local deployment target setup
  env        Print sourceable canister ID exports
  help       Print this message or the help of the given subcommand(s)

Examples:
  canic info list test --subtree scale_hub
  canic info cycles test --subtree scale_hub
  canic info metrics test
  canic info endpoints test app
  canic info medic test
  canic info medic test --blob-storage backend
  canic info env test";
const INFO_SUBCOMMANDS: &[&str] = &["list", "cycles", "metrics", "endpoints", "medic", "env"];

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

    #[error("list: {0}")]
    List(#[from] list::ListCommandError),

    #[error("cycles: {0}")]
    Cycles(#[from] cycles::CyclesCommandError),

    #[error("env: {0}")]
    Env(#[from] info_env::InfoEnvCommandError),

    #[error("endpoints: {0}")]
    Endpoints(#[from] endpoints::EndpointsCommandError),

    #[error("medic: {0}")]
    Medic(#[from] medic::MedicCommandError),

    #[error("metrics: {0}")]
    Metrics(#[from] metrics::MetricsCommandError),
}

/// Run the installed-deployment information command group.
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
        "list" => list::run_info(tail).map_err(InfoCommandError::from),
        "cycles" => cycles::run_info(tail).map_err(InfoCommandError::from),
        "metrics" => metrics::run_info(tail).map_err(InfoCommandError::from),
        "endpoints" => endpoints::run_info(tail).map_err(InfoCommandError::from),
        "medic" => medic::run_info(tail).map_err(InfoCommandError::from),
        "env" => info_env::run(tail).map_err(InfoCommandError::from),
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
        .about("Group read-only installed-deployment information commands")
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
    fn info_usage_mentions_targeted_blob_storage_medic() {
        let text = usage();

        assert!(text.contains("canic info medic test"));
        assert!(text.contains("canic info medic test --blob-storage backend"));
    }
}
