use crate::version_text;
use candid::Principal;
use canic_installer::install_root::{DEFAULT_FLEET_NAME, InstallRootOptions, install_root};
use std::{env, ffi::OsString};
use thiserror::Error as ThisError;

const DEFAULT_ROOT_TARGET: &str = "root";
const DEFAULT_READY_TIMEOUT_SECONDS: u64 = 120;

///
/// InstallCommandError
///

#[derive(Debug, ThisError)]
pub enum InstallCommandError {
    #[error("{0}")]
    Usage(&'static str),

    #[error("unknown option {0}")]
    UnknownOption(String),

    #[error("option {0} requires a value")]
    MissingValue(&'static str),

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
    pub fleet_name: String,
    pub root_target: String,
    pub root_build_target: String,
    pub network: String,
    pub ready_timeout_seconds: u64,
    pub config_path: Option<String>,
}

impl InstallOptions {
    /// Parse install options from CLI arguments and environment defaults.
    pub fn parse<I>(args: I) -> Result<Self, InstallCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let mut root_target = None;
        let mut root_build_target = None;
        let mut fleet_name =
            env::var("CANIC_FLEET").unwrap_or_else(|_| DEFAULT_FLEET_NAME.to_string());
        let mut network = env::var("DFX_NETWORK").unwrap_or_else(|_| "local".to_string());
        let mut config_path = None;
        let mut ready_timeout_seconds = env::var("READY_TIMEOUT_SECONDS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(DEFAULT_READY_TIMEOUT_SECONDS);

        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            let arg = arg
                .into_string()
                .map_err(|_| InstallCommandError::Usage(usage()))?;

            if let Some(value) = arg.strip_prefix("--root=") {
                set_root_target(&mut root_target, value.to_string())?;
                continue;
            }
            if let Some(value) = arg.strip_prefix("--root-build-target=") {
                root_build_target = Some(value.to_string());
                continue;
            }
            if let Some(value) = arg.strip_prefix("--fleet=") {
                fleet_name = value.to_string();
                continue;
            }
            if let Some(value) = arg.strip_prefix("--network=") {
                network = value.to_string();
                continue;
            }
            if let Some(value) = arg.strip_prefix("--config=") {
                config_path = Some(value.to_string());
                continue;
            }
            if let Some(value) = arg.strip_prefix("--ready-timeout-seconds=") {
                ready_timeout_seconds = parse_ready_timeout(value)?;
                continue;
            }

            match arg.as_str() {
                "--root" => {
                    let value = next_value(&mut args, "--root")?;
                    set_root_target(&mut root_target, value)?;
                }
                "--root-build-target" => {
                    root_build_target = Some(next_value(&mut args, "--root-build-target")?);
                }
                "--fleet" => {
                    fleet_name = next_value(&mut args, "--fleet")?;
                }
                "--network" => {
                    network = next_value(&mut args, "--network")?;
                }
                "--config" => {
                    config_path = Some(next_value(&mut args, "--config")?);
                }
                "--ready-timeout-seconds" => {
                    let value = next_value(&mut args, "--ready-timeout-seconds")?;
                    ready_timeout_seconds = parse_ready_timeout(&value)?;
                }
                "--help" | "-h" => return Err(InstallCommandError::Usage(usage())),
                _ if arg.starts_with('-') => return Err(InstallCommandError::UnknownOption(arg)),
                _ => set_root_target(&mut root_target, arg)?,
            }
        }

        let root_target = root_target.unwrap_or_else(|| DEFAULT_ROOT_TARGET.to_string());
        let root_build_target =
            root_build_target.unwrap_or_else(|| default_root_build_target(&root_target));

        Ok(Self {
            fleet_name,
            root_target,
            root_build_target,
            network,
            ready_timeout_seconds,
            config_path,
        })
    }

    /// Convert parsed CLI options into installer options.
    #[must_use]
    pub fn into_install_root_options(self) -> InstallRootOptions {
        InstallRootOptions {
            fleet_name: self.fleet_name,
            root_canister: self.root_target,
            root_build_target: self.root_build_target,
            network: self.network,
            ready_timeout_seconds: self.ready_timeout_seconds,
            config_path: self.config_path,
            interactive_config_selection: true,
        }
    }
}

/// Run the root install workflow.
pub fn run<I>(args: I) -> Result<(), InstallCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if args
        .first()
        .and_then(|arg| arg.to_str())
        .is_some_and(|arg| matches!(arg, "help" | "--help" | "-h"))
    {
        println!("{}", usage());
        return Ok(());
    }
    if args
        .first()
        .and_then(|arg| arg.to_str())
        .is_some_and(|arg| matches!(arg, "version" | "--version" | "-V"))
    {
        println!("{}", version_text());
        return Ok(());
    }

