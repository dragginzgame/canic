use super::clock::current_unix_timestamp_label;
use crate::deployment_truth::{
    DeploymentCheckV1, DeploymentCommandResultV1, DeploymentExecutionStatusV1, DeploymentReceiptV1,
    PhaseReceiptV1, RolePhaseReceiptV1, SafetyFindingV1, deployment_receipt_from_check_with_status,
};

pub(super) fn enforce_install_deployment_truth_gate(
    check: &DeploymentCheckV1,
) -> Result<(), Box<dyn std::error::Error>> {
    let blockers = install_deployment_truth_gate_blockers(check);
    if blockers.is_empty() {
        return Ok(());
    }

    let details = blockers
        .iter()
        .map(|finding| deployment_truth_finding_label(finding))
        .collect::<Vec<_>>()
        .join("; ");
    Err(format!("deployment truth safety gate blocked install: {details}").into())
}

fn install_deployment_truth_gate_blockers(check: &DeploymentCheckV1) -> Vec<&SafetyFindingV1> {
    check.report.hard_failures.iter().collect()
}

pub(super) fn print_install_deployment_truth_gate(
    check: &DeploymentCheckV1,
    receipt: &DeploymentReceiptV1,
) {
    for line in install_deployment_truth_gate_lines(check, receipt) {
        println!("{line}");
    }
}

pub(super) fn install_deployment_truth_gate_lines(
    check: &DeploymentCheckV1,
    receipt: &DeploymentReceiptV1,
) -> Vec<String> {
    let mut lines = vec![
        format!("Deployment truth: {}", check.report.summary),
        format!(
            "Deployment truth receipt: operation={} status={:?}",
            receipt.operation_id, receipt.operation_status
        ),
    ];
    for phase_receipt in &receipt.phase_receipts {
        lines.push(format!(
            "Deployment truth phase receipt: phase={} postcondition={:?}",
            phase_receipt.phase, phase_receipt.verified_postcondition.status
        ));
    }
    if !receipt.role_phase_receipts.is_empty() {
        lines.push(format!(
            "Deployment truth role receipts: {}",
            receipt.role_phase_receipts.len()
        ));
    }
    for role_receipt in &receipt.role_phase_receipts {
        lines.push(format!(
            "Deployment truth role receipt: phase={} role={} result={:?}",
            role_receipt.phase, role_receipt.role, role_receipt.result
        ));
    }

    if !check.report.hard_failures.is_empty() {
        lines.push(format!(
            "Deployment truth hard failures: {}",
            check.report.hard_failures.len()
        ));
    }
    for finding in install_deployment_truth_gate_blockers(check) {
        lines.push(format!(
            "Deployment truth blocker: {}",
            deployment_truth_finding_label(finding)
        ));
    }
    if !check.report.warnings.is_empty() {
        lines.push(format!(
            "Deployment truth warnings: {}",
            check.report.warnings.len()
        ));
    }
    for finding in &check.report.warnings {
        lines.push(format!(
            "Deployment truth warning: {}",
            deployment_truth_finding_label(finding)
        ));
    }
    lines
}

pub(super) fn install_deployment_truth_gate_receipt(
    check: &DeploymentCheckV1,
    started_at: String,
    phase_receipts: Vec<PhaseReceiptV1>,
    role_phase_receipts: Vec<RolePhaseReceiptV1>,
) -> DeploymentReceiptV1 {
    let blockers = install_deployment_truth_gate_blockers(check);
    let (operation_status, command_result) = if blockers.is_empty() {
        (
            DeploymentExecutionStatusV1::Complete,
            DeploymentCommandResultV1::Succeeded,
        )
    } else {
        (
            DeploymentExecutionStatusV1::FailedBeforeMutation,
            DeploymentCommandResultV1::Failed {
                code: "deployment_truth_blocked".to_string(),
                message: check.report.summary.clone(),
            },
        )
    };
    deployment_receipt_from_check_with_status(
        check,
        format!("{}:materialize_artifacts", check.check_id),
        operation_status,
        started_at,
        Some(current_unix_timestamp_label().unwrap_or_else(|_| "unknown".to_string())),
        phase_receipts,
        role_phase_receipts,
        command_result,
    )
}

pub(super) fn deployment_truth_finding_label(finding: &SafetyFindingV1) -> String {
    let subject = finding
        .subject
        .as_ref()
        .map_or_else(|| "<none>".to_string(), Clone::clone);
    format!(
        "{}:{}:{}: {}",
        deployment_truth_finding_source(&finding.code),
        finding.code,
        subject,
        finding.message
    )
}

fn deployment_truth_finding_source(code: &str) -> &'static str {
    match code {
        "plan_assumption" => "plan",
        "observation_gap" => "inventory",
        _ => "diff",
    }
}
