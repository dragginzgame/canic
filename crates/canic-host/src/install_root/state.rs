use crate::release_set::icp_root;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path, path::PathBuf};

pub(super) const INSTALL_STATE_SCHEMA_VERSION: u32 = 2;

///
/// RootVerificationStatus
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RootVerificationStatus {
    Verified,
    NotVerified,
}

///
/// InstallState
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct InstallState {
    pub schema_version: u32,
    pub deployment_name: String,
    pub fleet_template: String,
    pub fleet: String,
    pub installed_at_unix_secs: u64,
    pub network: String,
    pub root_target: String,
    pub root_canister_id: String,
    pub root_verification: RootVerificationStatus,
    pub root_build_target: String,
    pub workspace_root: String,
    pub icp_root: String,
    pub config_path: String,
    pub release_set_manifest_path: String,
}

/// Read deployment-target install state for one project/network when present.
pub(super) fn read_fleet_install_state(
    icp_root: &Path,
    network: &str,
    deployment: &str,
) -> Result<Option<InstallState>, Box<dyn std::error::Error>> {
    validate_network_name(network)?;
    validate_fleet_name(deployment)?;
    let path = deployment_install_state_path(icp_root, network, deployment);
    if !path.is_file() {
        reject_legacy_fleet_state(icp_root, network, deployment)?;
        return Ok(None);
    }

    let bytes = fs::read(&path)?;
    let state: InstallState = serde_json::from_slice(&bytes)?;
    Ok(Some(state))
}

/// Read deployment-target install state for the discovered current project.
pub fn read_named_fleet_install_state(
    network: &str,
    deployment: &str,
) -> Result<Option<InstallState>, Box<dyn std::error::Error>> {
    let icp_root = icp_root()?;
    read_fleet_install_state(&icp_root, network, deployment)
}

/// Read deployment-target install state for an explicit ICP project root.
pub fn read_named_fleet_install_state_from_root(
    icp_root: &Path,
    network: &str,
    deployment: &str,
) -> Result<Option<InstallState>, Box<dyn std::error::Error>> {
    read_fleet_install_state(icp_root, network, deployment)
}

/// Return the legacy project-local fleet state path.
#[must_use]
pub(super) fn fleet_install_state_path(icp_root: &Path, network: &str, fleet: &str) -> PathBuf {
    fleets_dir(icp_root, network).join(format!("{fleet}.json"))
}

/// Return the project-local state path for one deployment target.
#[must_use]
pub(super) fn deployment_install_state_path(
    icp_root: &Path,
    network: &str,
    deployment: &str,
) -> PathBuf {
    deployments_dir(icp_root, network).join(format!("{deployment}.json"))
}

// Return the directory that owns named fleet state files.
fn fleets_dir(icp_root: &Path, network: &str) -> PathBuf {
    icp_root.join(".canic").join(network).join("fleets")
}

// Return the directory that owns deployment-target state files.
fn deployments_dir(icp_root: &Path, network: &str) -> PathBuf {
    icp_root.join(".canic").join(network).join("deployments")
}

// Persist the completed install state under the project-local `.canic` directory.
pub(super) fn write_install_state(
    icp_root: &Path,
    network: &str,
    state: &InstallState,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    validate_network_name(network)?;
    validate_fleet_name(&state.deployment_name)?;
    if state.network != network {
        return Err(format!(
            "deployment state network mismatch: state is for {}, requested {network}",
            state.network
        )
        .into());
    }
    let path = deployment_install_state_path(icp_root, network, &state.deployment_name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, serde_json::to_vec_pretty(state)?)?;
    Ok(path)
}

fn reject_legacy_fleet_state(
    icp_root: &Path,
    network: &str,
    deployment: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = fleet_install_state_path(icp_root, network, deployment);
    if path.exists() {
        return Err(format!(
            "legacy fleet install state found: {}\n\nCanic 0.46 stores live deployment state by deployment target, not fleet template.\nCreate explicit deployment state with:\n  canic deploy register {deployment} --fleet-template {deployment} --root <principal>\n\nOr reinstall the deployment with a 0.46 install path that writes deployment-target state:\n  canic install {deployment}\n\nIf the old state is obsolete, remove:\n  {}",
            path.display(),
            path.display()
        )
        .into());
    }
    Ok(())
}

// Keep fleet names filesystem-safe and easy to type in commands.
pub(super) fn validate_fleet_name(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let valid = !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'));
    if valid {
        Ok(())
    } else {
        Err(format!("invalid fleet name {name:?}; use letters, numbers, '-' or '_'").into())
    }
}

// Keep network names safe for `.canic/<network>` state paths.
pub(super) fn validate_network_name(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let valid = !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'));
    if valid {
        Ok(())
    } else {
        Err(format!("invalid network name {name:?}; use letters, numbers, '-' or '_'").into())
    }
}