    let options = InstallOptions::parse(args)?;
    install_root(options.into_install_root_options()).map_err(InstallCommandError::from)
}

// Set the root target once, accepting either a canister name or principal text.
fn set_root_target(target: &mut Option<String>, value: String) -> Result<(), InstallCommandError> {
    if target.replace(value).is_some() {
        return Err(InstallCommandError::ConflictingRootTarget);
    }

    Ok(())
}

// Read the next required option value.
fn next_value<I>(args: &mut I, option: &'static str) -> Result<String, InstallCommandError>
where
    I: Iterator<Item = OsString>,
{
    args.next()
        .and_then(|value| value.into_string().ok())
        .ok_or(InstallCommandError::MissingValue(option))
}

// Parse the operator-supplied readiness timeout.
fn parse_ready_timeout(value: &str) -> Result<u64, InstallCommandError> {
    value
        .parse::<u64>()
        .map_err(|_| InstallCommandError::InvalidReadyTimeout(value.to_string()))
}

// Use the conventional root build target when the install target is a principal.
fn default_root_build_target(root_target: &str) -> String {
    if Principal::from_text(root_target).is_ok() {
        DEFAULT_ROOT_TARGET.to_string()
    } else {
        root_target.to_string()
    }
}

// Return install command usage text.
const fn usage() -> &'static str {
    "usage: canic install [root-target] [--fleet <name>] [--root <name-or-principal>] [--root-build-target <dfx-canister-name>] [--config <canic.toml>] [--network <name>] [--ready-timeout-seconds <seconds>]"
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOT_PRINCIPAL: &str = "uxrrr-q7777-77774-qaaaq-cai";

    // Ensure install defaults to the conventional local root canister target.
    #[test]
    fn install_defaults_to_root_target() {
        let options = InstallOptions::parse([]).expect("parse defaults");

        assert_eq!(options.root_target, "root");
        assert_eq!(options.root_build_target, "root");
        assert_eq!(options.fleet_name, DEFAULT_FLEET_NAME);
        assert_eq!(options.network, "local");
        assert_eq!(options.ready_timeout_seconds, DEFAULT_READY_TIMEOUT_SECONDS);
        assert_eq!(options.config_path, None);
    }

    // Ensure canister names are used for both build and install by default.
    #[test]
    fn install_accepts_positional_canister_name() {
        let options =
            InstallOptions::parse([OsString::from("custom_root")]).expect("parse root name");

        assert_eq!(options.root_target, "custom_root");
        assert_eq!(options.root_build_target, "custom_root");
    }

    // Ensure principal targets still build the conventional root artifact.
    #[test]
    fn install_accepts_principal_target() {
        let options =
            InstallOptions::parse([OsString::from(ROOT_PRINCIPAL)]).expect("parse principal");

        assert_eq!(options.root_target, ROOT_PRINCIPAL);
        assert_eq!(options.root_build_target, "root");
    }

    // Ensure --root accepts the same target syntax as the positional argument.
    #[test]
    fn install_accepts_root_flag() {
        let options = InstallOptions::parse([
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
            OsString::from("--config"),
            OsString::from("canisters/demo/canic.toml"),
            OsString::from("--fleet"),
            OsString::from("demo"),
        ])
        .expect("parse config path");

        assert_eq!(
            options.config_path,
            Some("canisters/demo/canic.toml".to_string())
        );
        assert_eq!(options.fleet_name, "demo");
    }

    // Ensure custom principal installs can override the build target explicitly.
    #[test]
    fn install_accepts_explicit_root_build_target() {
        let options = InstallOptions::parse([
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
        let err = InstallOptions::parse([OsString::from("root"), OsString::from("--root=root")])
            .expect_err("duplicate root target should fail");

        assert!(matches!(err, InstallCommandError::ConflictingRootTarget));
    }
}
