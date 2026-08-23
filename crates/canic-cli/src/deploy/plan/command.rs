//! Module: canic_cli::deploy::plan::command
//!
//! Responsibility: define and parse the deploy-plan command input boundary.
//! Does not own: plan construction, report rendering, or deployment mutation.
//! Boundary: resolves CLI options and local roots for the plan orchestrator.

use crate::{
    cli::{
        clap::{
            flag_arg, parse_matches, path_option, render_usage, required_path, required_string,
            string_option_or_else, typed_option,
        },
        defaults::{default_icp, local_environment},
        globals::{internal_environment_arg, internal_icp_arg},
    },
    deploy::{DeployCommandError, value_arg},
};
use std::{ffi::OsString, path::PathBuf};

use canic_core::{bootstrap::compiled::validate_app_name, ids::ReleaseBuildId};
use canic_host::{
    canister_build::CanisterBuildProfile,
    release_set::{icp_root as resolve_icp_root, workspace_root as resolve_workspace_root},
};
use clap::Command as ClapCommand;

pub(super) const REPORT_COMMAND: &str = "canic deploy plan";

const APP_ARG: &str = "app";
const FLEET_ARG: &str = "fleet";
const FLEET_INPUT_ARG: &str = "fleet-input";
const JSON_ARG: &str = "json";
const OUT_ARG: &str = "out";
const PROFILE_ARG: &str = "profile";
const REFRESH_CATALOG_ARG: &str = "refresh-catalog";
const RELEASE_BUILD_ARG: &str = "release-build";

const DEPLOY_PLAN_HELP_AFTER: &str = "\
Examples:
  canic deploy plan demo-local --app demo --fleet-input deployments/demo-local.toml
  canic --environment ic deploy plan demo --app demo --fleet-input deployments/demo-ic.toml --refresh-catalog

Read-only deployment planning. Every plan verifies the active ICP identity and
queries its relevant ledger account and balance. By default, existing validated
catalog evidence is used; --refresh-catalog may also issue read-only public NNS
Registry queries and update Canic's private .canic/ic-query cache when it is
missing or invalid. No mode builds, changes deployment state, or performs an IC
update call. Put the top-level --environment before deploy to select the exact
ICP target.";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::deploy) struct DeployPlanOptions {
    pub(in crate::deploy) fleet: String,
    pub(in crate::deploy) app: String,
    pub(in crate::deploy) environment: String,
    pub(in crate::deploy) fleet_input: PathBuf,
    pub(in crate::deploy) icp: String,
    pub(in crate::deploy) json: bool,
    pub(in crate::deploy) out: Option<PathBuf>,
    pub(in crate::deploy) profile: Option<CanisterBuildProfile>,
    pub(in crate::deploy) refresh_catalog: bool,
    pub(in crate::deploy) release_build_id: Option<ReleaseBuildId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::deploy) struct DeployPlanRoots {
    pub(in crate::deploy) workspace_root: PathBuf,
    pub(in crate::deploy) icp_root: PathBuf,
}

impl DeployPlanOptions {
    pub(in crate::deploy) fn parse<I>(args: I) -> Result<Self, DeployCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches =
            parse_matches(command(), args).map_err(|_| DeployCommandError::Usage(usage()))?;
        let app = required_string(&matches, APP_ARG);
        validate_app_name(&app).map_err(|issue| {
            DeployCommandError::Usage(format!("invalid App name {app:?}: {issue}\n\n{}", usage()))
        })?;
        Ok(Self {
            fleet: required_string(&matches, FLEET_ARG),
            app,
            environment: string_option_or_else(&matches, "environment", local_environment),
            fleet_input: required_path(&matches, FLEET_INPUT_ARG),
            icp: string_option_or_else(&matches, "icp", default_icp),
            json: matches.get_flag(JSON_ARG),
            out: path_option(&matches, OUT_ARG),
            profile: typed_option(&matches, PROFILE_ARG),
            refresh_catalog: matches.get_flag(REFRESH_CATALOG_ARG),
            release_build_id: typed_option(&matches, RELEASE_BUILD_ARG),
        })
    }
}

impl DeployPlanRoots {
    pub(super) fn discover() -> Result<Self, DeployCommandError> {
        Ok(Self {
            workspace_root: resolve_workspace_root().map_err(DeployCommandError::from)?,
            icp_root: resolve_icp_root()
                .map_err(|source| DeployCommandError::Check(Box::new(source)))?,
        })
    }
}

pub(in crate::deploy) fn command() -> ClapCommand {
    ClapCommand::new("plan")
        .bin_name(REPORT_COMMAND)
        .about("Explain the deterministic plan without deployment mutation")
        .disable_help_flag(true)
        .override_usage("canic deploy plan <fleet> --app <app> --fleet-input <PATH>")
        .arg(fleet_arg())
        .arg(app_arg())
        .arg(fleet_input_arg())
        .arg(json_arg())
        .arg(out_arg())
        .arg(profile_arg())
        .arg(refresh_catalog_arg())
        .arg(release_build_arg())
        .arg(internal_environment_arg())
        .arg(internal_icp_arg())
        .after_help(DEPLOY_PLAN_HELP_AFTER)
}

fn fleet_arg() -> clap::Arg {
    value_arg(FLEET_ARG)
        .value_name(FLEET_ARG)
        .required(true)
        .help("Fleet name to plan")
}

fn app_arg() -> clap::Arg {
    value_arg(APP_ARG)
        .long(APP_ARG)
        .value_name(APP_ARG)
        .num_args(1)
        .required(true)
        .help("Source App identity under apps/<app>/canic.toml")
}

fn json_arg() -> clap::Arg {
    flag_arg(JSON_ARG)
        .long(JSON_ARG)
        .help("Print JSON DeploymentPlanReport to stdout")
}

fn fleet_input_arg() -> clap::Arg {
    value_arg(FLEET_INPUT_ARG)
        .long(FLEET_INPUT_ARG)
        .value_name("PATH")
        .num_args(1)
        .required(true)
        .help("Operator-owned Fleet placement, admission, limit, and funding input TOML")
}

fn out_arg() -> clap::Arg {
    value_arg(OUT_ARG)
        .long(OUT_ARG)
        .value_name("path")
        .num_args(1)
        .help("Write JSON DeploymentPlanReport to a new file")
}

fn profile_arg() -> clap::Arg {
    value_arg(PROFILE_ARG)
        .long(PROFILE_ARG)
        .value_name("debug|fast|release")
        .num_args(1)
        .value_parser(clap::value_parser!(CanisterBuildProfile))
        .help("Canister wasm build profile; defaults to release")
}

fn refresh_catalog_arg() -> clap::Arg {
    flag_arg(REFRESH_CATALOG_ARG)
        .long(REFRESH_CATALOG_ARG)
        .help("Refresh a missing or invalid mainnet Subnet Catalog with read-only Registry queries")
}

fn release_build_arg() -> clap::Arg {
    value_arg(RELEASE_BUILD_ARG)
        .long(RELEASE_BUILD_ARG)
        .value_name("ID")
        .num_args(1)
        .value_parser(clap::value_parser!(ReleaseBuildId))
        .help("Use one exact finalized release build as the plan source")
}

pub(in crate::deploy) fn usage() -> String {
    render_usage(command)
}
