use crate::{
    cli::{
        clap::{parse_subcommand, passthrough_subcommand},
        help::print_help_or_version,
    },
    cycles, info_env, list, version_text,
};
use clap::Command as ClapCommand;
use std::ffi::OsString;
use thiserror::Error as ThisError;

const INFO_USAGE: &str = "\
Group read-only installed-deployment information commands

Usage: canic info <command> [OPTIONS]

Commands:
  list     List installed deployment canisters
  cycles   Summarize deployment cycle history
  env      Print sourceable canister ID exports
  help     Print this message or the help of the given subcommand(s)

Examples:
  canic info list test --subtree scale_hub
  canic info cycles test --subtree scale_hub
  canic info env test";

///
/// InfoCommandError
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

    match parse_info_command(args)? {
        ("list", tail) => list::run_info(tail).map_err(InfoCommandError::from),
        ("cycles", tail) => cycles::run_info(tail).map_err(InfoCommandError::from),
        ("env", tail) => info_env::run(tail).map_err(InfoCommandError::from),
        _ => unreachable!("clap restricts info subcommands"),
    }
}

fn parse_info_command(
    args: Vec<OsString>,
) -> Result<(&'static str, Vec<OsString>), InfoCommandError> {
    let (command, tail) = parse_subcommand(command(), args)
        .map_err(|_| InfoCommandError::Usage(usage()))?
        .ok_or_else(|| InfoCommandError::Usage(usage()))?;
    match command.as_str() {
        "list" => Ok(("list", tail)),
        "cycles" => Ok(("cycles", tail)),
        "env" => Ok(("env", tail)),
        _ => unreachable!("clap restricts info subcommands"),
    }
}

fn command() -> ClapCommand {
    ClapCommand::new("info")
        .bin_name("canic info")
        .about("Group read-only installed-deployment information commands")
        .disable_help_flag(true)
        .subcommand(passthrough_subcommand(ClapCommand::new("list")))
        .subcommand(passthrough_subcommand(ClapCommand::new("cycles")))
        .subcommand(passthrough_subcommand(ClapCommand::new("env")))
}

#[must_use]
fn usage() -> String {
    INFO_USAGE.to_string()
}

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
}
