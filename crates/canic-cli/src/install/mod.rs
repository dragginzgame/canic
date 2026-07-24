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
    cli::defaults::local_environment,
    cli::globals::internal_environment_arg,
    cli::help::print_help_or_version,
    version_text,
};
use canic_host::canister_build::CanisterBuildProfile;
use canic_host::icp::{IcpDiagnostic, classify_icp_diagnostic};
use canic_host::icp_config::{IcpConfigError, resolve_current_canic_icp_root};
use canic_host::install_root::{
    InstallRootBlockedError, InstallRootError, InstallRootOptions, install_root,
};
use clap::Command as ClapCommand;
use std::{ffi::OsString, path::PathBuf};
use thiserror::Error as ThisError;

const DEFAULT_ROOT_TARGET: &str = "root";
const INSTALL_HELP_AFTER: &str = "\
Examples:
  canic install toko toko-local
  canic install toko toko-local --profile fast

canic install uses apps/<app>/canic.toml.
Use it for fresh local creation or recreating local state after the ICP CLI
replica lost canisters. For an existing canister that only needs new Wasm,
inspect with canic info list and canic medic deployment, then use the
project upgrade flow.

Install removes its transient target/canic-wasm Cargo cache after canonical
.icp artifacts are written. Advanced Cargo callers can select their own target
with the standard CARGO_TARGET_DIR input.

The selected canic.toml must include:
  [app]
  name = \"test\"";

///
/// InstallCommandError
///

#[derive(Debug, ThisError)]
pub enum InstallCommandError {
    #[error("{0}")]
    Usage(String),

    #[error("failed to resolve ICP project root: {0}")]
    IcpRoot(#[from] IcpConfigError),

    #[error(transparent)]
    Install(#[from] InstallRootError),

    #[error("{source}\n\nHint: {hint}")]
    InstallHint {
        source: InstallRootError,
        hint: String,
    },
}

///
/// InstallOptions
///

#[derive(Clone, Debug, Eq, PartialEq)]
struct InstallOptions {
    app: String,
    fleet: String,
    environment: String,
    profile: Option<CanisterBuildProfile>,
}

impl InstallOptions {
    fn parse<I>(args: I) -> Result<Self, InstallCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(install_command(), args)
            .map_err(|_| InstallCommandError::Usage(usage()))?;
        Ok(Self {
            app: required_string(&matches, "app"),
            fleet: required_string(&matches, "fleet"),
            environment: string_option_or_else(&matches, "environment", local_environment),
            profile: typed_option(&matches, "profile"),
        })
    }

    fn into_install_root_options_with_icp_root(
        self,
        icp_root: Option<PathBuf>,
    ) -> InstallRootOptions {
        let config_path = icp_root
            .as_deref()
            .map(|root| root.join(default_app_config_path(&self.app)))
            .filter(|path| path.is_file())
            .map_or_else(
                || default_app_config_path(&self.app),
                |path| path.display().to_string(),
            );
        InstallRootOptions {
            root_canister: DEFAULT_ROOT_TARGET.to_string(),
            root_build_target: DEFAULT_ROOT_TARGET.to_string(),
            environment: self.environment,
            fleet_name: self.fleet,
            icp_root,
            build_profile: self.profile,
            config_path: Some(config_path),
            expected_app: Some(self.app),
            interactive_config_selection: false,
            deployment_plan_override: None,
        }
    }
}

fn install_command() -> ClapCommand {
    ClapCommand::new("install")
        .bin_name("canic install")
        .about("Install and bootstrap a Canic fleet")
        .disable_help_flag(true)
        .override_usage("canic install <app> <fleet>")
        .arg(
            value_arg("app")
                .value_name("app")
                .required(true)
                .help("Source App identity under apps/<app>/canic.toml"),
        )
        .arg(
            value_arg("fleet")
                .value_name("fleet")
                .required(true)
                .help("Operator-facing name for the installed Fleet"),
        )
        .arg(
            value_arg("profile")
                .long("profile")
                .value_name("debug|fast|release")
                .num_args(1)
                .value_parser(clap::value_parser!(CanisterBuildProfile))
                .help("Canister wasm build profile; defaults to release"),
        )
        .arg(internal_environment_arg())
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
    let environment = options.environment.clone();
    let icp_root = Some(resolve_current_canic_icp_root()?);
    install_root(options.into_install_root_options_with_icp_root(icp_root))
        .map_err(|err| install_error_with_context(err, &fleet, &environment))
}

fn default_app_config_path(app: &str) -> String {
    format!("apps/{app}/canic.toml")
}

fn usage() -> String {
    render_usage(install_command)
}

fn install_error_with_context(
    err: InstallRootError,
    fleet: &str,
    environment: &str,
) -> InstallCommandError {
    if install_error_needs_existing_deployment_hint(&err) {
        return InstallCommandError::InstallHint {
            source: err,
            hint: format!(
                "If this deployment or canister already exists, run `canic --environment {environment} info list {fleet}` and `canic --environment {environment} medic deployment {fleet}` before retrying. For code-only changes, use the project upgrade flow instead of another fresh install."
            ),
        };
    }

    InstallCommandError::Install(err)
}

fn install_error_needs_existing_deployment_hint(error: &(dyn std::error::Error + 'static)) -> bool {
    let mut source = Some(error);
    while let Some(error) = source {
        if error.downcast_ref::<InstallRootBlockedError>().is_some() {
            return true;
        }
        source = error.source();
    }

    matches!(
        classify_icp_diagnostic(&error.to_string()),
        Some(IcpDiagnostic::AlreadyInstalled)
    )
}
