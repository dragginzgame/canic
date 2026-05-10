use crate::{
    args::{
        internal_network_arg, local_network, parse_matches, print_help_or_version, string_option,
        value_arg,
    },
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
mod tests {
    use super::*;

    // Ensure install defaults to the conventional local root canister target.
    #[test]
    fn install_defaults_to_root_target() {
        let options = InstallOptions::parse([OsString::from("demo")]).expect("parse defaults");
        let install = options.clone().into_install_root_options();

        assert_eq!(options.fleet, "demo");
        assert_eq!(options.network, local_network());
        assert_eq!(install.root_canister, "root");
        assert_eq!(install.root_build_target, "root");
        assert_eq!(install.ready_timeout_seconds, DEFAULT_READY_TIMEOUT_SECONDS);
        assert_eq!(
            install.config_path,
            Some("fleets/demo/canic.toml".to_string())
        );
        assert_eq!(install.expected_fleet, Some("demo".to_string()));
    }

    // Ensure top-level dispatch can pass network selection internally.
    #[test]
    fn install_accepts_internal_network() {
        let options = InstallOptions::parse([
            OsString::from("demo"),
            OsString::from(crate::args::INTERNAL_NETWORK_OPTION),
            OsString::from("local"),
        ])
        .expect("parse internal network");

        assert_eq!(options.network, "local");
    }

    // Ensure removed install target forms are rejected before mutation starts.
    #[test]
    fn install_rejects_target_overrides() {
        let root_arg = InstallOptions::parse([OsString::from("demo"), OsString::from("root")])
            .expect_err("positional root target should be removed");
        let root_flag = InstallOptions::parse([
            OsString::from("demo"),
            OsString::from("--root"),
            OsString::from("uxrrr-q7777-77774-qaaaq-cai"),
        ])
        .expect_err("root flag should be removed");
        let build_target = InstallOptions::parse([
            OsString::from("demo"),
            OsString::from("--root-build-target"),
            OsString::from("custom_root"),
        ])
        .expect_err("root build target should be removed");

        assert!(matches!(root_arg, InstallCommandError::Usage(_)));
        assert!(matches!(root_flag, InstallCommandError::Usage(_)));
        assert!(matches!(build_target, InstallCommandError::Usage(_)));
    }

    // Ensure removed install config selection is rejected before mutation starts.
    #[test]
    fn install_rejects_config_path() {
        let err = InstallOptions::parse([
            OsString::from("demo"),
            OsString::from("--config"),
            OsString::from("fleets/demo/canic.toml"),
        ])
        .expect_err("config override should be removed");

        assert!(matches!(err, InstallCommandError::Usage(_)));
    }

    // Ensure removed ready timeout controls are rejected before mutation starts.
    #[test]
    fn install_rejects_ready_timeout() {
        let err = InstallOptions::parse([
            OsString::from("demo"),
            OsString::from("--ready-timeout-seconds"),
            OsString::from("30"),
        ])
        .expect_err("ready timeout override should be removed");

        assert!(matches!(err, InstallCommandError::Usage(_)));
    }

    // Ensure install requires an explicit fleet argument.
    #[test]
    fn install_requires_fleet_argument() {
        let err = InstallOptions::parse([]).expect_err("missing fleet should fail");

        assert!(matches!(err, InstallCommandError::Usage(_)));
    }

    // Ensure install help documents config-owned fleet identity.
    #[test]
    fn install_usage_explains_fleet_config() {
        let text = usage();

        assert!(text.contains("Install and bootstrap a Canic fleet"));
        assert!(text.contains("Usage: canic install <fleet>"));
        assert!(text.contains("<fleet>"));
        assert!(!text.contains("--fleet <name>"));
        assert!(!text.contains("[name-or-principal]"));
        assert!(!text.contains("--config"));
        assert!(!text.contains("--ready-timeout-seconds"));
        assert!(!text.contains("--root <name-or-principal>"));
        assert!(!text.contains("--root-build-target"));
        assert!(!text.contains("--network"));
        assert!(text.contains("[fleet]"));
        assert!(text.contains("name = \"test\""));
    }
}
