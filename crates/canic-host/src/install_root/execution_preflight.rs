use super::deployment_truth_gate::deployment_truth_finding_label;
use super::phase_receipts::receipt_with_execution_context;
use super::receipt_io::write_install_deployment_truth_receipt;
use crate::deployment_truth::{
    CurrentCliDeploymentExecutor, DeploymentCheckV1, DeploymentCommandResultV1,
    DeploymentExecutionContextV1, DeploymentExecutionPreflightV1, DeploymentExecutionStatusV1,
    ObservationStatusV1, deployment_execution_preflight_from_check,
    deployment_receipt_from_check_with_status, phase_receipt,
    validate_deployment_execution_preflight_for_check,
};
use std::path::{Path, PathBuf};

pub(super) fn write_current_install_execution_preflight_receipt(
    icp_root: &Path,
    network: &str,
    deployment_name: &str,
    check: &DeploymentCheckV1,
    execution_context: &DeploymentExecutionContextV1,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let started_at = super::current_unix_timestamp_label()?;
    let executor = CurrentCliDeploymentExecutor::new(
        execution_context.workspace_root.clone(),
        execution_context.icp_root.clone(),
        execution_context.artifact_roots.clone(),
    );
    let preflight = deployment_execution_preflight_from_check(
        check,
        &executor,
        super::CURRENT_INSTALL_REQUIRED_CAPABILITIES,
    );
    validate_deployment_execution_preflight_for_check(check, &preflight)?;
    let blockers = preflight.blockers.clone();
    let (operation_status, command_result) = if blockers.is_empty() {
        (
            DeploymentExecutionStatusV1::Complete,
            DeploymentCommandResultV1::Succeeded,
        )
    } else {
        (
            DeploymentExecutionStatusV1::FailedBeforeMutation,
            DeploymentCommandResultV1::Failed {
                code: "execution_preflight_blocked".to_string(),
                message: "deployment execution preflight blocked current install".to_string(),
            },
        )
    };
    let finished_at = super::current_unix_timestamp_label()?;
    let receipt = receipt_with_execution_context(
        deployment_receipt_from_check_with_status(
            check,
            format!("{}:execution_preflight", check.check_id),
            operation_status,
            started_at.clone(),
            Some(finished_at.clone()),
            vec![phase_receipt(
                "execution_preflight",
                started_at,
                Some(finished_at),
                "validate deployment plan, authority, and executor capability readiness",
                ObservationStatusV1::Observed,
                current_install_execution_preflight_evidence(&preflight),
            )],
            Vec::new(),
            command_result,
        ),
        execution_context,
    );
    let path =
        write_install_deployment_truth_receipt(icp_root, network, deployment_name, &receipt)?;
    println!("Deployment truth receipt JSON: {}", path.display());
    if !blockers.is_empty() {
        let details = blockers
            .iter()
            .map(deployment_truth_finding_label)
            .collect::<Vec<_>>()
            .join("; ");
        return Err(format!("deployment execution preflight blocked install: {details}").into());
    }
    Ok(path)
}

fn current_install_execution_preflight_evidence(
    preflight: &DeploymentExecutionPreflightV1,
) -> Vec<String> {
    let mut evidence = vec![
        format!("execution_preflight_status:{:?}", preflight.status),
        format!("authority_plan:{}", preflight.authority_plan_id),
        format!("planned_phases:{}", preflight.planned_phases.len()),
        format!(
            "required_capabilities:{}",
            preflight.required_capabilities.len()
        ),
        format!(
            "missing_capabilities:{}",
            preflight.missing_capabilities.len()
        ),
        format!("blockers:{}", preflight.blockers.len()),
    ];
    evidence.extend(
        preflight
            .missing_capabilities
            .iter()
            .map(|capability| format!("missing_capability:{capability:?}")),
    );
    evidence.extend(
        preflight
            .blockers
            .iter()
            .map(|finding| format!("blocker:{}:{}", finding.code, finding.message)),
    );
    evidence
}
