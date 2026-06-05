use super::{DEFAULT_READY_TIMEOUT_SECONDS, DEFAULT_ROOT_TARGET, DeployCommandError, value_arg};
use crate::{
    cli::{
        clap::{
            parse_matches, render_usage, required_path, required_string, string_option_or_else,
            typed_option,
        },
        defaults::local_network,
        globals::internal_network_arg,
        help::print_help_or_version,
    },
    version_text,
};
use canic_host::{
    canister_build::CanisterBuildProfile,
    deployment_truth::{
        ArtifactPromotionPlanV1, DeploymentPlanV1, PromotionReadinessStatusV1,
        validate_artifact_promotion_plan,
    },
    icp_config::resolve_current_canic_icp_root,
    install_root::{InstallRootOptions, install_root},
};
use clap::Command as ClapCommand;
use std::{ffi::OsString, fs, path::PathBuf};

const DEPLOY_INSTALL_HELP_AFTER: &str = "\
Examples:
  canic deploy install demo-local --plan promoted-plan.json
  canic --network local deploy install demo-local --plan promoted-plan.json --profile fast

Installs through the current install runner using a supplied DeploymentPlanV1
or ArtifactPromotionPlanV1. The deployment-truth/preflight gate runs before
mutation, and activation phases still execute through the current-install
operation runner.";

const DEPLOYMENT_ARG: &str = "deployment";
const PLAN_ARG: &str = "plan";
const PROFILE_ARG: &str = "profile";

///
/// DeployInstallPlanOptions
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct DeployInstallPlanOptions {
    pub(super) deployment: String,
    pub(super) plan: PathBuf,
    pub(super) network: String,
    pub(super) profile: Option<CanisterBuildProfile>,
}

///
/// DeployInstallPlanInput
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct DeployInstallPlanInput {
    pub(super) deployment_plan: DeploymentPlanV1,
    pub(super) artifact_promotion_plan: Option<ArtifactPromotionPlanV1>,
}

pub(super) fn run<I>(args: I) -> Result<(), DeployCommandError>
where
    I: IntoIterator<Item = OsString>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if print_help_or_version(&args, usage, version_text()) {
        return Ok(());
    }

    let options = DeployInstallPlanOptions::parse(args)?;
    let plan = read_plan(&options.plan)?;
    let icp_root = resolve_current_canic_icp_root().ok();
    install_root(options.into_install_root_options(plan, icp_root))
        .map_err(DeployCommandError::from)
}

pub(super) fn read_plan(path: &PathBuf) -> Result<DeployInstallPlanInput, DeployCommandError> {
    let bytes = fs::read(path).map_err(Box::<dyn std::error::Error>::from)?;
    if let Ok(plan) = serde_json::from_slice::<ArtifactPromotionPlanV1>(&bytes) {
        validate_artifact_promotion_plan(&plan).map_err(Box::<dyn std::error::Error>::from)?;
        if plan.status != PromotionReadinessStatusV1::Ready {
            return Err(DeployCommandError::Blocked(format!(
                "artifact promotion plan {} is not ready",
                plan.plan_id
            )));
        }
        return Ok(DeployInstallPlanInput {
            deployment_plan: plan.transform.promoted_plan.clone(),
            artifact_promotion_plan: Some(plan),
        });
    }

    serde_json::from_slice::<DeploymentPlanV1>(&bytes)
        .map(|deployment_plan| DeployInstallPlanInput {
            deployment_plan,
            artifact_promotion_plan: None,
        })
        .map_err(|err| {
            DeployCommandError::Check(
                format!(
                    "failed to decode {} as ArtifactPromotionPlanV1 or DeploymentPlanV1: {err}",
                    path.display()
                )
                .into(),
            )
        })
}

impl DeployInstallPlanOptions {
    pub(super) fn parse<I>(args: I) -> Result<Self, DeployCommandError>
    where
        I: IntoIterator<Item = OsString>,
    {
        let matches =
            parse_matches(command(), args).map_err(|_| DeployCommandError::Usage(usage()))?;
        Ok(Self {
            deployment: required_string(&matches, DEPLOYMENT_ARG),
            plan: required_path(&matches, PLAN_ARG),
            network: string_option_or_else(&matches, "network", local_network),
            profile: typed_option(&matches, PROFILE_ARG),
        })
    }

    pub(super) fn into_install_root_options(
        self,
        plan: DeployInstallPlanInput,
        icp_root: Option<PathBuf>,
    ) -> InstallRootOptions {
        let fleet_template = plan.deployment_plan.fleet_template.clone();
        InstallRootOptions {
            root_canister: root_canister_for_plan(&plan.deployment_plan),
            root_build_target: DEFAULT_ROOT_TARGET.to_string(),
            network: self.network,
            deployment_name: Some(self.deployment),
            icp_root,
            build_profile: self.profile,
            ready_timeout_seconds: DEFAULT_READY_TIMEOUT_SECONDS,
            config_path: Some(default_fleet_config_path(&fleet_template)),
            expected_fleet: Some(fleet_template),
            interactive_config_selection: false,
            deployment_plan_override: Some(plan.deployment_plan),
            artifact_promotion_plan_override: plan.artifact_promotion_plan,
        }
    }
}

fn root_canister_for_plan(plan: &DeploymentPlanV1) -> String {
    plan.trust_domain
        .root_trust_anchor
        .clone()
        .or_else(|| plan.deployment_identity.root_principal.clone())
        .or_else(|| {
            plan.expected_canisters
                .iter()
                .find(|canister| canister.role == DEFAULT_ROOT_TARGET)
                .and_then(|canister| canister.canister_id.clone())
        })
        .unwrap_or_else(|| DEFAULT_ROOT_TARGET.to_string())
}

fn default_fleet_config_path(fleet: &str) -> String {
    format!("fleets/{fleet}/canic.toml")
}

pub(super) fn command() -> ClapCommand {
    ClapCommand::new("install")
        .bin_name("canic deploy install")
        .about("Install through the current runner using a supplied deployment plan")
        .disable_help_flag(true)
        .override_usage("canic deploy install <deployment> --plan <file>")
        .arg(deployment_arg())
        .arg(plan_arg())
        .arg(profile_arg())
        .arg(internal_network_arg())
        .after_help(DEPLOY_INSTALL_HELP_AFTER)
}

fn deployment_arg() -> clap::Arg {
    value_arg(DEPLOYMENT_ARG)
        .required(true)
        .help("Deployment target name that must match the supplied plan")
}

fn plan_arg() -> clap::Arg {
    value_arg(PLAN_ARG)
        .long(PLAN_ARG)
        .value_name("file")
        .required(true)
        .help("DeploymentPlanV1 or ArtifactPromotionPlanV1 JSON file to install")
}

fn profile_arg() -> clap::Arg {
    value_arg(PROFILE_ARG)
        .long(PROFILE_ARG)
        .value_name("debug|fast|release")
        .num_args(1)
        .value_parser(clap::value_parser!(CanisterBuildProfile))
        .help("Canister wasm build profile; defaults to CANIC_WASM_PROFILE or release")
}

pub(super) fn usage() -> String {
    render_usage(command)
}
