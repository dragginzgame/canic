//! Module: canic_cli::cli::clap
//!
//! Responsibility: provide shared Clap adapters for Canic CLI command modules.
//! Does not own: command-specific option semantics, dispatch policy, or rendered help text.
//! Boundary: wraps common argument extraction and parser construction patterns.

use clap::{Arg, ArgAction, ArgMatches, Command};
use std::{ffi::OsString, path::PathBuf};

const PASSTHROUGH_ARGS: &str = "args";

/// Parse a command from an argument iterator without requiring callers to
/// include the command binary name.
pub fn parse_matches<I>(command: Command, args: I) -> Result<ArgMatches, clap::Error>
where
    I: IntoIterator<Item = OsString>,
{
    let name = command.get_name().to_string();
    command.try_get_matches_from(std::iter::once(OsString::from(name)).chain(args))
}

/// Configure a subcommand to collect all remaining arguments for later parsing.
pub fn passthrough_subcommand(command: Command) -> Command {
    command.arg(
        Arg::new(PASSTHROUGH_ARGS)
            .num_args(0..)
            .allow_hyphen_values(true)
            .trailing_var_arg(true)
            .value_parser(clap::value_parser!(OsString)),
    )
}

/// Parse an optional passthrough subcommand and return its collected tail args.
pub fn parse_subcommand<I>(
    command: Command,
    args: I,
) -> Result<Option<(String, Vec<OsString>)>, clap::Error>
where
    I: IntoIterator<Item = OsString>,
{
    let matches = parse_matches(command, args)?;
    Ok(matches.subcommand().map(|(name, matches)| {
        let args = matches
            .get_many::<OsString>(PASSTHROUGH_ARGS)
            .map(|values| values.cloned().collect::<Vec<_>>())
            .unwrap_or_default();

        (name.to_string(), args)
    }))
}

/// Parse a required passthrough subcommand and return its collected tail args.
pub fn parse_required_subcommand<I>(
    command: Command,
    args: I,
) -> Result<(String, Vec<OsString>), clap::Error>
where
    I: IntoIterator<Item = OsString>,
{
    parse_subcommand(command.subcommand_required(true), args).map(|subcommand| match subcommand {
        Some(subcommand) => subcommand,
        None => unreachable!("clap requires a subcommand"),
    })
}

/// Build a single-value Clap argument.
pub fn value_arg(id: &'static str) -> Arg {
    Arg::new(id).num_args(1)
}

/// Build a boolean flag Clap argument.
pub fn flag_arg(id: &'static str) -> Arg {
    Arg::new(id).action(ArgAction::SetTrue)
}

/// Read an optional string argument from parsed matches.
pub fn string_option(matches: &ArgMatches, id: &str) -> Option<String> {
    matches.get_one::<String>(id).cloned()
}

/// Read an optional string argument only when the command defined that id.
pub fn defined_string_option(matches: &ArgMatches, id: &str) -> Option<String> {
    matches.try_get_one::<String>(id).ok().flatten().cloned()
}

/// Read an optional string argument or produce a default.
pub fn string_option_or_else(
    matches: &ArgMatches,
    id: &str,
    default: impl FnOnce() -> String,
) -> String {
    string_option(matches, id).unwrap_or_else(default)
}

/// Read a defined optional string argument or produce a default.
pub fn defined_string_or_else(
    matches: &ArgMatches,
    id: &str,
    default: impl FnOnce() -> String,
) -> String {
    defined_string_option(matches, id).unwrap_or_else(default)
}

/// Read a required string argument from parsed matches.
///
/// # Panics
///
/// Panics when the command definition did not require `id` but the caller
/// treats it as required.
pub fn required_string(matches: &ArgMatches, id: &str) -> String {
    string_option(matches, id).unwrap_or_else(|| panic!("clap requires {id}"))
}

/// Read an optional typed argument from parsed matches.
pub fn typed_option<T>(matches: &ArgMatches, id: &str) -> Option<T>
where
    T: Clone + Send + Sync + 'static,
{
    matches.get_one::<T>(id).cloned()
}

/// Read a required typed argument from parsed matches.
///
/// # Panics
///
/// Panics when the command definition did not require `id` but the caller
/// treats it as required.
pub fn required_typed<T>(matches: &ArgMatches, id: &str) -> T
where
    T: Clone + Send + Sync + 'static,
{
    typed_option(matches, id).unwrap_or_else(|| panic!("clap requires {id}"))
}

/// Read an optional path argument from parsed matches.
pub fn path_option(matches: &ArgMatches, id: &str) -> Option<PathBuf> {
    string_option(matches, id).map(PathBuf::from)
}

/// Read a required path argument from parsed matches.
///
/// # Panics
///
/// Panics when the command definition did not require `id` but the caller
/// treats it as required.
pub fn required_path(matches: &ArgMatches, id: &str) -> PathBuf {
    path_option(matches, id).unwrap_or_else(|| panic!("clap requires {id}"))
}

/// Parse a strictly positive `usize` option value.
pub fn parse_positive_usize(value: &str) -> Result<usize, String> {
    let value = value
        .parse::<usize>()
        .map_err(|_| "must be a non-negative integer".to_string())?;
    if value == 0 {
        return Err("must be greater than zero".to_string());
    }
    Ok(value)
}

/// Parse a strictly positive `u64` option value.
pub fn parse_positive_u64(value: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| "must be a positive integer".to_string())
}

/// Render command help through the shared Canic usage helper shape.
pub fn render_usage(command: impl FnOnce() -> Command) -> String {
    command().render_help().to_string()
}

// -----------------------------------------------------------------------------
// Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_subcommand_returns_passthrough_args() {
        let command = Command::new("canic")
            .subcommand_required(true)
            .subcommand(passthrough_subcommand(Command::new("run")));

        let parsed =
            parse_required_subcommand(command, [OsString::from("run"), OsString::from("--flag")])
                .expect("parse required subcommand");

        assert_eq!(parsed, ("run".to_string(), vec![OsString::from("--flag")]));
    }

    #[test]
    fn positive_parsers_reject_zero() {
        assert_eq!(parse_positive_usize("1"), Ok(1));
        assert_eq!(parse_positive_u64("2"), Ok(2));
        assert!(parse_positive_usize("0").is_err());
        assert!(parse_positive_u64("0").is_err());
    }
}
