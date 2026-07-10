//! Module: canic_cli::install
//!
//! Responsibility: parse `canic install` and delegate fleet bootstrap to the
//! host install runner.
//! Does not own: install planning, controller mutation, canister lifecycle
//! side effects, or deployment state persistence.
//! Boundary: resolves local project context, builds host install options, and
//! adds CLI-facing diagnostics.

#[cfg(test)]
mod tests;

use crate::{
    cli::clap::{
        parse_matches, render_usage, required_string, string_option_or_else, typed_option,
        value_arg,
    },
    cli::defaults::local_network,
    cli::globals::internal_network_arg,
    cli::help::print_help_or_version,
    version_text,
};
use canic_host::canister_build::CanisterBuildProfile;
use canic_host::icp_config::resolve_current_canic_icp_root;
use canic_host::install_root::{InstallRootOptions, install_root};
use clap::Command as ClapCommand;
use std::{ffi::OsString, path::PathBuf};
use thiserror::Error as ThisError;

const DEFAULT_ROOT_TARGET: &str = "root";
const DEFAULT_READY_TIMEOUT_SECONDS: u64 = 120;
const INSTALL_HELP_AFTER: &str = "\
Examples:
  canic install test
  canic install --profile fast test

canic install uses fleets/<fleet>/canic.toml.
Use it for fresh local creation or recreating local state after the ICP CLI
replica lost canisters. For an existing canister that only needs new Wasm,
inspect with canic info list and canic medic deployment, then use the
project upgrade flow.

Install removes its transient target/canic-wasm Cargo cache after canonical
.icp artifacts are written. Set CANIC_KEEP_WASM_BUILD_CACHE=1 to retain it for
faster repeated local installs.

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

    #[error("{source}\n\nHint: {hint}")]
    InstallHint {
        source: Box<dyn std::error::Error>,
        hint: String,
    },
}

///
/// InstallOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct InstallOptions {
    fleet: String,
    network: String,
    profile: Option<CanisterBuildProfile>,
}

impl InstallOptions {
    fn parse<I>(args: I) -> Result<Self, InstallCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(install_command(), args)
            .map_err(|_| InstallCommandError::Usage(usage()))?;
        let fleet = required_string(&matches, "fleet");

        Ok(Self {
            fleet,
            network: string_option_or_else(&matches, "network", local_network),
            profile: typed_option(&matches, "profile"),
        })
    }

    fn into_install_root_options_with_icp_root(
        self,
        icp_root: Option<PathBuf>,
    ) -> InstallRootOptions {
        let config_path = icp_root
            .as_deref()
            .map(|root| root.join(default_fleet_config_path(&self.fleet)))
            .filter(|path| path.is_file())
            .map_or_else(
                || default_fleet_config_path(&self.fleet),
                |path| path.display().to_string(),
            );
        InstallRootOptions {
            root_canister: DEFAULT_ROOT_TARGET.to_string(),
            root_build_target: DEFAULT_ROOT_TARGET.to_string(),
            network: self.network,
            deployment_name: None,
            icp_root,
            build_profile: self.profile,
            ready_timeout_seconds: DEFAULT_READY_TIMEOUT_SECONDS,
            config_path: Some(config_path),
            expected_fleet: Some(self.fleet),
            interactive_config_selection: false,
            deployment_plan_override: None,
            artifact_promotion_plan_override: None,
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
        .arg(
            value_arg("profile")
                .long("profile")
                .value_name("debug|fast|release")
                .num_args(1)
                .value_parser(clap::value_parser!(CanisterBuildProfile))
                .help("Canister wasm build profile; defaults to CANIC_WASM_PROFILE or release"),
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
    let fleet = options.fleet.clone();
    let network = options.network.clone();
    let icp_root = resolve_current_canic_icp_root().ok();
    install_root(options.into_install_root_options_with_icp_root(icp_root))
        .map_err(|err| install_error_with_context(err, &fleet, &network))
}

fn default_fleet_config_path(fleet: &str) -> String {
    format!("fleets/{fleet}/canic.toml")
}

fn usage() -> String {
    render_usage(install_command)
}

fn install_error_with_context(
    err: Box<dyn std::error::Error>,
    fleet: &str,
    network: &str,
) -> InstallCommandError {
    if install_error_needs_existing_deployment_hint(&err.to_string()) {
        return InstallCommandError::InstallHint {
            source: err,
            hint: format!(
                "If this deployment or canister already exists, run `canic --network {network} info list {fleet}` and `canic --network {network} medic deployment {fleet}` before retrying. For code-only changes, use the project upgrade flow instead of another fresh install."
            ),
        };
    }

    InstallCommandError::Install(err)
}

fn install_error_needs_existing_deployment_hint(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    lower.contains("blocked install")
        || lower.contains("preflight blocked")
        || (lower.contains("already")
            && (lower.contains("install")
                || lower.contains("installed")
                || lower.contains("canister")))
}
