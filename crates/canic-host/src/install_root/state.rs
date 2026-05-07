use crate::release_set::dfx_root;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path, path::PathBuf};

pub(super) const INSTALL_STATE_SCHEMA_VERSION: u32 = 1;
const INSTALL_STATE_FILE: &str = "install-state.json";
const CURRENT_FLEET_FILE: &str = "current-fleet";

///
/// InstallState
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct InstallState {
    pub schema_version: u32,
    #[serde(default = "default_fleet_name")]
    pub fleet: String,
    pub installed_at_unix_secs: u64,
    pub network: String,
    pub root_target: String,
    pub root_canister_id: String,
    pub root_build_target: String,
    pub workspace_root: String,
    pub dfx_root: String,
    pub config_path: String,
    pub release_set_manifest_path: String,
}

///
/// FleetSummary
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetSummary {
    pub name: String,
    pub current: bool,
    pub state: InstallState,
}

/// Read the persisted install state for one project/network when present.
pub(super) fn read_install_state(
    dfx_root: &Path,
    network: &str,
) -> Result<Option<InstallState>, Box<dyn std::error::Error>> {
    if let Some(fleet) = read_selected_fleet_name(dfx_root, network)? {
        return read_fleet_install_state(dfx_root, network, &fleet);
    }

    read_legacy_install_state(dfx_root, network)
}

/// Read a named fleet install state for one project/network when present.
pub(super) fn read_fleet_install_state(
    dfx_root: &Path,
    network: &str,
    fleet: &str,
) -> Result<Option<InstallState>, Box<dyn std::error::Error>> {
    validate_fleet_name(fleet)?;
    let path = fleet_install_state_path(dfx_root, network, fleet);
    if !path.is_file() {
        return Ok(None);
    }

    let bytes = fs::read(&path)?;
    let mut state: InstallState = serde_json::from_slice(&bytes)?;
    if state.fleet.is_empty() {
        state.fleet = fleet.to_string();
    }
    Ok(Some(state))
}

/// Read the install state for the discovered current project/network.
pub fn read_current_install_state(
    network: &str,
) -> Result<Option<InstallState>, Box<dyn std::error::Error>> {
    let dfx_root = dfx_root()?;
    read_install_state(&dfx_root, network)
}

/// Read either a named fleet state or the selected current fleet state.
pub fn read_current_or_fleet_install_state(
    network: &str,
    fleet: Option<&str>,
) -> Result<Option<InstallState>, Box<dyn std::error::Error>> {
    let dfx_root = dfx_root()?;
    match fleet {
        Some(fleet) => read_fleet_install_state(&dfx_root, network, fleet),
        None => read_install_state(&dfx_root, network),
    }
}

/// List installed fleets for the current project/network.
pub fn list_current_fleets(network: &str) -> Result<Vec<FleetSummary>, Box<dyn std::error::Error>> {
    let dfx_root = dfx_root()?;
    list_fleets(&dfx_root, network)
}

/// List installed fleets for one project/network.
pub(super) fn list_fleets(
    dfx_root: &Path,
    network: &str,
) -> Result<Vec<FleetSummary>, Box<dyn std::error::Error>> {
    let current = read_selected_fleet_name(dfx_root, network)?;
    let mut fleets = Vec::new();
    let dir = fleets_dir(dfx_root, network);
    if dir.is_dir() {
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            let Some(name) = path.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            if let Some(state) = read_fleet_install_state(dfx_root, network, name)? {
                fleets.push(FleetSummary {
                    name: name.to_string(),
                    current: current.as_deref() == Some(name),
                    state,
                });
            }
        }
    }

    if fleets.is_empty()
        && let Some(state) = read_legacy_install_state(dfx_root, network)?
    {
        fleets.push(FleetSummary {
            name: state.fleet.clone(),
            current: true,
            state,
        });
    }

    fleets.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(fleets)
}

/// Select one installed fleet as the current project/network default.
pub fn select_current_fleet(
    network: &str,
    fleet: &str,
) -> Result<InstallState, Box<dyn std::error::Error>> {
    let dfx_root = dfx_root()?;
    select_fleet(&dfx_root, network, fleet)
}

