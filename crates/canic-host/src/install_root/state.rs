use crate::{
    durable_io::write_bytes,
    release_set::{WorkspaceDiscoveryError, icp_root},
};
use std::{fs, io, path::Path, path::PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

pub(super) const INSTALL_STATE_SCHEMA_VERSION: u32 = 2;

/// Typed failure while locating, validating, decoding, or persisting install state.
#[derive(Debug, ThisError)]
pub enum InstallStateError {
    #[error(
        "deployment state identity mismatch: state is for {state_deployment}, requested {requested_deployment}"
    )]
    DeploymentMismatch {
        state_deployment: String,
        requested_deployment: String,
    },

    #[error("invalid deployment/template name {name:?}; use letters, numbers, '-' or '_'")]
    InvalidStateName { name: String },

    #[error("invalid network name {name:?}; use letters, numbers, '-' or '_'")]
    InvalidNetworkName { name: String },

    #[error(
        "deployment state network mismatch: state is for {state_network}, requested {requested_network}"
    )]
    NetworkMismatch {
        state_network: String,
        requested_network: String,
    },

    #[error(
        "unsupported deployment state schema version {state_version}; supported version is {supported_version}"
    )]
    SchemaVersionMismatch {
        state_version: u32,
        supported_version: u32,
    },

    #[error("failed to resolve ICP root from {}: {source}", path.display())]
    ResolveIcpRoot {
        path: PathBuf,
        #[source]
        source: WorkspaceDiscoveryError,
    },

    #[error("failed to read deployment state {}: {source}", path.display())]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to decode deployment state {}: {source}", path.display())]
    Decode {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("failed to encode deployment state {}: {source}", path.display())]
    Encode {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("failed to write deployment state {}: {source}", path.display())]
    Write {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

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
) -> Result<Option<InstallState>, InstallStateError> {
    validate_network_name(network)?;
    validate_state_name(deployment)?;
    let path = deployment_install_state_path(icp_root, network, deployment);
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(source) if source.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(InstallStateError::Read { path, source });
        }
    };
    decode_install_state(&bytes, &path, network, deployment).map(Some)
}

/// Decode and validate install state against its canonical network/deployment path.
pub fn decode_install_state(
    bytes: &[u8],
    path: &Path,
    network: &str,
    deployment: &str,
) -> Result<InstallState, InstallStateError> {
    validate_network_name(network)?;
    validate_state_name(deployment)?;
    let state = serde_json::from_slice(bytes).map_err(|source| InstallStateError::Decode {
        path: path.to_path_buf(),
        source,
    })?;
    validate_loaded_install_state(&state, network, deployment)?;
    Ok(state)
}

/// Read deployment-target install state for the discovered current project.
pub fn read_named_deployment_install_state(
    network: &str,
    deployment: &str,
) -> Result<Option<InstallState>, InstallStateError> {
    let start = PathBuf::from(".");
    let icp_root = icp_root().map_err(|source| InstallStateError::ResolveIcpRoot {
        path: start,
        source,
    })?;
    read_deployment_install_state(&icp_root, network, deployment)
}

/// Read deployment-target install state for an explicit ICP project root.
pub fn read_named_deployment_install_state_from_root(
    icp_root: &Path,
    network: &str,
    deployment: &str,
) -> Result<Option<InstallState>, InstallStateError> {
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
) -> Result<PathBuf, InstallStateError> {
    validate_network_name(network)?;
    validate_state_name(&state.deployment_name)?;
    validate_schema_version(state.schema_version)?;
    if state.network != network {
        return Err(InstallStateError::NetworkMismatch {
            state_network: state.network.clone(),
            requested_network: network.to_string(),
        });
    }
    let path = deployment_install_state_path(icp_root, network, &state.deployment_name);
    let bytes = serde_json::to_vec_pretty(state).map_err(|source| InstallStateError::Encode {
        path: path.clone(),
        source,
    })?;
    write_bytes(&path, &bytes).map_err(|source| InstallStateError::Write {
        path: path.clone(),
        source,
    })?;
    Ok(path)
}

// Reject state that does not belong to the requested canonical path.
fn validate_loaded_install_state(
    state: &InstallState,
    requested_network: &str,
    requested_deployment: &str,
) -> Result<(), InstallStateError> {
    validate_schema_version(state.schema_version)?;
    if state.deployment_name != requested_deployment {
        return Err(InstallStateError::DeploymentMismatch {
            state_deployment: state.deployment_name.clone(),
            requested_deployment: requested_deployment.to_string(),
        });
    }
    if state.network != requested_network {
        return Err(InstallStateError::NetworkMismatch {
            state_network: state.network.clone(),
            requested_network: requested_network.to_string(),
        });
    }
    Ok(())
}

// Keep readers and writers on the one supported install-state schema.
const fn validate_schema_version(schema_version: u32) -> Result<(), InstallStateError> {
    if schema_version == INSTALL_STATE_SCHEMA_VERSION {
        Ok(())
    } else {
        Err(InstallStateError::SchemaVersionMismatch {
            state_version: schema_version,
            supported_version: INSTALL_STATE_SCHEMA_VERSION,
        })
    }
}

// Keep deployment and template names filesystem-safe and easy to type.
pub(super) fn validate_state_name(name: &str) -> Result<(), InstallStateError> {
    let valid = !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'));
    if valid {
        Ok(())
    } else {
        Err(InstallStateError::InvalidStateName {
            name: name.to_string(),
        })
    }
}

// Keep network names safe for `.canic/<network>` state paths.
pub fn validate_network_name(name: &str) -> Result<(), InstallStateError> {
    let valid = !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'));
    if valid {
        Ok(())
    } else {
        Err(InstallStateError::InvalidNetworkName {
            name: name.to_string(),
        })
    }
}
