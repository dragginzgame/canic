//! Module: backup::render::inspect
//!
//! Responsibility: render backup inspect reports.
//! Does not own: backup layout inspection, integrity checks, or output writing.
//! Boundary: table formatting for inspect summary, targets, and operations.

use super::super::BackupInspectReport;
use canic_host::table::{ColumnAlign, render_table};

pub(super) fn render_inspect_report(report: &BackupInspectReport) -> String {
    let summary_rows = [[
        report.layout_status.label().to_string(),
        report.deployment.clone(),
        report.network.clone(),
        report.scope.clone(),
        report.targets.len().to_string(),
        report.operations.len().to_string(),
        report.execution.next_operation.as_ref().map_or_else(
            || "-".to_string(),
            |operation| operation.operation_id.clone(),
        ),
    ]];
    let target_rows = report
        .targets
        .iter()
        .map(|target| {
            [
                target.role.clone(),
                target.canister_id.clone(),
                target.parent_canister_id.clone(),
                target.expected_module_hash.clone(),
                target.depth.to_string(),
                target.control_authority.clone(),
                target.snapshot_read_authority.clone(),
            ]
        })
        .collect::<Vec<_>>();
    let operation_rows = report
        .operations
        .iter()
        .map(|operation| {
            [
                operation.sequence.to_string(),
                operation.kind.clone(),
                operation.target_canister_id.clone(),
                operation.state.clone(),
                operation.blocking_reasons.join("; "),
            ]
        })
        .collect::<Vec<_>>();

    [
        format!("Plan: {}", report.plan_id),
        format!("Run:  {}", report.run_id),
        String::new(),
        render_table(
            &[
                "STATUS",
                "DEPLOYMENT",
                "NETWORK",
                "SCOPE",
                "TARGETS",
                "OPERATIONS",
                "NEXT",
            ],
            &summary_rows,
            &[ColumnAlign::Left; 7],
        ),
        String::new(),
        "Targets".to_string(),
        render_table(
            &[
                "ROLE",
                "CANISTER_ID",
                "PARENT",
                "MODULE_HASH",
                "DEPTH",
                "CONTROL",
                "SNAPSHOT_READ",
            ],
            &target_rows,
            &[ColumnAlign::Left; 7],
        ),
        String::new(),
        "Operations".to_string(),
        render_table(
            &["SEQ", "KIND", "TARGET", "STATE", "REASONS"],
            &operation_rows,
            &[
                ColumnAlign::Right,
                ColumnAlign::Left,
                ColumnAlign::Left,
                ColumnAlign::Left,
                ColumnAlign::Left,
            ],
        ),
    ]
    .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backup::{BackupInspectOperation, BackupInspectTarget};

    // Ensure backup inspect text includes summary, target, and operation sections.
    #[test]
    fn render_backup_inspect_report_shows_layout_sections() {
        let report = inspect_report();

        let rendered = render_inspect_report(&report);

        assert!(rendered.contains("DEPLOYMENT"));
        assert!(!rendered.contains("FLEET"));
        assert!(rendered.contains("Plan: plan-test"));
        assert!(rendered.contains("Targets"));
        assert!(rendered.contains("Operations"));
        assert!(rendered.contains("child"));
        assert!(rendered.contains("MODULE_HASH"));
        assert!(rendered.contains("hash-test"));
    }

    // Ensure backup inspect JSON exposes deployment identity, not stale fleet identity.
    #[test]
    fn backup_inspect_report_json_uses_deployment_identity_field() {
        let value = serde_json::to_value(inspect_report()).expect("serialize inspect report");

        assert_eq!(value["layout_status"], "dry-run");
        assert_eq!(value["deployment"], "demo");
        assert!(value.get("fleet").is_none());
    }

    fn inspect_report() -> BackupInspectReport {
        BackupInspectReport {
            layout_status: crate::backup::BackupExecutionLayoutStatus::DryRun,
            plan_id: "plan-test".to_string(),
            run_id: "run-test".to_string(),
            deployment: "demo".to_string(),
            network: "local".to_string(),
            scope: "non-root-deployment".to_string(),
            targets: vec![BackupInspectTarget {
                role: "child".to_string(),
                canister_id: "aaaaa-aa".to_string(),
                parent_canister_id: "bbbbb-bb".to_string(),
                expected_module_hash: "hash-test".to_string(),
                depth: 1,
                control_authority: "root-controller/proven".to_string(),
                snapshot_read_authority: "root-configured-read/proven".to_string(),
            }],
            operations: vec![BackupInspectOperation {
                sequence: 1,
                kind: "create-snapshot".to_string(),
                target_canister_id: "aaaaa-aa".to_string(),
                state: "ready".to_string(),
                blocking_reasons: Vec::new(),
            }],
            execution: canic_backup::execution::BackupExecutionResumeSummary {
                plan_id: "plan-test".to_string(),
                run_id: "run-test".to_string(),
                preflight_id: None,
                preflight_accepted: false,
                restart_required: false,
                total_operations: 1,
                ready_operations: 1,
                pending_operations: 0,
                blocked_operations: 0,
                completed_operations: 0,
                failed_operations: 0,
                skipped_operations: 0,
                next_operation: None,
            },
        }
    }
}
