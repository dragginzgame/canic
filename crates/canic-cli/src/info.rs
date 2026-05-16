use crate::{cli::help::print_help_or_version, cycles, list, version_text};
use std::ffi::OsString;
use thiserror::Error as ThisError;

const INFO_USAGE: &str = "\
Group read-only deployed-fleet information commands

Usage: canic info <command> [OPTIONS]

Commands:
  list     List deployed fleet canisters
  cycles   Summarize fleet cycle history
  help     Print this message or the help of the given subcommand(s)

Examples:
  canic info list test --subtree scale_hub
  canic info cycles test --subtree scale_hub";

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
}

/// Run the deployed-fleet information command group.
pub fn run<I>(args: I) -> Result<(), InfoCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    let Some((command, tail)) = args.split_first() else {
        return Err(InfoCommandError::Usage(usage()));
    };
    match command.to_str() {
        Some("list") => list::run_info(tail.iter().cloned()).map_err(InfoCommandError::from),
        Some("cycles") => cycles::run_info(tail.iter().cloned()).map_err(InfoCommandError::from),
        Some("help" | "--help" | "-h") => {
            println!("{}", usage());
            Ok(())
        }
        _ => Err(InfoCommandError::Usage(usage())),
    }
}

#[must_use]
pub fn usage() -> String {
    INFO_USAGE.to_string()
}
