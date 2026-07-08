//! Module: backup::tests::status::execution
//!
//! Responsibility: execution-backed backup status tests.
//! Does not own: generic completion gate assertions.
//! Boundary: status reports for running, failed, and finalized execution layouts.

use super::super::super::BackupExecutionLayoutStatus;
use super::super::fixtures::*;

// Ensure backup status reports an execution layout as running once preflight is accepted.
#[test]
fn backup_status_reports_running_execution_layout() {
    let report = backup_status_for_execution_journal(
        "canic-cli-backup-status-running",
        accepted_execution_journal(),
        false,
    );

    assert_eq!(report.layout_status, BackupExecutionLayoutStatus::Running);
    assert!(report.execution.preflight_accepted);
    assert_eq!(report.execution.failed_operations, 0);
    assert!(report.execution.ready_operations > 0);
}

// Ensure backup status reports failed execution journals without requiring a manifest.
#[test]
fn backup_status_reports_failed_execution_layout() {
    let mut journal = accepted_execution_journal();
    complete_execution_operation(&mut journal, 4);
    fail_execution_operation(&mut journal, 5, "snapshot failed");

    let report =
        backup_status_for_execution_journal("canic-cli-backup-status-failed", journal, false);

    assert_eq!(report.layout_status, BackupExecutionLayoutStatus::Failed);
    assert_eq!(report.execution.failed_operations, 1);
    assert_eq!(
        report
            .execution
            .next_operation
            .expect("failed operation")
            .sequence,
        5
    );
}

// Ensure backup status reports complete only when the execution journal is complete and a manifest exists.
#[test]
fn backup_status_reports_complete_execution_layout() {
    let mut journal = accepted_execution_journal();
    for sequence in 4..=9 {
        complete_execution_operation(&mut journal, sequence);
    }

    let report =
        backup_status_for_execution_journal("canic-cli-backup-status-complete", journal, true);

    assert_eq!(report.layout_status, BackupExecutionLayoutStatus::Complete);
    assert_eq!(
        report.execution.completed_operations + report.execution.skipped_operations,
        report.execution.total_operations
    );
}
