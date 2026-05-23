use super::*;

/// Build an evidence-only receipt for a completed dry-run authority
/// reconciliation.
///
/// The receipt records that no controller mutations were attempted. The
/// original plan/report remain the authority for whether later apply work is
/// safe.
#[must_use]
pub fn authority_dry_run_receipt_from_plan(
    plan: &AuthorityReconciliationPlanV1,
    report: &AuthorityReportV1,
    operation_id: impl Into<String>,
    started_at: impl Into<String>,
    finished_at: Option<String>,
) -> AuthorityReceiptV1 {
    AuthorityReceiptV1 {
        schema_version: DEPLOYMENT_TRUTH_SCHEMA_VERSION,
        operation_id: operation_id.into(),
        reconciliation_plan_id: plan.plan_id.clone(),
        operation_status: DeploymentExecutionStatusV1::Complete,
        started_at: started_at.into(),
        finished_at,
        attempted_actions: Vec::new(),
        verified_controller_observations: plan
            .canister_actions
            .iter()
            .map(authority_controller_observation_from_action)
            .collect(),
        unresolved_external_actions: report.external_actions_required.clone(),
        command_result: DeploymentCommandResultV1::Succeeded,
    }
}

/// Build a lightweight receipt for the current-install artifact materialization
/// gate. The receipt is evidence only; live inventory/check data remains the
/// authority for any installer decision.
#[must_use]
pub fn artifact_gate_phase_receipt(
    check: &DeploymentCheckV1,
    started_at: impl Into<String>,
    finished_at: Option<String>,
) -> PhaseReceiptV1 {
    let missing = check
        .report
        .hard_failures
        .iter()
        .filter(|finding| finding.code == "artifact_missing")
        .collect::<Vec<_>>();
    let mut evidence = check
        .inventory
        .observed_artifacts
        .iter()
        .filter_map(|artifact| {
            artifact
                .file_sha256
                .as_ref()
                .map(|hash| format!("artifact:{}:sha256:{hash}", artifact.role))
        })
        .collect::<Vec<_>>();
    evidence.extend(
        missing
            .iter()
            .filter_map(|finding| finding.subject.as_ref())
            .map(|role| format!("artifact:{role}:missing")),
    );
    let status = if missing.is_empty() {
        ObservationStatusV1::Observed
    } else {
        ObservationStatusV1::Missing
    };

    phase_receipt(
        "materialize_artifacts",
        started_at,
        finished_at,
        "verify configured role artifacts are materialized",
        status,
        evidence,
    )
}

/// Build role-scoped evidence for the current-install artifact materialization
/// gate.
///
/// These records do not decide safety; they preserve the per-role facts already
/// present in the check so later resume/reporting work can distinguish which
/// roles were verified and which failed materialization.
#[must_use]
pub fn artifact_gate_role_phase_receipts(check: &DeploymentCheckV1) -> Vec<RolePhaseReceiptV1> {
    check
        .plan
        .role_artifacts
        .iter()
        .map(|planned| {
            let observed = check
                .inventory
                .observed_artifacts
                .iter()
                .find(|artifact| artifact.role == planned.role);
            let failures = check
                .report
                .hard_failures
                .iter()
                .filter(|finding| finding.subject.as_deref() == Some(planned.role.as_str()))
                .filter(|finding| finding.code.starts_with("artifact_"))
                .collect::<Vec<_>>();
            let error = if failures.is_empty() {
                None
            } else {
                Some(
                    failures
                        .iter()
                        .map(|finding| format!("{}: {}", finding.code, finding.message))
                        .collect::<Vec<_>>()
                        .join("; "),
                )
            };
            let artifact_digest = observed
                .and_then(|artifact| artifact.file_sha256.clone())
                .or_else(|| observed.and_then(|artifact| artifact.payload_sha256.clone()))
                .or_else(|| planned.observed_wasm_gz_file_sha256.clone())
                .or_else(|| planned.wasm_gz_sha256.clone());
            let result = if !failures.is_empty() {
                RolePhaseResultV1::Failed
            } else if observed
                .and_then(|artifact| artifact.file_sha256.as_ref())
                .is_some()
            {
                RolePhaseResultV1::VerifiedAlreadyApplied
            } else {
                RolePhaseResultV1::NotAttempted
            };

            RolePhaseReceiptV1 {
                role: planned.role.clone(),
                phase: "materialize_artifacts".to_string(),
                result,
                previous_module_hash: None,
                target_module_hash: planned.installed_module_hash.clone(),
                observed_module_hash_after: None,
                artifact_digest,
                canonical_embedded_config_sha256: planned.canonical_embedded_config_sha256.clone(),
                error,
            }
        })
        .collect()
}

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
    let operation_status = operation_status_for_command_result(&command_result);
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
#[allow(clippy::too_many_arguments)]
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

fn authority_controller_observation_from_action(
    action: &CanisterAuthorityActionV1,
) -> AuthorityControllerObservationV1 {
    AuthorityControllerObservationV1 {
        subject: authority_action_subject(action),
        canister_id: action.canister_id.clone(),
        role: action.role.clone(),
        state: action.state,
        action: action.action,
        observed_controllers: action.observed_controllers.clone(),
        desired_controllers: action.desired_controllers.clone(),
    }
}

fn authority_action_subject(action: &CanisterAuthorityActionV1) -> String {
    action
        .canister_id
        .clone()
        .or_else(|| action.role.as_ref().map(|role| format!("role:{role}")))
        .unwrap_or_else(|| "unknown".to_string())
}

const fn operation_status_for_command_result(
    result: &DeploymentCommandResultV1,
) -> DeploymentExecutionStatusV1 {
    match result {
        DeploymentCommandResultV1::NotFinished => DeploymentExecutionStatusV1::InProgress,
        DeploymentCommandResultV1::Succeeded => DeploymentExecutionStatusV1::Complete,
        DeploymentCommandResultV1::Failed { .. } => {
            DeploymentExecutionStatusV1::FailedAfterMutation
        }
    }
}
