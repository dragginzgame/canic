//! Module: backup::tests::inspect
//!
//! Responsibility: backup inspect command behavior tests.
//! Does not own: backup persistence fixtures or inspect implementation.
//! Boundary: CLI inspect report behavior for dry-run and incomplete layouts.

use super::super::*;
use super::fixtures::*;
use crate::test_support::temp_dir;
use std::fs;

// Ensure backup inspect reads dry-run plan and execution details.
#[test]
fn backup_inspect_reads_dry_run_details() {
    let root = temp_dir("canic-cli-backup-inspect-dry-run");
    let plan = valid_backup_plan();
    persist_backup_create_dry_run(&root, &plan).expect("persist dry-run plan");

    let options = BackupInspectOptions {
        backup_ref: None,
        dir: Some(root.clone()),
        out: None,
        json: false,
    };
    let report = backup_inspect(&options).expect("inspect dry-run");

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(report.layout_status, BackupExecutionLayoutStatus::DryRun);
    assert_eq!(report.plan_id, plan.plan_id);
    assert_eq!(report.targets.len(), 1);
    assert_eq!(report.targets[0].expected_module_hash, HASH);
    assert_eq!(report.operations.len(), 10);
}

// Ensure backup inspect reports incomplete execution-backed layouts clearly.
#[test]
fn backup_inspect_rejects_manifest_layout_missing_execution_journal() {
    let root = temp_dir("canic-cli-backup-inspect-missing-execution-journal");
    write_manifest_plan_without_execution_journal(&root);

    let options = BackupInspectOptions {
        backup_ref: None,
        dir: Some(root.clone()),
        out: None,
        json: false,
    };
    let err = backup_inspect(&options).expect_err("missing execution journal rejects");

    fs::remove_dir_all(root).expect("remove temp root");
    std::assert_matches!(
        err,
        BackupCommandError::BackupLayoutIncomplete {
            missing: "backup-execution-journal.json"
        }
    );
}
