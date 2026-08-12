use super::clock::current_unix_timestamp_label;
use super::operations::InstallPhaseLabel;
use super::output::TerminalStyle;
use super::{InstallRootBlockKind, InstallRootBlockedError};
use crate::deployment_truth::{
    DeploymentCheckV1, DeploymentCommandResultV1, DeploymentExecutionStatusV1, DeploymentReceiptV1,
    PhaseReceiptV1, RolePhaseReceiptV1, SafetyFindingV1, deployment_receipt_from_check_with_status,
};
use crate::table::{ColumnAlign, render_bordered_table};
use std::{collections::BTreeMap, fmt::Write as _};

const DEPLOYMENT_MANIFEST_PATH_SUBJECT: &str = "deployment_manifest.path";
const LOCAL_RELEASE_SET_MANIFEST_SUBJECT: &str = "local_artifacts.release_set_manifest";

pub(super) fn enforce_install_deployment_truth_gate(
    check: &DeploymentCheckV1,
) -> Result<(), Box<dyn std::error::Error>> {
    let blockers = install_deployment_truth_gate_blockers(check);
    if blockers.is_empty() {
        return Ok(());
    }

    let details = deployment_truth_findings_summary(&blockers);
    Err(Box::new(InstallRootBlockedError::new(
        InstallRootBlockKind::DeploymentTruth,
        format!("deployment truth safety gate blocked install: {details}"),
    )))
}

fn install_deployment_truth_gate_blockers(check: &DeploymentCheckV1) -> Vec<&SafetyFindingV1> {
    check.report.hard_failures.iter().collect()
}

pub(super) fn print_install_deployment_truth_gate(
    check: &DeploymentCheckV1,
    receipt: &DeploymentReceiptV1,
) {
    TerminalStyle::detected()
        .print_section("Deployment truth", &deployment_truth_display_summary(check));
    for line in install_deployment_truth_gate_lines(check, receipt) {
        println!("{line}");
    }
    println!();
}

pub(super) fn deployment_truth_display_summary(check: &DeploymentCheckV1) -> String {
    let deferred_warning_count = check
        .report
        .warnings
        .iter()
        .filter(|finding| is_pre_manifest_observation(finding))
        .count();
    let visible_warning_count = check.report.warnings.len() - deferred_warning_count;
    let blocker_count = check.report.hard_failures.len();
    let mut parts = Vec::new();

    if blocker_count > 0 {
        parts.push(counted_label(blocker_count, "blocker"));
    }
    if visible_warning_count > 0 {
        parts.push(counted_label(visible_warning_count, "warning"));
    }
    if deferred_warning_count > 0 {
        parts.push(format!(
            "{} pending manifest {}",
            deferred_warning_count,
            if deferred_warning_count == 1 {
                "check"
            } else {
                "checks"
            }
        ));
    }
    if parts.is_empty() {
        return "ready".to_string();
    }
    parts.join("; ")
}

fn counted_label(count: usize, label: &str) -> String {
    format!("{count} {label}{}", if count == 1 { "" } else { "s" })
}

pub(super) fn install_deployment_truth_gate_lines(
    check: &DeploymentCheckV1,
    receipt: &DeploymentReceiptV1,
) -> Vec<String> {
    let deferred_warning_count = check
        .report
        .warnings
        .iter()
        .filter(|finding| is_pre_manifest_observation(finding))
        .count();
    let visible_warnings = check
        .report
        .warnings
        .iter()
        .filter(|finding| !is_pre_manifest_observation(finding))
        .collect::<Vec<_>>();
    let phase = receipt
        .phase_receipts
        .first()
        .map_or("-", |phase| phase.phase.as_str());
    let postcondition = receipt.phase_receipts.first().map_or_else(
        || "-".to_string(),
        |phase| format!("{:?}", phase.verified_postcondition.status),
    );
    let summary_rows = [[
        format!("{:?}", receipt.operation_status),
        phase.to_string(),
        postcondition,
        receipt.role_phase_receipts.len().to_string(),
        visible_warnings.len().to_string(),
        check.report.hard_failures.len().to_string(),
    ]];
    let mut lines = vec![
        format!("operation: {}", receipt.operation_id),
        render_bordered_table(
            &[
                "STATUS", "PHASE", "OBSERVED", "ROLES", "WARNINGS", "BLOCKERS",
            ],
            &summary_rows,
            &[
                ColumnAlign::Left,
                ColumnAlign::Left,
                ColumnAlign::Left,
                ColumnAlign::Right,
                ColumnAlign::Right,
                ColumnAlign::Right,
            ],
        ),
    ];

    if !check.report.hard_failures.is_empty() {
        lines.push("blockers:".to_string());
        lines.push(render_finding_table(
            install_deployment_truth_gate_blockers(check),
        ));
    }

    if !visible_warnings.is_empty() {
        lines.push("warnings:".to_string());
        lines.push(render_finding_table(visible_warnings));
    }
    if deferred_warning_count > 0 {
        lines.push(format!(
            "manifest observations: {deferred_warning_count} pending until emit_manifest"
        ));
    }
    lines
}

