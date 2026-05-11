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

pub fn path_option(matches: &ArgMatches, id: &str) -> Option<PathBuf> {
    string_option(matches, id).map(PathBuf::from)
}
