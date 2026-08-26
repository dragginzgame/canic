//! Module: canic_cli::install
//!
//! Responsibility: parse `canic install` and delegate fleet bootstrap to the
//! host install runner.
//! Does not own: install planning, controller mutation, canister lifecycle
//! side effects, or deployment state persistence.
//! Boundary: resolves local workspace context, builds host install options, and
//! adds CLI-facing diagnostics.

#[cfg(test)]
mod tests;

use crate::{
    cli::clap::{
        flag_arg, parse_matches, render_usage, required_string, string_option_or_else,
        typed_option, value_arg,
    },
    cli::defaults::{default_icp, local_environment},
    cli::globals::{internal_environment_arg, internal_icp_arg},
    cli::help::print_help_or_version,
    version_text,
};
use canic_core::ids::ReleaseBuildId;
use canic_host::canister_build::CanisterBuildProfile;
use canic_host::icp::{IcpDiagnostic, classify_icp_diagnostic};
use canic_host::icp_config::{IcpConfigError, resolve_current_canic_icp_root};
use canic_host::install_root::{
    InstallRootBlockedError, InstallRootError, InstallRootOptions, RetainedRootRepairAdoption,
    install_root, preflight_install_root,
};
use clap::Command as ClapCommand;
use std::{ffi::OsString, path::PathBuf};
use thiserror::Error as ThisError;

const DEFAULT_ROOT_TARGET: &str = "root";
const EXPECTED_PLAN_DIGEST_ARG: &str = "expected-plan-digest";
const FLEET_INPUT_ARG: &str = "fleet-input";
const PREFLIGHT_ARG: &str = "preflight";
const RELEASE_BUILD_ARG: &str = "release-build";
const RETAINED_ROOT_REPAIR_ARG: &str = "adopt-retained-root-repair";
const INSTALL_HELP_AFTER: &str = "\
Examples:
  canic install toko toko-local --fleet-input deployments/toko-local.toml
  canic install toko toko-test --fleet-input deployments/toko-test.toml --release-build <ID>
  canic install toko toko-mainnet --fleet-input deployments/toko-mainnet.toml --preflight

Creates a fresh Fleet from the App config and required operator-owned Fleet input.
Before building, Canic refreshes missing or invalid mainnet catalog evidence,
resolves the effective ICP identity, rejects anonymous or unusable credentials,
and requires that Principal to equal the Fleet input operator. For encrypted
non-interactive identities, set CANIC_ICP_IDENTITY_PASSWORD_FILE to an absolute
operator-owned password file. `--preflight` requires an incomplete retained
install session and stops before operational authority publication or IC updates.";

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
    icp: String,
    environment: String,
    expected_plan_digest: Option<String>,
    preflight: bool,
    profile: Option<CanisterBuildProfile>,
    release_build_id: Option<ReleaseBuildId>,
    retained_root_repair_adoption: Option<RetainedRootRepairAdoption>,
    fleet_input: PathBuf,
}

impl InstallOptions {
    fn parse<I>(args: I) -> Result<Self, InstallCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches = parse_matches(install_command(), args)
            .map_err(|error| InstallCommandError::Usage(format!("{error}\n{}", usage())))?;
        Ok(Self {
            app: required_string(&matches, "app"),
            fleet: required_string(&matches, "fleet"),
            icp: string_option_or_else(&matches, "icp", default_icp),
            environment: string_option_or_else(&matches, "environment", local_environment),
            expected_plan_digest: matches.get_one::<String>(EXPECTED_PLAN_DIGEST_ARG).cloned(),
            preflight: matches.get_flag(PREFLIGHT_ARG),
            profile: typed_option(&matches, "profile"),
            release_build_id: typed_option(&matches, RELEASE_BUILD_ARG),
            retained_root_repair_adoption: typed_option(&matches, RETAINED_ROOT_REPAIR_ARG),
            fleet_input: PathBuf::from(required_string(&matches, FLEET_INPUT_ARG)),
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
            icp_executable: self.icp,
            environment: self.environment,
            fleet_name: self.fleet,
            icp_root,
            build_profile: self.profile,
            release_build_id: self.release_build_id,
            config_path: Some(config_path),
            fleet_install_input_path: Some(self.fleet_input),
            expected_fresh_fleet_plan_digest: self.expected_plan_digest,
            admitted_fresh_fleet_plan_digest: None,
            expected_app: Some(self.app),
            retained_root_repair_adoption: self.retained_root_repair_adoption,
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
        .override_usage("canic install <app> <fleet> --fleet-input <PATH>")
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
            value_arg(EXPECTED_PLAN_DIGEST_ARG)
                .long(EXPECTED_PLAN_DIGEST_ARG)
                .value_name("SHA256")
                .num_args(1)
                .value_parser(parse_plan_digest)
                .help("Require the exact canonical pre-effect plan digest"),
        )
        .arg(
            value_arg(FLEET_INPUT_ARG)
                .long(FLEET_INPUT_ARG)
                .value_name("PATH")
                .required(true)
                .num_args(1)
                .help("Operator-owned Fleet placement, admission, limit, and funding input TOML"),
        )
        .arg(
            value_arg("profile")
                .long("profile")
                .value_name("debug|fast|release")
                .num_args(1)
                .value_parser(clap::value_parser!(CanisterBuildProfile))
                .help("Canister wasm build profile; defaults to release"),
        )
        .arg(
            flag_arg(PREFLIGHT_ARG)
                .long(PREFLIGHT_ARG)
                .help(
                    "Run exact retained-recovery installer preparation through a verified bundle checkpoint without operational authority publication or IC updates",
                ),
        )
        .arg(
            value_arg(RELEASE_BUILD_ARG)
                .long(RELEASE_BUILD_ARG)
                .value_name("ID")
                .num_args(1)
                .value_parser(clap::value_parser!(ReleaseBuildId))
                .help("Reuse one finalized release build instead of compiling artifacts"),
        )
        .arg(
            value_arg(RETAINED_ROOT_REPAIR_ARG)
                .long(RETAINED_ROOT_REPAIR_ARG)
                .value_name("ROOT,POOL=LIVE_WASM,SUCCESSOR_WASM")
                .num_args(1)
                .value_parser(clap::value_parser!(RetainedRootRepairAdoption))
                .help(
                    "Authorize one exact retained Root repair; resolves adjacent .did sidecars first and retains content-addressed evidence before effects",
                ),
        )
        .arg(internal_icp_arg())
        .arg(internal_environment_arg())
        .after_help(INSTALL_HELP_AFTER)
}

fn parse_plan_digest(value: &str) -> Result<String, String> {
    let valid = value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'));
    valid.then(|| value.to_string()).ok_or_else(|| {
        "plan digest must contain exactly 64 lowercase hexadecimal characters".to_string()
    })
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
    let preflight = options.preflight;
    let options = options.into_install_root_options_with_icp_root(icp_root);
    let result = if preflight {
        preflight_install_root(options)
    } else {
        install_root(options)
    };
    result.map_err(|err| install_error_with_context(err, &fleet, &environment))
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
                "If this Fleet or canister already exists, run `canic --environment {environment} info list {fleet}` and `canic --environment {environment} medic fleet {fleet}` before retrying. `canic install` is for fresh Fleet creation, not code-only updates."
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
