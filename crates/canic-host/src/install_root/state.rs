use crate::release_set::icp_root;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path, path::PathBuf};

pub(super) const INSTALL_STATE_SCHEMA_VERSION: u32 = 1;

///
/// InstallState
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct InstallState {
    pub schema_version: u32,
    pub fleet: String,
    pub installed_at_unix_secs: u64,
    pub network: String,
    pub root_target: String,
    pub root_canister_id: String,
    pub root_build_target: String,
    pub workspace_root: String,
    pub icp_root: String,
    pub config_path: String,
    pub release_set_manifest_path: String,
}

/// Read a named fleet install state for one project/network when present.
pub(super) fn read_fleet_install_state(
    icp_root: &Path,
    network: &str,
    fleet: &str,
) -> Result<Option<InstallState>, Box<dyn std::error::Error>> {
    validate_network_name(network)?;
    validate_fleet_name(fleet)?;
    let path = fleet_install_state_path(icp_root, network, fleet);
    if !path.is_file() {
        return Ok(None);
    }

    let bytes = fs::read(&path)?;
    let state: InstallState = serde_json::from_slice(&bytes)?;
    Ok(Some(state))
}

/// Read a named fleet state for the discovered current project.
pub fn read_named_fleet_install_state(
    network: &str,
    fleet: &str,
) -> Result<Option<InstallState>, Box<dyn std::error::Error>> {
    let icp_root = icp_root()?;
    read_fleet_install_state(&icp_root, network, fleet)
}

/// Read a named fleet state for an explicit ICP project root.
pub fn read_named_fleet_install_state_from_root(
    icp_root: &Path,
    network: &str,
    fleet: &str,
) -> Result<Option<InstallState>, Box<dyn std::error::Error>> {
    read_fleet_install_state(icp_root, network, fleet)
}

/// Return the project-local state path for one named fleet.
#[must_use]
pub(super) fn fleet_install_state_path(icp_root: &Path, network: &str, fleet: &str) -> PathBuf {
    fleets_dir(icp_root, network).join(format!("{fleet}.json"))
}

// Return the directory that owns named fleet state files.
fn fleets_dir(icp_root: &Path, network: &str) -> PathBuf {
    icp_root.join(".canic").join(network).join("fleets")
}

// Persist the completed install state under the project-local `.canic` directory.
pub(super) fn write_install_state(
    icp_root: &Path,
    network: &str,
    state: &InstallState,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    validate_network_name(network)?;
    validate_fleet_name(&state.fleet)?;
    let path = fleet_install_state_path(icp_root, network, &state.fleet);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    remove_conflicting_fleet_states(icp_root, network, state)?;
    fs::write(&path, serde_json::to_vec_pretty(state)?)?;
    Ok(path)
}

// A named ICP canister belongs to one installed fleet at a time. When a new
// install reuses that root, older fleet state would otherwise point at the new
// deployment and make `canic list <old-fleet>` show the wrong topology.
fn remove_conflicting_fleet_states(
    icp_root: &Path,
    network: &str,
    state: &InstallState,
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = fleets_dir(icp_root, network);
    if !dir.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() || path == fleet_install_state_path(icp_root, network, &state.fleet) {
            continue;
        }

        let Ok(bytes) = fs::read(&path) else {
            continue;
        };
        let Ok(existing) = serde_json::from_slice::<InstallState>(&bytes) else {
            continue;
        };
        if existing.network == state.network
            && (existing.root_target == state.root_target
                || existing.root_canister_id == state.root_canister_id)
        {
            fs::remove_file(path)?;
        }
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
fn validate_network_name(name: &str) -> Result<(), Box<dyn std::error::Error>> {
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
