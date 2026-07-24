use crate::{canister_build::CanisterBuildProfile, deployment_truth::DeploymentPlanV1};
use std::path::PathBuf;

///
/// InstallRootOptions
///

#[derive(Clone, Debug)]
pub struct InstallRootOptions {
    pub root_canister: String,
    pub root_build_target: String,
    pub environment: String,
    pub fleet_name: String,
    pub icp_root: Option<PathBuf>,
    pub build_profile: Option<CanisterBuildProfile>,
    pub config_path: Option<String>,
    pub expected_app: Option<String>,
    pub interactive_config_selection: bool,
    pub deployment_plan_override: Option<DeploymentPlanV1>,
}

impl InstallRootOptions {
    /// Return the exact ICP artifact environment owned by this install mode.
    pub(super) fn artifact_environment(&self) -> &str {
        if self.deployment_plan_override.is_some() {
            &self.environment
        } else {
            "local"
        }
    }
}
