//! Module: backup::tests::status::read
//!
//! Responsibility: backup status read-path tests.
//! Does not own: execution status state-machine assertions.
//! Boundary: status command reports for persisted backup layouts.

use super::super::super::*;
use super::super::fixtures::*;
use crate::test_support::temp_dir;
use canic_backup::persistence::BackupLayout;
use std::fs;

// Ensure backup status reads the journal and reports resume actions.
#[test]
fn backup_status_reads_journal_resume_report() {
    let root = temp_dir("canic-cli-backup-status");
    let layout = BackupLayout::new(root.clone());
    layout
        .write_journal(&journal_with_checksum(HASH.to_string()))
        .expect("write journal");

    let options = BackupStatusOptions {
        backup_ref: None,
        dir: Some(root.clone()),
        out: None,
        require_complete: false,
    };
    let report = backup_status(&options).expect("read backup status");

    fs::remove_dir_all(root).expect("remove temp root");
    let BackupStatusReport::Download(report) = report else {
        panic!("expected download status");
    };
    assert_eq!(report.backup_id, "backup-test");
    assert_eq!(report.total_artifacts, 1);
    assert!(report.is_complete);
    assert_eq!(report.pending_artifacts, 0);
    assert_eq!(report.counts.skip, 1);
}

// Ensure backup status can summarize dry-run plan/execution layouts.
#[test]
fn backup_status_reads_dry_run_execution_summary() {
    let root = temp_dir("canic-cli-backup-status-dry-run");
    let plan = valid_backup_plan();
    persist_backup_create_dry_run(&root, &plan).expect("persist dry-run plan");

    let options = BackupStatusOptions {
        backup_ref: None,
        dir: Some(root.clone()),
        out: None,
        require_complete: false,
    };
    let report = backup_status(&options).expect("read dry-run status");

    fs::remove_dir_all(root).expect("remove temp root");
    let BackupStatusReport::DryRun(report) = report else {
        panic!("expected dry-run status");
    };
    assert_eq!(report.layout_status, "dry-run");
    assert_eq!(report.plan_id, plan.plan_id);
    assert_eq!(report.targets, 1);
    assert_eq!(report.execution.plan_id, plan.plan_id);
    assert!(!report.execution.preflight_accepted);
    assert!(report.execution.blocked_operations > 0);
}

// Ensure backup status reports incomplete execution-backed layouts clearly.
#[test]
fn backup_status_rejects_manifest_layout_missing_execution_journal() {
    let root = temp_dir("canic-cli-backup-status-missing-execution-journal");
    write_manifest_plan_without_execution_journal(&root);

    let options = BackupStatusOptions {
        backup_ref: None,
        dir: Some(root.clone()),
        out: None,
        require_complete: false,
    };
    let err = backup_status(&options).expect_err("missing execution journal rejects");

    fs::remove_dir_all(root).expect("remove temp root");
    std::assert_matches!(
        err,
        BackupCommandError::BackupLayoutIncomplete {
            missing: "backup-execution-journal.json"
        }
    );
}
