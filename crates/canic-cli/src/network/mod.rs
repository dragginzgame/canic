use crate::{
    args::{default_network, parse_matches, print_help_or_version},
    version_text,
};
use clap::Command as ClapCommand;
use std::ffi::OsString;
use thiserror::Error as ThisError;

const NETWORK_HELP_AFTER: &str = "\
Examples:
  canic network current

Canic uses local by default. Pass --network ic to a fleet command when you
intentionally want one command to target mainnet.";
const NETWORK_CURRENT_HELP_AFTER: &str = "\
Examples:
  canic network current";

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
            println!("{}", usage());
            Ok(())
        }
        Some("current") => {
            let args = args.collect::<Vec<_>>();
            if print_help_or_version(&args, current_usage, version_text()) {
                return Ok(());
            }
            parse_matches(network_current_command(), args)
                .map_err(|_| NetworkCommandError::Usage(current_usage()))?;
            print_current_network(&default_network());
            Ok(())
        }
        _ => Err(NetworkCommandError::Usage(usage())),
    }
}

// Print the implicit network context.
fn print_current_network(network: &str) {
    println!("{}", render_current_network(network));
}

// Render the implicit network as a shell-friendly scalar value.
fn render_current_network(network: &str) -> String {
    network.to_string()
}

// Build the network command-family parser for help rendering.
fn network_command() -> ClapCommand {
    ClapCommand::new("network")
        .bin_name("canic network")
        .about("Show Canic network context")
        .disable_help_flag(true)
        .subcommand(ClapCommand::new("current").about("Print the implicit network"))
        .after_help(NETWORK_HELP_AFTER)
}

// Build the current-network parser.
fn network_current_command() -> ClapCommand {
    ClapCommand::new("current")
        .bin_name("canic network current")
        .about("Print the implicit network")
        .disable_help_flag(true)
        .after_help(NETWORK_CURRENT_HELP_AFTER)
}

// Return network command-family usage text.
fn usage() -> String {
    let mut command = network_command();
    command.render_help().to_string()
}

// Return current network usage text.
fn current_usage() -> String {
    let mut command = network_current_command();
    command.render_help().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Ensure network help explains the persistent default.
    #[test]
    fn network_usage_lists_use_command() {
        let text = usage();

        assert!(text.contains("Show Canic network context"));
        assert!(text.contains("current"));
        assert!(text.contains("canic network current"));
        assert!(!text.contains("canic network use"));
        assert!(text.contains("Pass --network ic"));
    }

    // Ensure current network help renders the scalar command.
    #[test]
    fn current_usage_lists_examples() {
        let text = current_usage();

        assert!(text.contains("Print the implicit network"));
        assert!(text.contains("Usage: canic network current"));
        assert!(text.contains("canic network current"));
    }

    // Ensure current network output is just the selected scalar value.
    #[test]
    fn renders_current_network_as_plain_value() {
        assert_eq!(render_current_network("local"), "local");
    }
}
