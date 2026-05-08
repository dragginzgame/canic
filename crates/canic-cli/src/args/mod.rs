use clap::{Arg, ArgAction, ArgMatches, Command};
use std::{ffi::OsString, path::PathBuf};

/// Return whether an argument is a command help request.
pub fn is_help_arg(arg: &OsString) -> bool {
    arg.to_str()
        .is_some_and(|arg| matches!(arg, "help" | "--help" | "-h"))
}

/// Return whether an argument is a command version request.
pub fn is_version_arg(arg: &OsString) -> bool {
    arg.to_str()
        .is_some_and(|arg| matches!(arg, "version" | "--version" | "-V"))
}

/// Return whether the first argument is a command help request.
pub fn first_arg_is_help(args: &[OsString]) -> bool {
    args.first().is_some_and(is_help_arg)
}

/// Return whether the first argument is a command version request.
pub fn first_arg_is_version(args: &[OsString]) -> bool {
    args.first().is_some_and(is_version_arg)
}

/// Print command help/version prelude output and return whether it was handled.
pub fn print_help_or_version(
    args: &[OsString],
    usage: impl FnOnce() -> String,
    version_text: &str,
) -> bool {
    if first_arg_is_help(args) {
        println!("{}", usage());
        return true;
    }
    if first_arg_is_version(args) {
        println!("{version_text}");
        return true;
    }
    false
}

/// Parse one command-family option set with the binary name injected for Clap.
pub fn parse_matches<I>(command: Command, args: I) -> Result<ArgMatches, clap::Error>
where
    I: IntoIterator<Item = OsString>,
{
    let name = command.get_name().to_string();
    command.try_get_matches_from(std::iter::once(OsString::from(name)).chain(args))
}

/// Build one string-valued Clap argument.
pub fn value_arg(id: &'static str) -> Arg {
    Arg::new(id).num_args(1)
}

/// Build one boolean Clap argument.
pub fn flag_arg(id: &'static str) -> Arg {
    Arg::new(id).action(ArgAction::SetTrue)
}

/// Read one string option from Clap matches.
pub fn string_option(matches: &ArgMatches, id: &str) -> Option<String> {
    matches.get_one::<String>(id).cloned()
}

/// Read all string values from one Clap argument.
pub fn string_values(matches: &ArgMatches, id: &str) -> Vec<String> {
    matches
        .get_many::<String>(id)
        .map(|values| values.cloned().collect())
        .unwrap_or_default()
}

/// Read one optional path from Clap matches.
pub fn path_option(matches: &ArgMatches, id: &str) -> Option<PathBuf> {
    string_option(matches, id).map(PathBuf::from)
}

/// Return the default ICP CLI executable used by live host commands.
pub fn default_icp() -> String {
    "icp".to_string()
}

/// Return the local network name used when --network is omitted.
pub fn local_network() -> String {
    "local".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Keep omitted --network behavior tied to the local replica.
    #[test]
    fn local_network_is_always_local() {
        assert_eq!(local_network(), "local");
    }
}
