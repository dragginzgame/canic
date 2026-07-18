use super::operations::InstallPhaseLabel;
use super::phase_receipts::{
    CompletedInstallPhase, InstallReceiptScope, write_completed_install_phase_receipt,
};
use super::state::{
    INSTALL_STATE_SCHEMA_VERSION, InstallState, RootVerificationStatus, write_install_state,
};
use super::{
    clock::{current_unix_secs, current_unix_timestamp_label},
    options::InstallRootOptions,
};
use std::path::{Path, PathBuf};

pub(super) fn write_install_state_with_deployment_truth_receipt(
    receipt_scope: InstallReceiptScope<'_>,
    environment: &str,
    state: &InstallState,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let started_at = current_unix_timestamp_label()?;
    let state_path = write_install_state(receipt_scope.icp_root, environment, state)?;
    let completed = CompletedInstallPhase {
        phase: InstallPhaseLabel::WRITE_INSTALL_STATE,
        attempted_action: "write local install state",
        started_at,
        finished_at: Some(current_unix_timestamp_label()?),
        evidence: vec![
            format!("install_state:{}", state_path.display()),
            format!("deployment:{}", state.deployment_name),
            format!("fleet_template:{}", state.fleet_template),
            format!("root_canister:{}", state.root_canister_id),
        ],
        role_names: Vec::new(),
    };
    write_completed_install_phase_receipt(receipt_scope, completed)?;
    Ok(state_path)
}

// Build the persisted project-local install state from a completed install.
pub(super) fn build_install_state(
    options: &InstallRootOptions,
    workspace_root: &Path,
    icp_root: &Path,
    config_path: &Path,
    release_set_manifest_path: &Path,
    identity: (&str, &str),
    root_canister_id: &str,
) -> Result<InstallState, Box<dyn std::error::Error>> {
    let (deployment_name, fleet_name) = identity;
    let timestamp = current_unix_secs()?;
    Ok(InstallState {
        schema_version: INSTALL_STATE_SCHEMA_VERSION,
        deployment_name: deployment_name.to_string(),
        fleet_template: fleet_name.to_string(),
        created_at_unix_secs: timestamp,
        updated_at_unix_secs: timestamp,
        environment: options.environment.clone(),
        root_target: options.root_canister.clone(),
        root_canister_id: root_canister_id.to_string(),
        root_verification: RootVerificationStatus::Verified,
        root_build_target: options.root_build_target.clone(),
        workspace_root: workspace_root.display().to_string(),
        icp_root: icp_root.display().to_string(),
        config_path: config_path.display().to_string(),
        release_set_manifest_path: release_set_manifest_path.display().to_string(),
    })
}
