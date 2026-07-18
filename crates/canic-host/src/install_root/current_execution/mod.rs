use super::deployment_truth_gate::{
    enforce_install_deployment_truth_gate, install_deployment_truth_gate_receipt,
    print_install_deployment_truth_gate,
};
use super::execution_preflight::write_current_install_execution_preflight_receipt;
use super::phase_receipts::receipt_with_execution_context;
use super::receipt_io::write_install_deployment_truth_receipt;
use super::{
    capabilities::CURRENT_INSTALL_REQUIRED_CAPABILITIES, clock::current_unix_timestamp_label,
    options::InstallRootOptions,
};
use crate::deployment_truth::{
    CurrentCliDeploymentExecutor, DeploymentCheckV1, DeploymentExecutionContextV1,
    DeploymentExecutor, DeploymentExecutorCapabilityV1, artifact_gate_phase_receipt,
    artifact_gate_role_phase_receipts, missing_executor_capabilities,
};
use crate::release_set::artifact_root_path;
use std::path::Path;

pub(super) fn current_install_execution_context(
    workspace_root: &Path,
    icp_root: &Path,
    artifact_network: &str,
) -> DeploymentExecutionContextV1 {
    CurrentCliDeploymentExecutor::new(
        Some(workspace_root.display().to_string()),
        Some(icp_root.display().to_string()),
        vec![
            artifact_root_path(icp_root, artifact_network)
                .display()
                .to_string(),
        ],
    )
    .execution_context()
}

pub(super) fn ensure_current_install_executor_capabilities(
    execution_context: &DeploymentExecutionContextV1,
) -> Result<(), Box<dyn std::error::Error>> {
    let missing = current_install_executor_missing_capabilities(execution_context);
    if missing.is_empty() {
        return Ok(());
    }

    Err(format!(
        "current install executor backend {:?} is missing required capabilities: {missing:?}",
        execution_context.backend
    )
    .into())
}

pub(super) fn current_install_executor_missing_capabilities(
    execution_context: &DeploymentExecutionContextV1,
) -> Vec<DeploymentExecutorCapabilityV1> {
    missing_executor_capabilities(
        &execution_context.backend_capabilities,
        CURRENT_INSTALL_REQUIRED_CAPABILITIES,
    )
}

pub(super) fn run_install_deployment_truth_safety_gate(
    options: &InstallRootOptions,
    workspace_root: &Path,
    icp_root: &Path,
    config_path: &Path,
    deployment_name: &str,
    execution_context: &DeploymentExecutionContextV1,
) -> Result<DeploymentCheckV1, Box<dyn std::error::Error>> {
    let truth_gate_started_at = current_unix_timestamp_label()?;
    let deployment_truth_check = super::truth_check::current_install_deployment_truth_check_at(
        options,
        workspace_root,
        icp_root,
        config_path,
        deployment_name,
        truth_gate_started_at.clone(),
    )?;
    let artifact_gate_receipt = artifact_gate_phase_receipt(
        &deployment_truth_check,
        truth_gate_started_at.clone(),
        Some(current_unix_timestamp_label()?),
    );
    let role_receipts = artifact_gate_role_phase_receipts(&deployment_truth_check);
    let deployment_receipt = receipt_with_execution_context(
        install_deployment_truth_gate_receipt(
            &deployment_truth_check,
            truth_gate_started_at,
            vec![artifact_gate_receipt],
            role_receipts,
        ),
        execution_context,
    );
    let receipt_write = write_install_deployment_truth_receipt(
        icp_root,
        &options.network,
        deployment_name,
        &deployment_receipt,
    );
    match &receipt_write {
        Ok(path) => println!("Deployment truth receipt JSON: {}", path.display()),
        Err(err) => eprintln!("Deployment truth receipt JSON write failed: {err}"),
    }
    print_install_deployment_truth_gate(&deployment_truth_check, &deployment_receipt);
    enforce_install_deployment_truth_gate(&deployment_truth_check)?;
    receipt_write?;
    write_current_install_execution_preflight_receipt(
        icp_root,
        &options.network,
        deployment_name,
        &deployment_truth_check,
        execution_context,
    )?;
    Ok(deployment_truth_check)
}
