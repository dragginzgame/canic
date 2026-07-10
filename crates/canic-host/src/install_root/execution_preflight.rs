use super::deployment_truth_gate::deployment_truth_finding_label;
use super::operations::InstallPhaseLabel;
use super::phase_receipts::receipt_with_execution_context;
use super::receipt_io::write_install_deployment_truth_receipt;
use super::{
    InstallRootBlockKind, InstallRootBlockedError,
    capabilities::CURRENT_INSTALL_REQUIRED_CAPABILITIES, clock::current_unix_timestamp_label,
};
use crate::deployment_truth::{
    CurrentCliDeploymentExecutor, DeploymentCheckV1, DeploymentCommandResultV1,
    DeploymentExecutionContextV1, DeploymentExecutionPreflightV1, DeploymentExecutionStatusV1,
    ObservationStatusV1, deployment_execution_preflight_from_check,
    deployment_receipt_from_check_with_status, phase_receipt,
    validate_deployment_execution_preflight_for_check,
};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ExecutionPreflightReceiptLabel(&'static str);

impl ExecutionPreflightReceiptLabel {
    const AUTHORITY_PLAN: Self = Self("authority_plan");
    const BLOCKED_CODE: Self = Self("execution_preflight_blocked");
    const BLOCKER: Self = Self("blocker");
    const BLOCKERS: Self = Self("blockers");
    const MISSING_CAPABILITIES: Self = Self("missing_capabilities");
    const MISSING_CAPABILITY: Self = Self("missing_capability");
    const PLANNED_PHASES: Self = Self("planned_phases");
    const REQUIRED_CAPABILITIES: Self = Self("required_capabilities");
    const STATUS: Self = Self("execution_preflight_status");

    #[must_use]
    const fn as_str(self) -> &'static str {
        self.0
    }
}

pub(super) fn write_current_install_execution_preflight_receipt(
    icp_root: &Path,
    network: &str,
    deployment_name: &str,
    check: &DeploymentCheckV1,
    execution_context: &DeploymentExecutionContextV1,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let started_at = current_unix_timestamp_label()?;
    let executor = CurrentCliDeploymentExecutor::new(
        execution_context.workspace_root.clone(),
        execution_context.icp_root.clone(),
        execution_context.artifact_roots.clone(),
    );
    let preflight = deployment_execution_preflight_from_check(
        check,
        &executor,
        CURRENT_INSTALL_REQUIRED_CAPABILITIES,
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
                code: ExecutionPreflightReceiptLabel::BLOCKED_CODE
                    .as_str()
                    .to_string(),
                message: "deployment execution preflight blocked current install".to_string(),
            },
        )
    };
    let finished_at = current_unix_timestamp_label()?;
    let phase_label = InstallPhaseLabel::EXECUTION_PREFLIGHT;
    let receipt = receipt_with_execution_context(
        deployment_receipt_from_check_with_status(
            check,
            format!("{}:{}", check.check_id, phase_label.as_str()),
            operation_status,
            started_at.clone(),
            Some(finished_at.clone()),
            vec![phase_receipt(
                phase_label.as_str(),
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
        return Err(Box::new(InstallRootBlockedError::new(
            InstallRootBlockKind::DeploymentExecutionPreflight,
            format!("deployment execution preflight blocked install: {details}"),
        )));
    }
    Ok(path)
}

fn current_install_execution_preflight_evidence(
    preflight: &DeploymentExecutionPreflightV1,
) -> Vec<String> {
    let mut evidence = vec![
        format!(
            "{}:{:?}",
            ExecutionPreflightReceiptLabel::STATUS.as_str(),
            preflight.status
        ),
        format!(
            "{}:{}",
            ExecutionPreflightReceiptLabel::AUTHORITY_PLAN.as_str(),
            preflight.authority_plan_id
        ),
        format!(
            "{}:{}",
            ExecutionPreflightReceiptLabel::PLANNED_PHASES.as_str(),
            preflight.planned_phases.len()
        ),
        format!(
            "{}:{}",
            ExecutionPreflightReceiptLabel::REQUIRED_CAPABILITIES.as_str(),
            preflight.required_capabilities.len()
        ),
        format!(
            "{}:{}",
            ExecutionPreflightReceiptLabel::MISSING_CAPABILITIES.as_str(),
            preflight.missing_capabilities.len()
        ),
        format!(
            "{}:{}",
            ExecutionPreflightReceiptLabel::BLOCKERS.as_str(),
            preflight.blockers.len()
        ),
    ];
    evidence.extend(preflight.missing_capabilities.iter().map(|capability| {
        format!(
            "{}:{capability:?}",
            ExecutionPreflightReceiptLabel::MISSING_CAPABILITY.as_str()
        )
    }));
    evidence.extend(preflight.blockers.iter().map(|finding| {
        format!(
            "{}:{}:{}",
            ExecutionPreflightReceiptLabel::BLOCKER.as_str(),
            finding.code,
            finding.message
        )
    }));
    evidence
}
