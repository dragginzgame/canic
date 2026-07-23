//! Module: canic_cli::deploy::plan::command
//!
//! Responsibility: define and parse the deploy-plan command input boundary.
//! Does not own: plan construction, report rendering, or deployment mutation.
//! Boundary: resolves CLI options and local roots for the plan orchestrator.

use crate::{
    cli::{
        clap::{
            flag_arg, parse_matches, path_option, render_usage, required_string,
            string_option_or_else, typed_option,
        },
        defaults::local_environment,
        globals::internal_environment_arg,
    },
    deploy::{DeployCommandError, value_arg},
};
use std::{ffi::OsString, path::PathBuf};

use canic_host::{
    canister_build::CanisterBuildProfile,
    release_set::{icp_root as resolve_icp_root, workspace_root as resolve_workspace_root},
};
use clap::Command as ClapCommand;

pub(super) const REPORT_COMMAND: &str = "canic deploy plan";

const DEPLOYMENT_ARG: &str = "deployment";
const JSON_ARG: &str = "json";
const OUT_ARG: &str = "out";
const CONFIG_ARG: &str = "config";
const BUILD_PROFILE_ARG: &str = "build-profile";

const DEPLOY_PLAN_HELP_AFTER: &str = "\
Examples:
  canic deploy plan demo-local
  canic deploy plan demo-local --json
  canic deploy plan demo-local --out deployment-plan.json
  canic deploy plan demo-local --config apps/demo/canic.toml

Builds a deterministic planning report from local project config. The command
does not install, upgrade, create canisters, write deployment truth, update
installed deployment records, or call live IC state. Future-apply preview rows
are proposed operation labels only; they are not executed and are not apply
operation objects. JSON output is a DeploymentPlanReport, not an EvidenceEnvelope,
deployment truth, or authorization to mutate. --out writes JSON only and fails if
the requested path already exists or its parent directory is missing.";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::deploy) struct DeployPlanOptions {
    pub(in crate::deploy) deployment: String,
    pub(in crate::deploy) environment: String,
    pub(in crate::deploy) json: bool,
    pub(in crate::deploy) out: Option<PathBuf>,
    pub(in crate::deploy) config: Option<PathBuf>,
    pub(in crate::deploy) build_profile: CanisterBuildProfile,
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
        Ok(Self {
            deployment: required_string(&matches, DEPLOYMENT_ARG),
            environment: string_option_or_else(&matches, "environment", local_environment),
            json: matches.get_flag(JSON_ARG),
            out: path_option(&matches, OUT_ARG),
            config: path_option(&matches, CONFIG_ARG),
            build_profile: typed_option(&matches, BUILD_PROFILE_ARG)
                .unwrap_or(CanisterBuildProfile::Release),
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
        .about("Explain the deterministic deployment plan without mutation")
        .disable_help_flag(true)
        .override_usage("canic deploy plan <deployment>")
        .arg(deployment_arg())
        .arg(json_arg())
        .arg(out_arg())
        .arg(config_arg())
        .arg(build_profile_arg())
        .arg(internal_environment_arg())
        .after_help(DEPLOY_PLAN_HELP_AFTER)
}

fn deployment_arg() -> clap::Arg {
    value_arg(DEPLOYMENT_ARG)
        .value_name(DEPLOYMENT_ARG)
        .required(true)
        .help("Deployment target name to plan")
}

fn json_arg() -> clap::Arg {
    flag_arg(JSON_ARG)
        .long(JSON_ARG)
        .help("Print JSON DeploymentPlanReport to stdout")
}

fn out_arg() -> clap::Arg {
    value_arg(OUT_ARG)
        .long(OUT_ARG)
        .value_name("path")
        .num_args(1)
        .help("Write JSON DeploymentPlanReport to a new file")
}

fn config_arg() -> clap::Arg {
    value_arg(CONFIG_ARG)
        .long(CONFIG_ARG)
        .value_name("path")
        .num_args(1)
        .help("Fleet config path used to build the desired plan")
}

fn build_profile_arg() -> clap::Arg {
    value_arg(BUILD_PROFILE_ARG)
        .long(BUILD_PROFILE_ARG)
        .value_name("debug|fast|release")
        .num_args(1)
        .value_parser(clap::value_parser!(CanisterBuildProfile))
        .help("Expected canister wasm build profile")
}

pub(in crate::deploy) fn usage() -> String {
    render_usage(command)
}
