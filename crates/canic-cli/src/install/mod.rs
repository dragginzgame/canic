use crate::{
    args::{
        local_network, parse_matches, print_help_or_version, string_option, string_values,
        value_arg,
    },
    version_text,
};
use candid::Principal;
use canic_host::install_root::{InstallRootOptions, install_root};
use clap::{Arg, Command as ClapCommand};
use std::{env, ffi::OsString};
use thiserror::Error as ThisError;

const DEFAULT_ROOT_TARGET: &str = "root";
const DEFAULT_READY_TIMEOUT_SECONDS: u64 = 120;
const INSTALL_HELP_AFTER: &str = "\
Examples:
  canic install test
  canic install test root
  canic install test uxrrr-q7777-77774-qaaaq-cai
  canic install test --config fleets/test/canic.toml

Without --config, canic install uses fleets/<fleet>/canic.toml.

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

    #[error("cannot provide both positional root target and --root")]
    ConflictingRootTarget,

    #[error("invalid --ready-timeout-seconds value {0}")]
    InvalidReadyTimeout(String),

    #[error(transparent)]
    Install(#[from] Box<dyn std::error::Error>),
}

///
/// InstallOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstallOptions {
    pub fleet: String,
    pub root_target: String,
    pub root_build_target: String,
    pub network: String,
    pub ready_timeout_seconds: u64,
    pub config_path: Option<String>,
}

impl InstallOptions {
    /// Parse install options from CLI arguments.
    pub fn parse<I>(args: I) -> Result<Self, InstallCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(install_command(), args)
            .map_err(|_| InstallCommandError::Usage(usage()))?;
        let positional_targets = string_values(&matches, "root-target");
        let fleet = string_option(&matches, "fleet").expect("clap requires fleet");
        let flag_target = string_option(&matches, "root");
        let root_target = resolve_root_target(positional_targets, flag_target)?;
        let root_build_target = string_option(&matches, "root-build-target")
            .unwrap_or_else(|| default_root_build_target(&root_target));
        let ready_timeout_seconds = string_option(&matches, "ready-timeout-seconds")
            .map(|value| parse_ready_timeout(&value))
            .transpose()?
            .unwrap_or_else(default_ready_timeout_seconds);

        Ok(Self {
            config_path: string_option(&matches, "config")
                .or_else(|| Some(default_fleet_config_path(&fleet))),
            fleet,
            root_target,
            root_build_target,
            network: string_option(&matches, "network").unwrap_or_else(local_network),
            ready_timeout_seconds,
        })
    }

    /// Convert parsed CLI options into host install options.
    #[must_use]
    pub fn into_install_root_options(self) -> InstallRootOptions {
        InstallRootOptions {
            root_canister: self.root_target,
            root_build_target: self.root_build_target,
            network: self.network,
            ready_timeout_seconds: self.ready_timeout_seconds,
            config_path: self.config_path,
            interactive_config_selection: false,
        }
    }
}