/// Select one installed fleet for one project/network.
fn select_fleet(
    dfx_root: &Path,
    network: &str,
    fleet: &str,
) -> Result<InstallState, Box<dyn std::error::Error>> {
    let Some(state) = read_fleet_install_state(dfx_root, network, fleet)?.or_else(|| {
        matching_legacy_fleet_state(dfx_root, network, fleet)
            .ok()
            .flatten()
    }) else {
        return Err(format!("unknown fleet {fleet} on network {network}").into());
    };
    if fleet_install_state_path(dfx_root, network, fleet).is_file() {
        write_current_fleet_name(dfx_root, network, fleet)?;
    } else {
        write_install_state(dfx_root, network, &state)?;
    }
    Ok(state)
}

/// Return the legacy project-local install state path for one network.
#[must_use]
fn install_state_path(dfx_root: &Path, network: &str) -> PathBuf {
    dfx_root
        .join(".canic")
        .join(network)
        .join(INSTALL_STATE_FILE)
}

/// Return the project-local state path for one named fleet.
#[must_use]
pub(super) fn fleet_install_state_path(dfx_root: &Path, network: &str, fleet: &str) -> PathBuf {
    fleets_dir(dfx_root, network).join(format!("{fleet}.json"))
}

/// Return the project-local current-fleet pointer path for one network.
#[must_use]
pub(super) fn current_fleet_path(dfx_root: &Path, network: &str) -> PathBuf {
    dfx_root
        .join(".canic")
        .join(network)
        .join(CURRENT_FLEET_FILE)
}

// Return the directory that owns named fleet state files.
fn fleets_dir(dfx_root: &Path, network: &str) -> PathBuf {
    dfx_root.join(".canic").join(network).join("fleets")
}

// Persist the completed install state under the project-local `.canic` directory.
pub(super) fn write_install_state(
    dfx_root: &Path,
    network: &str,
    state: &InstallState,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    validate_fleet_name(&state.fleet)?;
    let path = fleet_install_state_path(dfx_root, network, &state.fleet);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, serde_json::to_vec_pretty(state)?)?;
    write_current_fleet_name(dfx_root, network, &state.fleet)?;
    Ok(path)
}

// Read a legacy single-slot install state when no named fleet pointer exists.
fn read_legacy_install_state(
    dfx_root: &Path,
    network: &str,
) -> Result<Option<InstallState>, Box<dyn std::error::Error>> {
    let path = install_state_path(dfx_root, network);
    if !path.is_file() {
        return Ok(None);
    }

    let bytes = fs::read(&path)?;
    let state: InstallState = serde_json::from_slice(&bytes)?;
    if state.fleet.is_empty() {
        return Err(format!(
            "install state at {} is missing required fleet name; reinstall from a config with [fleet].name",
            path.display()
        )
        .into());
    }
    Ok(Some(state))
}

// Return the legacy single-slot state only when it matches the requested fleet.
fn matching_legacy_fleet_state(
    dfx_root: &Path,
    network: &str,
    fleet: &str,
) -> Result<Option<InstallState>, Box<dyn std::error::Error>> {
    Ok(read_legacy_install_state(dfx_root, network)?.filter(|state| state.fleet == fleet))
}

// Read the selected fleet name for one project/network.
fn read_selected_fleet_name(
    dfx_root: &Path,
    network: &str,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let path = current_fleet_path(dfx_root, network);
    if !path.is_file() {
        return Ok(None);
    }

    let name = fs::read_to_string(path)?.trim().to_string();
    validate_fleet_name(&name)?;
    Ok(Some(name))
}

// Write the selected fleet name for one project/network.
fn write_current_fleet_name(
    dfx_root: &Path,
    network: &str,
    fleet: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_fleet_name(fleet)?;
    let path = current_fleet_path(dfx_root, network);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, format!("{fleet}\n"))?;
    Ok(())
}

// Return the serde default for legacy install-state records.
const fn default_fleet_name() -> String {
    String::new()
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
