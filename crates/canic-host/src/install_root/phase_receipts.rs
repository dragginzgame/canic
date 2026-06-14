use super::InstallPhaseOperation;
use super::receipt_io::write_install_deployment_truth_receipt;
use crate::deployment_truth::{
    DeploymentCheckV1, DeploymentCommandResultV1, DeploymentExecutionContextV1,
    DeploymentExecutionStatusV1, DeploymentReceiptV1, ObservationStatusV1, RolePhaseReceiptV1,
    RolePhaseResultV1, deployment_receipt_from_check_with_status, phase_receipt,
};
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

#[derive(Clone, Copy)]
pub(super) struct InstallReceiptScope<'a> {
    pub(super) icp_root: &'a Path,
    pub(super) network: &'a str,
    pub(super) deployment_name: &'a str,
    pub(super) check: &'a DeploymentCheckV1,
    pub(super) execution_context: Option<&'a DeploymentExecutionContextV1>,
}

pub(super) struct CompletedInstallPhase {
    pub(super) phase: &'static str,
    pub(super) attempted_action: &'static str,
    pub(super) started_at: String,
    pub(super) finished_at: Option<String>,
    pub(super) evidence: Vec<String>,
    pub(super) role_names: Vec<String>,
}

pub(super) fn write_completed_install_phase_receipt(
    receipt_scope: InstallReceiptScope<'_>,
    completed: CompletedInstallPhase,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let role_phase_receipts = completed
        .role_names
        .iter()
        .filter_map(|role| {
            completed_phase_role_receipt(
                receipt_scope.check,
                completed.phase,
                role,
                RolePhaseResultV1::Applied,
                None,
            )
        })
        .collect();
    let receipt =
        receipt_scope.with_execution_context(install_deployment_truth_phase_receipt_with_result(
            receipt_scope.check,
            PhaseReceiptInput {
                phase: completed.phase,
                started_at: completed.started_at,
                finished_at: completed.finished_at,
                attempted_action: completed.attempted_action,
                status: ObservationStatusV1::Observed,
                evidence: completed.evidence,
                role_phase_receipts,
                operation_status: DeploymentExecutionStatusV1::Complete,
                command_result: DeploymentCommandResultV1::Succeeded,
            },
        ));
    receipt_scope.write_receipt(&receipt)
}

pub(super) fn completed_phase_role_receipt(
    check: &DeploymentCheckV1,
    phase: &str,
    role: &str,
    result: RolePhaseResultV1,
    error: Option<String>,
) -> Option<RolePhaseReceiptV1> {
    let planned = check
        .plan
        .role_artifacts
        .iter()
        .find(|artifact| artifact.role == role)?;
    let observed = check
        .inventory
        .observed_artifacts
        .iter()
        .find(|artifact| artifact.role == role);
    let artifact_digest = observed
        .and_then(|artifact| artifact.file_sha256.clone())
        .or_else(|| observed.and_then(|artifact| artifact.payload_sha256.clone()))
        .or_else(|| planned.observed_wasm_gz_file_sha256.clone())
        .or_else(|| planned.wasm_gz_sha256.clone());

    Some(RolePhaseReceiptV1 {
        role: role.to_string(),
        phase: phase.to_string(),
        result,
        previous_module_hash: None,
        target_module_hash: planned.installed_module_hash.clone(),
        observed_module_hash_after: None,
        artifact_digest,
        canonical_embedded_config_sha256: planned.canonical_embedded_config_sha256.clone(),
        error,
    })
}

pub(super) fn install_deployment_truth_phase_receipt(
    check: &DeploymentCheckV1,
    phase: &str,
    started_at: String,
    finished_at: Option<String>,
    attempted_action: &str,
    status: ObservationStatusV1,
    evidence: Vec<String>,
) -> DeploymentReceiptV1 {
    install_deployment_truth_phase_receipt_with_result(
        check,
        PhaseReceiptInput {
            phase,
            started_at,
            finished_at,
            attempted_action,
            status,
            evidence,
            role_phase_receipts: Vec::new(),
            operation_status: DeploymentExecutionStatusV1::Complete,
            command_result: DeploymentCommandResultV1::Succeeded,
        },
    )
}

