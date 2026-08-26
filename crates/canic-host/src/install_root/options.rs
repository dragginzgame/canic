use crate::{canister_build::CanisterBuildProfile, deployment_truth::DeploymentPlanV1};
use candid::Principal;
use canic_core::ids::ReleaseBuildId;
use std::{path::PathBuf, str::FromStr};

/// One explicit adoption request for an already-applied state-preserving Root repair.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetainedRootRepairAdoption {
    pub fleet_subnet_root: Principal,
    pub pool_canister: Principal,
    pub live_predecessor_wasm: PathBuf,
    pub successor_wasm: PathBuf,
}

impl FromStr for RetainedRootRepairAdoption {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (authority, artifacts) = value.split_once('=').ok_or_else(|| {
            "retained Root repair must be formatted as <ROOT_PRINCIPAL>,<POOL_CANISTER_PRINCIPAL>=<LIVE_RAW_WASM_PATH>,<SUCCESSOR_RAW_WASM_PATH>"
                .to_string()
        })?;
        let (root, pool_canister) = authority.split_once(',').ok_or_else(|| {
            "retained Root repair no longer accepts <ROOT_PRINCIPAL>=<RAW_WASM_PATH>; expected <ROOT_PRINCIPAL>,<POOL_CANISTER_PRINCIPAL>=<LIVE_RAW_WASM_PATH>,<SUCCESSOR_RAW_WASM_PATH>"
                .to_string()
        })?;
        let fleet_subnet_root = Principal::from_text(root)
            .map_err(|_| "retained Root repair has an invalid Root Principal".to_string())?;
        if fleet_subnet_root == Principal::anonymous() {
            return Err("retained Root repair cannot name the anonymous Principal".to_string());
        }
        let pool_canister = Principal::from_text(pool_canister).map_err(|_| {
            "retained Root repair has an invalid pool Canister Principal".to_string()
        })?;
        if pool_canister == Principal::anonymous() || pool_canister == fleet_subnet_root {
            return Err(
                "retained Root repair must name a distinct non-anonymous pool Canister".to_string(),
            );
        }
        let (live_predecessor_wasm, successor_wasm) =
            artifacts.split_once(',').ok_or_else(|| {
                "retained Root repair must name both the exact live predecessor Wasm and successor Wasm"
                    .to_string()
            })?;
        if live_predecessor_wasm.is_empty() || successor_wasm.is_empty() {
            return Err("retained Root repair raw Wasm paths must not be empty".to_string());
        }
        Ok(Self {
            fleet_subnet_root,
            pool_canister,
            live_predecessor_wasm: PathBuf::from(live_predecessor_wasm),
            successor_wasm: PathBuf::from(successor_wasm),
        })
    }
}

///
/// InstallRootOptions
///

#[derive(Clone, Debug)]
pub struct InstallRootOptions {
    pub root_canister: String,
    pub root_build_target: String,
    pub icp_executable: String,
    pub environment: String,
    pub fleet_name: String,
    pub icp_root: Option<PathBuf>,
    pub build_profile: Option<CanisterBuildProfile>,
    pub release_build_id: Option<ReleaseBuildId>,
    pub config_path: Option<String>,
    pub fleet_install_input_path: Option<PathBuf>,
    pub expected_fresh_fleet_plan_digest: Option<String>,
    pub admitted_fresh_fleet_plan_digest: Option<String>,
    pub expected_app: Option<String>,
    pub retained_root_repair_adoption: Option<RetainedRootRepairAdoption>,
    pub retained_root_repair_funding_authorization: Option<String>,
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
