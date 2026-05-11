use crate::{
    cli::clap::{parse_matches, string_option, value_arg},
    cli::defaults::local_network,
    cli::globals::internal_network_arg,
    cli::help::print_help_or_version,
    version_text,
};
use canic_host::install_root::{InstallRootOptions, install_root};
use clap::Command as ClapCommand;
use std::ffi::OsString;
use thiserror::Error as ThisError;

const DEFAULT_ROOT_TARGET: &str = "root";
const DEFAULT_READY_TIMEOUT_SECONDS: u64 = 120;
const INSTALL_HELP_AFTER: &str = "\
Examples:
  canic install test

canic install uses fleets/<fleet>/canic.toml.

The selected canic.toml must include:
  [fleet]
  name = \"test\"";

///
/// InstallCommandError
///

#[derive(Debug, ThisError)]
pub enum InstallCommandError {
    #[error("{0}")]
    Usage(String),

    #[error(transparent)]
    Install(#[from] Box<dyn std::error::Error>),
}

///
/// InstallOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstallOptions {
    pub fleet: String,
    pub network: String,
}

impl InstallOptions {
    pub fn parse<I>(args: I) -> Result<Self, InstallCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(install_command(), args)
            .map_err(|_| InstallCommandError::Usage(usage()))?;
        let fleet = string_option(&matches, "fleet").expect("clap requires fleet");

        Ok(Self {
            fleet,
            network: string_option(&matches, "network").unwrap_or_else(local_network),
        })
    }

    #[must_use]
    pub fn into_install_root_options(self) -> InstallRootOptions {
        InstallRootOptions {
            root_canister: DEFAULT_ROOT_TARGET.to_string(),
            root_build_target: DEFAULT_ROOT_TARGET.to_string(),
            network: self.network,
            ready_timeout_seconds: DEFAULT_READY_TIMEOUT_SECONDS,
            config_path: Some(default_fleet_config_path(&self.fleet)),
            expected_fleet: Some(self.fleet),
            interactive_config_selection: false,
        }
    }
}

fn install_command() -> ClapCommand {
    ClapCommand::new("install")
        .bin_name("canic install")
        .about("Install and bootstrap a Canic fleet")
        .disable_help_flag(true)
        .override_usage("canic install <fleet>")
        .arg(
            value_arg("fleet")
                .value_name("fleet")
                .required(true)
                .help("Config-defined fleet name to install"),
        )
        .arg(internal_network_arg())
        .after_help(INSTALL_HELP_AFTER)
}

/// Run the root install workflow.
pub fn run<I>(args: I) -> Result<(), InstallCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    let options = InstallOptions::parse(args)?;
    install_root(options.into_install_root_options()).map_err(InstallCommandError::from)
}

fn default_fleet_config_path(fleet: &str) -> String {
    format!("fleets/{fleet}/canic.toml")
}

fn usage() -> String {
    let mut command = install_command();
    command.render_help().to_string()
}

#[cfg(test)]
mod tests;