fn install_deployment_truth_phase_receipt_with_result(
    check: &DeploymentCheckV1,
    input: PhaseReceiptInput<'_>,
) -> DeploymentReceiptV1 {
    deployment_receipt_from_check_with_status(
        check,
        format!("{}:{}", check.check_id, input.phase),
        input.operation_status,
        input.started_at.clone(),
        input.finished_at.clone(),
        vec![phase_receipt(
            input.phase,
            input.started_at,
            input.finished_at,
            input.attempted_action,
            input.status,
            input.evidence,
        )],
        input.role_phase_receipts,
        input.command_result,
    )
}

pub(super) fn receipt_with_execution_context(
    mut receipt: DeploymentReceiptV1,
    execution_context: &DeploymentExecutionContextV1,
) -> DeploymentReceiptV1 {
    receipt.execution_context = Some(execution_context.clone());
    receipt
}

struct PhaseReceiptInput<'a> {
    phase: &'a str,
    started_at: String,
    finished_at: Option<String>,
    attempted_action: &'a str,
    status: ObservationStatusV1,
    evidence: Vec<String>,
    role_phase_receipts: Vec<RolePhaseReceiptV1>,
    operation_status: DeploymentExecutionStatusV1,
    command_result: DeploymentCommandResultV1,
}

impl InstallReceiptScope<'_> {
    pub(super) fn run_operation(
        self,
        operation: &impl InstallPhaseOperation,
    ) -> Result<Duration, Box<dyn std::error::Error>> {
        self.run_phase(
            operation.phase(),
            operation.attempted_action(),
            operation.evidence(),
            || operation.execute(),
        )
    }

    pub(super) fn run_phase(
        self,
        phase: &str,
        attempted_action: &str,
        evidence: Vec<String>,
        run: impl FnOnce() -> Result<(), Box<dyn std::error::Error>>,
    ) -> Result<Duration, Box<dyn std::error::Error>> {
        let started_at = super::current_unix_timestamp_label()?;
        let started = Instant::now();
        match run() {
            Ok(()) => {
                let duration = started.elapsed();
                let receipt = self.with_execution_context(install_deployment_truth_phase_receipt(
                    self.check,
                    phase,
                    started_at,
                    Some(super::current_unix_timestamp_label()?),
                    attempted_action,
                    ObservationStatusV1::Observed,
                    evidence,
                ));
                self.write_receipt(&receipt)?;
                Ok(duration)
            }
            Err(err) => {
                self.try_write_failed_phase_receipt(
                    phase,
                    started_at,
                    attempted_action,
                    evidence,
                    err.as_ref(),
                );
                Err(err)
            }
        }
    }

    pub(super) fn with_execution_context(
        self,
        receipt: DeploymentReceiptV1,
    ) -> DeploymentReceiptV1 {
        match self.execution_context {
            Some(context) => receipt_with_execution_context(receipt, context),
            None => receipt,
        }
    }

    fn write_receipt(
        self,
        receipt: &DeploymentReceiptV1,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let path = write_install_deployment_truth_receipt(
            self.icp_root,
            self.network,
            self.deployment_name,
            receipt,
        )?;
        println!("Deployment truth receipt JSON: {}", path.display());
        Ok(path)
    }

    fn try_write_failed_phase_receipt(
        self,
        phase: &str,
        started_at: String,
        attempted_action: &str,
        evidence: Vec<String>,
        err: &dyn std::error::Error,
    ) {
        let receipt = install_deployment_truth_phase_receipt_with_result(
            self.check,
            PhaseReceiptInput {
                phase,
                started_at,
                finished_at: Some(
                    super::current_unix_timestamp_label().unwrap_or_else(|_| "unknown".to_string()),
                ),
                attempted_action,
                status: ObservationStatusV1::Inconclusive,
                evidence,
                role_phase_receipts: Vec::new(),
                operation_status: DeploymentExecutionStatusV1::FailedAfterMutation,
                command_result: DeploymentCommandResultV1::Failed {
                    code: format!("{phase}_failed"),
                    message: err.to_string(),
                },
            },
        );
        let receipt = self.with_execution_context(receipt);
        if let Err(write_err) = self.write_receipt(&receipt) {
            eprintln!("Deployment truth receipt JSON write failed: {write_err}");
        }
    }
}
