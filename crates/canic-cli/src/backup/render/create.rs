//! Module: backup::render::create
//!
//! Responsibility: render backup create reports.
//! Does not own: command execution, backup planning, or output writing.
//! Boundary: table formatting for create command summaries.

use super::super::BackupCreateReport;
use canic_host::table::{ColumnAlign, render_table};

pub(super) fn render_create_report(report: &BackupCreateReport) -> String {
    let rows = [[
        report.deployment.clone(),
        report.network.clone(),
        report.mode.label().to_string(),
        report.layout.label().to_string(),
        report.status.label().to_string(),
        report.scope.clone(),
        report.targets.to_string(),
        report.operations.to_string(),
        report.executed_operations.to_string(),
        report.out.display().to_string(),
    ]];
    render_table(
        &[
            "DEPLOYMENT",
            "NETWORK",
            "MODE",
            "LAYOUT",
            "STATUS",
            "SCOPE",
            "TARGETS",
            "OPERATIONS",
            "EXECUTED",
            "OUT",
        ],
        &rows,
        &[ColumnAlign::Left; 10],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // Ensure backup create output makes reused output layouts visible.
    #[test]
    fn render_backup_create_report_shows_layout_source() {
        let report = BackupCreateReport {
            deployment: "demo".to_string(),
            network: "local".to_string(),
            out: PathBuf::from("backups/demo"),
            plan_id: "plan-demo".to_string(),
            run_id: "run-demo".to_string(),
            mode: crate::backup::BackupCreateMode::DryRun,
            layout: crate::backup::BackupCreateLayout::Existing,
            status: crate::backup::BackupRunStatus::Planned,
            scope: "non-root-deployment".to_string(),
            targets: 2,
            operations: 3,
            executed_operations: 0,
        };

        let text = render_create_report(&report);

        assert!(text.contains("DEPLOYMENT"));
        assert!(!text.contains("FLEET"));
        assert!(text.contains("LAYOUT"));
        assert!(text.contains("existing"));
    }
}