fn render_finding_table(findings: Vec<&SafetyFindingV1>) -> String {
    let rows = grouped_findings(&findings)
        .into_iter()
        .map(|group| {
            [
                group.source.to_string(),
                group.code,
                group.count.to_string(),
                compact_subjects(&group.subjects),
            ]
        })
        .collect::<Vec<_>>();
    render_bordered_table(
        &["SOURCE", "CODE", "COUNT", "SUBJECTS"],
        &rows,
        &[
            ColumnAlign::Left,
            ColumnAlign::Left,
            ColumnAlign::Right,
            ColumnAlign::Left,
        ],
    )
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
        format!(
            "{}:{}",
            check.check_id,
            InstallPhaseLabel::MATERIALIZE_ARTIFACTS.as_str()
        ),
        operation_status,
        started_at,
        Some(current_unix_timestamp_label().unwrap_or_else(|_| "unknown".to_string())),
        phase_receipts,
        role_phase_receipts,
        command_result,
    )
}

pub(super) fn deployment_truth_findings_summary(findings: &[&SafetyFindingV1]) -> String {
    grouped_findings(findings)
        .into_iter()
        .map(|group| {
            let subjects = compact_subjects(&group.subjects);
            format!(
                "{}:{} [{subjects}]: {}",
                group.source, group.code, group.message
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

struct GroupedFinding {
    source: &'static str,
    code: String,
    count: usize,
    subjects: Vec<String>,
    message: String,
}

fn grouped_findings(findings: &[&SafetyFindingV1]) -> Vec<GroupedFinding> {
    let mut groups = BTreeMap::<(&'static str, &str, &str), Vec<&SafetyFindingV1>>::new();
    for finding in findings {
        groups
            .entry((
                deployment_truth_finding_source(&finding.code),
                &finding.code,
                &finding.message,
            ))
            .or_default()
            .push(finding);
    }

    groups
        .into_iter()
        .map(|((source, code, message), findings)| GroupedFinding {
            source,
            code: code.to_string(),
            count: findings.len(),
            subjects: findings
                .iter()
                .map(|finding| {
                    finding
                        .subject
                        .clone()
                        .unwrap_or_else(|| "<none>".to_string())
                })
                .collect(),
            message: message.to_string(),
        })
        .collect()
}

fn compact_subjects(subjects: &[String]) -> String {
    const VISIBLE_SUBJECTS: usize = 4;
    let mut label = subjects
        .iter()
        .take(VISIBLE_SUBJECTS)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    if subjects.len() > VISIBLE_SUBJECTS {
        let _ = write!(label, ", +{}", subjects.len() - VISIBLE_SUBJECTS);
    }
    label
}

fn is_pre_manifest_observation(finding: &SafetyFindingV1) -> bool {
    matches!(
        finding.subject.as_deref(),
        Some(DEPLOYMENT_MANIFEST_PATH_SUBJECT | LOCAL_RELEASE_SET_MANIFEST_SUBJECT)
    )
}

fn deployment_truth_finding_source(code: &str) -> &'static str {
    match code {
        "plan_assumption" => "plan",
        "observation_gap" => "inventory",
        _ => "diff",
    }
}
