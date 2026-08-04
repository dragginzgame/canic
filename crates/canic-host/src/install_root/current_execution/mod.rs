use super::deployment_truth_gate::{
    enforce_install_deployment_truth_gate, install_deployment_truth_gate_receipt,
    print_install_deployment_truth_gate,
};
use super::execution_preflight::current_install_execution_preflight_receipt;
use super::phase_receipts::receipt_with_execution_context;
use super::{
    capabilities::CURRENT_INSTALL_REQUIRED_CAPABILITIES, clock::current_unix_timestamp_label,
    options::InstallRootOptions,
};
use crate::deployment_truth::{
    CurrentCliDeploymentExecutor, DeploymentCheckV1, DeploymentExecutionContextV1,
    DeploymentExecutor, DeploymentExecutorCapabilityV1, DeploymentReceiptV1,
    artifact_gate_phase_receipt, artifact_gate_role_phase_receipts, missing_executor_capabilities,
};
use crate::release_set::artifact_root_path;
use std::path::{Path, PathBuf};

pub(super) struct PreparedInstallSafetyGate {
    pub(super) check: DeploymentCheckV1,
    pub(super) receipts: Vec<DeploymentReceiptV1>,
}

pub(super) fn current_install_execution_context(
    workspace_root: &Path,
    icp_root: &Path,
    artifact_environment: &str,
) -> DeploymentExecutionContextV1 {
    let artifact_root = artifact_root_path(icp_root, artifact_environment);
    current_install_execution_context_at_root(workspace_root, icp_root, &artifact_root)
}

pub(super) fn current_install_execution_context_at_root(
    workspace_root: &Path,
    icp_root: &Path,
    artifact_root: &Path,
) -> DeploymentExecutionContextV1 {
    CurrentCliDeploymentExecutor::new(
        Some(workspace_root.display().to_string()),
        Some(icp_root.display().to_string()),
        vec![artifact_root.display().to_string()],
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
    fleet_name: &str,
    execution_context: &DeploymentExecutionContextV1,
    prepared_plan: Option<&crate::deployment_truth::DeploymentPlanV1>,
) -> Result<PreparedInstallSafetyGate, Box<dyn std::error::Error>> {
    let truth_gate_started_at = current_unix_timestamp_label()?;
    let artifact_root = exact_execution_artifact_root(execution_context)?;
    let deployment_truth_check =
        super::truth_check::current_install_deployment_truth_check_at_with_plan(
            options,
            super::truth_check::CurrentInstallTruthScope::new(
                workspace_root,
                icp_root,
                config_path,
                fleet_name,
                truth_gate_started_at.clone(),
                &artifact_root,
            ),
            prepared_plan,
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
    print_install_deployment_truth_gate(&deployment_truth_check, &deployment_receipt);
    enforce_install_deployment_truth_gate(&deployment_truth_check)?;
    let preflight_receipt =
        current_install_execution_preflight_receipt(&deployment_truth_check, execution_context)?;
    Ok(PreparedInstallSafetyGate {
        check: deployment_truth_check,
        receipts: vec![deployment_receipt, preflight_receipt],
    })
}

fn exact_execution_artifact_root(
    execution_context: &DeploymentExecutionContextV1,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let [artifact_root] = execution_context.artifact_roots.as_slice() else {
        return Err("current install requires exactly one execution artifact root".into());
    };
    if artifact_root.trim().is_empty() {
        return Err("current install execution artifact root is empty".into());
    }
    Ok(PathBuf::from(artifact_root))
}
