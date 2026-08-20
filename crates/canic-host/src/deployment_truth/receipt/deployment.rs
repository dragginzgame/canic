use super::super::*;

/// Build one phase receipt with a verified postcondition.
#[must_use]
pub fn phase_receipt(
    phase: impl Into<String>,
    started_at: impl Into<String>,
    finished_at: Option<String>,
    attempted_action: impl Into<String>,
    status: ObservationStatusV1,
    evidence: Vec<String>,
) -> PhaseReceiptV1 {
    PhaseReceiptV1 {
        phase: phase.into(),
        started_at: started_at.into(),
        finished_at,
        attempted_action: attempted_action.into(),
        verified_postcondition: VerifiedPostconditionV1 { status, evidence },
    }
}

/// Build a deployment receipt from a validated check and phase receipts.
#[must_use]
pub fn deployment_receipt_from_check(
    check: &DeploymentCheckV1,
    operation_id: impl Into<String>,
    started_at: impl Into<String>,
    finished_at: Option<String>,
    phase_receipts: Vec<PhaseReceiptV1>,
    role_phase_receipts: Vec<RolePhaseReceiptV1>,
    command_result: DeploymentCommandResultV1,
) -> DeploymentReceiptV1 {
    let operation_status =
        deployment_execution_status_for_receipt_parts(&command_result, &role_phase_receipts);
    deployment_receipt_from_check_with_status(
        check,
        operation_id,
        operation_status,
        started_at,
        finished_at,
        phase_receipts,
        role_phase_receipts,
        command_result,
    )
}

/// Build a deployment receipt when the caller knows whether failure happened
/// before or after mutation.
#[must_use]
#[expect(clippy::too_many_arguments)]
pub fn deployment_receipt_from_check_with_status(
    check: &DeploymentCheckV1,
    operation_id: impl Into<String>,
    operation_status: DeploymentExecutionStatusV1,
    started_at: impl Into<String>,
    finished_at: Option<String>,
    phase_receipts: Vec<PhaseReceiptV1>,
    role_phase_receipts: Vec<RolePhaseReceiptV1>,
    command_result: DeploymentCommandResultV1,
) -> DeploymentReceiptV1 {
    DeploymentReceiptV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        operation_id: operation_id.into(),
        plan_id: check.plan.plan_id.clone(),
        fresh_fleet_decision: None,
        execution_context: None,
        operation_status,
        started_at: started_at.into(),
        finished_at,
        operator_principal: None,
        root_principal: check
            .inventory
            .observed_identity
            .as_ref()
            .and_then(|identity| identity.root_principal.clone())
            .or_else(|| check.plan.deployment_identity.root_principal.clone()),
        previous_observed_deployment_epoch: None,
        phase_receipts,
        role_phase_receipts,
        final_inventory_id: Some(check.inventory.inventory_id.clone()),
        command_result,
    }
}

/// Derive deployment execution status from command and role-phase receipts.
///
/// Explicit status can still be supplied when the caller knows a failed phase
/// happened after a mutation that cannot be represented by role receipts yet.
/// The generic classifier is deliberately conservative: a failed command is
/// considered partial only when receipts show both applied and failed role
/// work, and failed after mutation only when at least one role was applied.
#[must_use]
pub fn deployment_execution_status_for_receipt_parts(
    command_result: &DeploymentCommandResultV1,
    role_phase_receipts: &[RolePhaseReceiptV1],
) -> DeploymentExecutionStatusV1 {
    match command_result {
        DeploymentCommandResultV1::NotFinished => DeploymentExecutionStatusV1::InProgress,
        DeploymentCommandResultV1::Succeeded => DeploymentExecutionStatusV1::Complete,
        DeploymentCommandResultV1::Failed { .. } => {
            failed_execution_status_from_role_phase_receipts(role_phase_receipts)
        }
    }
}

fn failed_execution_status_from_role_phase_receipts(
    role_phase_receipts: &[RolePhaseReceiptV1],
) -> DeploymentExecutionStatusV1 {
    let applied = role_phase_receipts
        .iter()
        .any(|receipt| receipt.result == RolePhaseResultV1::Applied);
    let failed = role_phase_receipts
        .iter()
        .any(|receipt| receipt.result == RolePhaseResultV1::Failed);

    match (applied, failed) {
        (true, true) => DeploymentExecutionStatusV1::PartiallyApplied,
        (true, false) => DeploymentExecutionStatusV1::FailedAfterMutation,
        (false, _) => DeploymentExecutionStatusV1::FailedBeforeMutation,
    }
}
