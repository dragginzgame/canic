mod catalog;
mod check;
mod command;
mod compare;
mod inspect;
mod output_format;
mod plan;
mod recovery;
mod resume_report;
mod truth;

pub use crate::cli::clap::value_arg;
use command::{FLEET_ARG, PROFILE_ARG};
pub use command::{deploy_command, deploy_truth_leaf_command, usage};
use output_format::JsonTextOutputFormat;

use crate::{
    cli::{
        clap::{
            parse_matches, parse_subcommand, required_string, string_option_or_else, typed_option,
        },
        defaults::{default_icp, local_environment},
        help::print_help_or_version,
    },
    evidence_support::current_evidence_timestamp,
    version_text,
};
use canic_host::{
    canister_build::CanisterBuildProfile,
    deployment_truth::DeploymentCheckV1,
    fleet_catalog::read_fleet_catalog_entry_from_root,
    icp_config::{IcpConfigError, resolve_current_canic_icp_root},
    install_root::{InstallRootError, InstallRootOptions, check_install_deployment_truth},
    release_set::WorkspaceDiscoveryError,
};
use clap::Command as ClapCommand;
use serde::de::DeserializeOwned;
use std::{ffi::OsString, fs, path::PathBuf};
use thiserror::Error as ThisError;

const DEFAULT_ROOT_TARGET: &str = "root";

///
/// DeployCommandError
///
#[derive(Debug, ThisError)]
pub enum DeployCommandError {
    #[error("{0}")]
    Usage(String),

    #[error("failed to resolve ICP project root: {0}")]
    IcpRoot(#[from] IcpConfigError),

    #[error("failed to resolve Cargo workspace: {0}")]
    WorkspaceRoot(#[from] WorkspaceDiscoveryError),

    #[error(transparent)]
    Check(#[from] Box<dyn std::error::Error>),

    #[error(transparent)]
    Install(#[from] InstallRootError),

    #[error("failed to write deployment plan output: {0}")]
    PlanOutput(#[source] Box<dyn std::error::Error>),

    #[error("deployment plan blocked: {0}")]
    PlanBlocked(String),

    #[error("deployment truth check blocked: {0}")]
    Blocked(String),
}

impl DeployCommandError {
    pub const fn exit_code(&self) -> u8 {
        match self {
            Self::Usage(_) | Self::IcpRoot(_) | Self::WorkspaceRoot(_) | Self::PlanOutput(_) => 2,
            Self::Check(_) | Self::Install(_) | Self::PlanBlocked(_) | Self::Blocked(_) => 1,
        }
    }

    pub const fn suppress_stderr(&self) -> bool {
        matches!(self, Self::PlanBlocked(_))
    }
}

///
/// DeployTruthOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeployTruthOptions {
    pub fleet: String,
    pub environment: String,
    pub profile: Option<CanisterBuildProfile>,
}

pub fn run<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    match parse_subcommand(deploy_command(), args)
        .map_err(|_| DeployCommandError::Usage(usage()))?
    {
        None => {
            println!("{}", usage());
            Ok(())
        }
        Some((command, args)) => match command.as_str() {
            "check" => check::run(args),
            "inspect" => inspect::run(args),
            "plan" => plan::run(args),
            "recovery" => recovery::run(args),
            _ => unreachable!("deploy dispatch command only defines known commands"),
        },
    }
}

pub fn load_deployment_check(
    options: DeployTruthOptions,
) -> Result<DeploymentCheckV1, DeployCommandError> {
    let icp_root = resolve_current_canic_icp_root()?;
    let fleet = read_fleet_catalog_entry_from_root(&icp_root, &options.environment, &options.fleet)
        .map_err(|error| DeployCommandError::Check(Box::new(error)))?
        .ok_or_else(|| {
            DeployCommandError::Usage(format!(
                "Fleet {} is not installed for environment profile {}",
                options.fleet, options.environment
            ))
        })?;
    let app = fleet.app.to_string();
    check_install_deployment_truth(
        &options.into_install_root_options_with_icp_root(Some(icp_root), app),
        current_evidence_timestamp()?,
    )
    .map_err(DeployCommandError::from)
}

pub fn print_json<T>(value: &T) -> Result<(), DeployCommandError>
where
    T: serde::Serialize,
{
    let json = serde_json::to_string_pretty(value).map_err(Box::<dyn std::error::Error>::from)?;
    println!("{json}");
    Ok(())
}

fn print_json_or_text<T>(
    format: JsonTextOutputFormat,
    value: &T,
    render_text: impl FnOnce(&T) -> String,
) -> Result<(), DeployCommandError>
where
    T: serde::Serialize,
{
    match format {
        JsonTextOutputFormat::Json => print_json(value),
        JsonTextOutputFormat::Text => {
            println!("{}", render_text(value));
            Ok(())
        }
    }
}

pub fn read_json_file<T>(path: &PathBuf) -> Result<T, DeployCommandError>
where
    T: DeserializeOwned,
{
    let bytes = fs::read(path).map_err(Box::<dyn std::error::Error>::from)?;
    serde_json::from_slice(&bytes)
        .map_err(Box::<dyn std::error::Error>::from)
        .map_err(DeployCommandError::from)
}

impl DeployTruthOptions {
    fn parse<I>(
        args: I,
        command: impl FnOnce() -> ClapCommand,
        usage: fn() -> String,
    ) -> Result<Self, DeployCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches =
            parse_matches(command(), args).map_err(|_| DeployCommandError::Usage(usage()))?;
        Ok(Self::from_matches(&matches))
    }

    pub(super) fn from_matches(matches: &clap::ArgMatches) -> Self {
        Self {
            fleet: required_string(matches, FLEET_ARG),
            environment: string_option_or_else(matches, "environment", local_environment),
            profile: typed_option(matches, PROFILE_ARG),
        }
    }

    fn into_install_root_options_with_icp_root(
        self,
        icp_root: Option<std::path::PathBuf>,
        app: String,
    ) -> InstallRootOptions {
        InstallRootOptions {
            root_canister: DEFAULT_ROOT_TARGET.to_string(),
            root_build_target: DEFAULT_ROOT_TARGET.to_string(),
            icp_executable: default_icp(),
            environment: self.environment,
            fleet_name: self.fleet,
            icp_root,
            build_profile: self.profile,
            release_build_id: None,
            config_path: Some(format!("apps/{app}/canic.toml")),
            fleet_install_input_path: None,
            expected_fresh_fleet_plan_digest: None,
            admitted_fresh_fleet_plan_digest: None,
            expected_app: Some(app),
            retained_root_repair_adoption: None,
            interactive_config_selection: false,
            deployment_plan_override: None,
        }
    }
}

#[cfg(test)]
mod tests;