// Build the install parser.
fn install_command() -> ClapCommand {
    ClapCommand::new("install")
        .bin_name("canic install")
        .about("Install and bootstrap a Canic fleet")
        .disable_help_flag(true)
        .arg(
            value_arg("fleet")
                .value_name("fleet")
                .required(true)
                .help("Config-defined fleet name to install"),
        )
        .arg(
            Arg::new("root-target")
                .num_args(0..)
                .value_name("name-or-principal")
                .help("Root canister name or principal to install"),
        )
        .arg(
            value_arg("root")
                .long("root")
                .value_name("name-or-principal")
                .help("Root canister name or principal to install"),
        )
        .arg(
            value_arg("root-build-target")
                .long("root-build-target")
                .value_name("canister-name")
                .help("Canister name used to build the root wasm"),
        )
        .arg(
            value_arg("config")
                .long("config")
                .value_name("canic.toml")
                .help("Canic install config to use"),
        )
        .arg(
            value_arg("network")
                .long("network")
                .value_name("name")
                .help("ICP CLI network to install against"),
        )
        .arg(
            value_arg("ready-timeout-seconds")
                .long("ready-timeout-seconds")
                .value_name("seconds")
                .help("Seconds to wait for root canic_ready"),
        )
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

// Resolve the install root target from positional and flag forms.
fn resolve_root_target(
    positional_targets: Vec<String>,
    flag_target: Option<String>,
) -> Result<String, InstallCommandError> {
    match (positional_targets.as_slice(), flag_target) {
        ([], None) => Ok(DEFAULT_ROOT_TARGET.to_string()),
        ([], Some(target)) => Ok(target),
        ([target], None) => Ok(target.clone()),
        _ => Err(InstallCommandError::ConflictingRootTarget),
    }
}

// Parse the operator-supplied readiness timeout.
fn parse_ready_timeout(value: &str) -> Result<u64, InstallCommandError> {
    value
        .parse::<u64>()
        .map_err(|_| InstallCommandError::InvalidReadyTimeout(value.to_string()))
}

// Resolve the readiness timeout from environment defaults.
fn default_ready_timeout_seconds() -> u64 {
    env::var("READY_TIMEOUT_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(DEFAULT_READY_TIMEOUT_SECONDS)
}

// Use the conventional root build target when the install target is a principal.
fn default_root_build_target(root_target: &str) -> String {
    if Principal::from_text(root_target).is_ok() {
        DEFAULT_ROOT_TARGET.to_string()
    } else {
        root_target.to_string()
    }
}

// Resolve the conventional config path for an explicitly named fleet.
fn default_fleet_config_path(fleet: &str) -> String {
    format!("fleets/{fleet}/canic.toml")
}

// Return install command usage text.
fn usage() -> String {
    let mut command = install_command();
    command.render_help().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOT_PRINCIPAL: &str = "uxrrr-q7777-77774-qaaaq-cai";

    // Ensure install defaults to the conventional local root canister target.
    #[test]
    fn install_defaults_to_root_target() {
        let options = InstallOptions::parse([OsString::from("demo")]).expect("parse defaults");

        assert_eq!(options.fleet, "demo");
        assert_eq!(options.root_target, "root");
        assert_eq!(options.root_build_target, "root");
        assert_eq!(options.network, local_network());
        assert_eq!(options.ready_timeout_seconds, DEFAULT_READY_TIMEOUT_SECONDS);
        assert_eq!(
            options.config_path,
            Some("fleets/demo/canic.toml".to_string())
        );
    }

    // Ensure canister names are used for both build and install by default.
    #[test]
    fn install_accepts_positional_canister_name() {
        let options =
            InstallOptions::parse([OsString::from("demo"), OsString::from("custom_root")])
                .expect("parse root name");

        assert_eq!(options.root_target, "custom_root");
        assert_eq!(options.root_build_target, "custom_root");
    }

    // Ensure principal targets still build the conventional root artifact.
    #[test]
    fn install_accepts_principal_target() {
        let options =
            InstallOptions::parse([OsString::from("demo"), OsString::from(ROOT_PRINCIPAL)])
                .expect("parse principal");

        assert_eq!(options.root_target, ROOT_PRINCIPAL);
        assert_eq!(options.root_build_target, "root");
    }

    // Ensure --root accepts the same target syntax as the positional argument.
    #[test]
    fn install_accepts_root_flag() {
        let options = InstallOptions::parse([
            OsString::from("demo"),
            OsString::from("--root"),
            OsString::from(ROOT_PRINCIPAL),
            OsString::from("--network"),
            OsString::from("local"),
            OsString::from("--ready-timeout-seconds"),
            OsString::from("30"),
        ])
        .expect("parse root flag");

        assert_eq!(options.root_target, ROOT_PRINCIPAL);
        assert_eq!(options.root_build_target, "root");
        assert_eq!(options.network, "local");
        assert_eq!(options.ready_timeout_seconds, 30);
    }

    // Ensure install accepts an explicit project config path.
    #[test]
    fn install_accepts_config_path() {
        let options = InstallOptions::parse([
            OsString::from("demo"),
            OsString::from("--config"),
            OsString::from("fleets/demo/canic.toml"),
        ])
        .expect("parse config path");

        assert_eq!(
            options.config_path,
            Some("fleets/demo/canic.toml".to_string())
        );
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
        assert!(text.contains("Usage: canic install [OPTIONS] <fleet>"));
        assert!(text.contains("<fleet>"));
        assert!(!text.contains("--fleet <name>"));
        assert!(text.contains("[fleet]"));
        assert!(text.contains("name = \"test\""));
    }

    // Ensure custom principal installs can override the build target explicitly.
    #[test]
    fn install_accepts_explicit_root_build_target() {
        let options = InstallOptions::parse([
            OsString::from("demo"),
            OsString::from("--root"),
            OsString::from(ROOT_PRINCIPAL),
            OsString::from("--root-build-target"),
            OsString::from("custom_root"),
        ])
        .expect("parse build target");

        assert_eq!(options.root_target, ROOT_PRINCIPAL);
        assert_eq!(options.root_build_target, "custom_root");
    }

    // Ensure duplicate root target forms are rejected before mutation starts.
    #[test]
    fn install_rejects_duplicate_root_targets() {
        let err = InstallOptions::parse([
            OsString::from("demo"),
            OsString::from("root"),
            OsString::from("--root=root"),
        ])
        .expect_err("duplicate root target should fail");

        assert!(matches!(err, InstallCommandError::ConflictingRootTarget));
    }
}
