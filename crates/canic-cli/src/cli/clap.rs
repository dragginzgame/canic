use clap::{Arg, ArgAction, ArgMatches, Command};
use std::{ffi::OsString, path::PathBuf};

const PASSTHROUGH_ARGS: &str = "args";

pub fn parse_matches<I>(command: Command, args: I) -> Result<ArgMatches, clap::Error>
where
    I: IntoIterator<Item = OsString>,
{
    let name = command.get_name().to_string();
    command.try_get_matches_from(std::iter::once(OsString::from(name)).chain(args))
}

pub fn passthrough_subcommand(command: Command) -> Command {
    command.arg(
        Arg::new(PASSTHROUGH_ARGS)
            .num_args(0..)
            .allow_hyphen_values(true)
            .trailing_var_arg(true)
            .value_parser(clap::value_parser!(OsString)),
    )
}

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

pub fn value_arg(id: &'static str) -> Arg {
    Arg::new(id).num_args(1)
}

pub fn flag_arg(id: &'static str) -> Arg {
    Arg::new(id).action(ArgAction::SetTrue)
}

pub fn string_option(matches: &ArgMatches, id: &str) -> Option<String> {
    matches.get_one::<String>(id).cloned()
}

pub fn defined_string_option(matches: &ArgMatches, id: &str) -> Option<String> {
    matches.try_get_one::<String>(id).ok().flatten().cloned()
}

pub fn string_option_or_else(
    matches: &ArgMatches,
    id: &str,
    default: impl FnOnce() -> String,
) -> String {
    string_option(matches, id).unwrap_or_else(default)
}

pub fn defined_string_or_else(
    matches: &ArgMatches,
    id: &str,
    default: impl FnOnce() -> String,
) -> String {
    defined_string_option(matches, id).unwrap_or_else(default)
}

pub fn required_string(matches: &ArgMatches, id: &str) -> String {
    string_option(matches, id).unwrap_or_else(|| panic!("clap requires {id}"))
}

pub fn typed_option<T>(matches: &ArgMatches, id: &str) -> Option<T>
where
    T: Clone + Send + Sync + 'static,
{
    matches.get_one::<T>(id).cloned()
}

pub fn required_typed<T>(matches: &ArgMatches, id: &str) -> T
where
    T: Clone + Send + Sync + 'static,
{
    typed_option(matches, id).unwrap_or_else(|| panic!("clap requires {id}"))
}

pub fn path_option(matches: &ArgMatches, id: &str) -> Option<PathBuf> {
    string_option(matches, id).map(PathBuf::from)
}

pub fn required_path(matches: &ArgMatches, id: &str) -> PathBuf {
    path_option(matches, id).unwrap_or_else(|| panic!("clap requires {id}"))
}

pub fn parse_usize(value: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map_err(|_| "must be a non-negative integer".to_string())
}

pub fn parse_positive_usize(value: &str) -> Result<usize, String> {
    let value = parse_usize(value)?;
    if value == 0 {
        return Err("must be greater than zero".to_string());
    }
    Ok(value)
}

pub fn parse_positive_u64(value: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| "must be a positive integer".to_string())
}

pub fn render_help(mut command: Command) -> String {
    command.render_help().to_string()
}

pub fn render_usage(command: impl FnOnce() -> Command) -> String {
    render_help(command())
}
