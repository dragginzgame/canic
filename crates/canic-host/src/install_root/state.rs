use crate::{durable_io::write_bytes, release_set::icp_root};
use std::{fs, path::Path, path::PathBuf};

use serde::{Deserialize, Serialize};

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
#[serde(deny_unknown_fields)]
pub struct InstallState {
    pub schema_version: u32,
    pub deployment_name: String,
    pub fleet_template: String,
    pub created_at_unix_secs: u64,
    pub updated_at_unix_secs: u64,
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
pub(super) fn read_deployment_install_state(
    icp_root: &Path,
    network: &str,
    deployment: &str,
) -> Result<Option<InstallState>, Box<dyn std::error::Error>> {
    validate_network_name(network)?;
    validate_state_name(deployment)?;
    let path = deployment_install_state_path(icp_root, network, deployment);
    if !path.is_file() {
        return Ok(None);
    }

    let bytes = fs::read(&path)?;
    let state: InstallState = serde_json::from_slice(&bytes)?;
    Ok(Some(state))
}

/// Read deployment-target install state for the discovered current project.
pub fn read_named_deployment_install_state(
    network: &str,
    deployment: &str,
) -> Result<Option<InstallState>, Box<dyn std::error::Error>> {
    let icp_root = icp_root()?;
    read_deployment_install_state(&icp_root, network, deployment)
}

/// Read deployment-target install state for an explicit ICP project root.
pub fn read_named_deployment_install_state_from_root(
    icp_root: &Path,
    network: &str,
    deployment: &str,
) -> Result<Option<InstallState>, Box<dyn std::error::Error>> {
    read_deployment_install_state(icp_root, network, deployment)
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
    validate_state_name(&state.deployment_name)?;
    if state.network != network {
        return Err(format!(
            "deployment state network mismatch: state is for {}, requested {network}",
            state.network
        )
        .into());
    }
    let path = deployment_install_state_path(icp_root, network, &state.deployment_name);
    write_bytes(&path, &serde_json::to_vec_pretty(state)?)?;
    Ok(path)
}

// Keep deployment and template names filesystem-safe and easy to type.
pub(super) fn validate_state_name(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let valid = !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'));
    if valid {
        Ok(())
    } else {
        Err(
            format!("invalid deployment/template name {name:?}; use letters, numbers, '-' or '_'")
                .into(),
        )
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
