use crate::{
    args::{default_network, parse_matches, print_help_or_version},
    version_text,
};
use canic_host::install_root::select_current_network_name;
use clap::{Arg, Command as ClapCommand};
use std::ffi::OsString;
use thiserror::Error as ThisError;

const NETWORK_HELP_AFTER: &str = "\
Examples:
  canic network
  canic network use local
  canic network use ic";

///
/// NetworkCommandError
///

#[derive(Debug, ThisError)]
pub enum NetworkCommandError {
    #[error("{0}")]
    Usage(String),

    #[error(transparent)]
    Host(#[from] Box<dyn std::error::Error>),
}

///
/// NetworkUseOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct NetworkUseOptions {
    network: String,
}

/// Run the network default command family.
pub fn run<I>(args: I) -> Result<(), NetworkCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    let mut args = args.into_iter();
    match args
        .next()
        .and_then(|arg| arg.into_string().ok())
        .as_deref()
    {
        None => {
            print_current_network(&default_network());
            Ok(())
        }
        Some("use") => {
            let options = NetworkUseOptions::parse(args)?;
            select_current_network_name(&options.network)?;
            print_current_network(&options.network);
            Ok(())
        }
        _ => Err(NetworkCommandError::Usage(usage())),
    }
}

impl NetworkUseOptions {
    // Parse the selected default network name.
    fn parse<I>(args: I) -> Result<Self, NetworkCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(network_use_command(), args)
            .map_err(|_| NetworkCommandError::Usage(use_usage()))?;
        let network = matches
            .get_one::<String>("network")
            .expect("clap requires network")
            .clone();

        Ok(Self { network })
    }
}

// Print the current default network context.
fn print_current_network(network: &str) {
    println!("{}", render_current_network(network));
}

// Render the current default network as a shell-friendly scalar value.
fn render_current_network(network: &str) -> String {
    network.to_string()
}

// Build the network command-family parser for help rendering.
fn network_command() -> ClapCommand {
    ClapCommand::new("network")
        .bin_name("canic network")
        .about("Show or select the current default network")
        .disable_help_flag(true)
        .subcommand(
            ClapCommand::new("use")
                .about("Select the current default network")
                .disable_help_flag(true),
        )
        .after_help(NETWORK_HELP_AFTER)
}

// Build the network selection parser.
fn network_use_command() -> ClapCommand {
    ClapCommand::new("use")
        .bin_name("canic network use")
        .about("Select the current default network")
        .disable_help_flag(true)
        .arg(
            Arg::new("network")
                .value_name("name")
                .required(true)
                .help("DFX network name to make current"),
        )
}

// Return network command-family usage text.
fn usage() -> String {
    let mut command = network_command();
    command.render_help().to_string()
}

// Return network selection usage text.
fn use_usage() -> String {
    let mut command = network_use_command();
    command.render_help().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Ensure network selection parses the network name.
    #[test]
    fn parses_network_use_options() {
        let options = NetworkUseOptions::parse([OsString::from("ic")]).expect("parse network");

        assert_eq!(options.network, "ic");
    }

    // Ensure network help explains the persistent default.
    #[test]
    fn network_usage_lists_use_command() {
        let text = usage();

        assert!(text.contains("Show or select the current default network"));
        assert!(text.contains("canic network use ic"));
    }

    // Ensure current network output is just the selected scalar value.
    #[test]
    fn renders_current_network_as_plain_value() {
        assert_eq!(render_current_network("local"), "local");
    }
}
