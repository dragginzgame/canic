//! Module: backup::tests::verify::rejection
//!
//! Responsibility: backup verification rejection tests.
//! Does not own: successful integrity-report behavior.
//! Boundary: invalid dry-run and execution-backed layouts.

use super::super::super::*;
use super::super::fixtures::*;
use crate::test_support::temp_dir;
use canic_backup::persistence::BackupLayout;
use std::fs;

// Ensure verification rejects dry-run plans with a backup-specific error.
#[test]
fn verify_backup_rejects_dry_run_layout() {
    let root = temp_dir("canic-cli-backup-verify-dry-run");
    let plan = valid_backup_plan();
    persist_backup_create_dry_run(&root, &plan).expect("persist dry-run plan");

    let options = BackupVerifyOptions {
        backup_ref: None,
        dir: Some(root.clone()),
        out: None,
    };
    let err = verify_backup(&options).expect_err("dry-run verify rejects");

    fs::remove_dir_all(root).expect("remove temp root");
    std::assert_matches!(
        err,
        BackupCommandError::DryRunNotComplete { plan_id } if plan_id == "plan-test"
    );
}

// Ensure verification reports incomplete execution-backed layouts clearly.
#[test]
fn verify_backup_rejects_manifest_layout_missing_execution_journal() {
    let root = temp_dir("canic-cli-backup-verify-missing-execution-journal");
    write_manifest_plan_without_execution_journal(&root);

    let options = BackupVerifyOptions {
        backup_ref: None,
        dir: Some(root.clone()),
        out: None,
    };
    let err = verify_backup(&options).expect_err("missing execution journal rejects");

    fs::remove_dir_all(root).expect("remove temp root");
    std::assert_matches!(
        err,
        BackupCommandError::BackupLayoutIncomplete {
            missing: "backup-execution-journal.json"
        }
    );
}

// Ensure verification rejects execution-backed layouts that finalized artifacts before execution completion.
#[test]
fn verify_backup_rejects_incomplete_execution_layout_with_manifest() {
    let root = temp_dir("canic-cli-backup-verify-incomplete-execution");
    let layout = BackupLayout::new(root.clone());
    let checksum = write_artifact(&root, b"root artifact");

    layout
        .publish_manifest(&valid_manifest())
        .expect("write manifest");
    layout
        .write_journal(&journal_with_checksum(checksum.hash))
        .expect("write journal");
    layout
        .write_backup_plan(&valid_backup_plan())
        .expect("write backup plan");
    layout
        .write_execution_journal(&accepted_execution_journal())
        .expect("write incomplete execution journal");

    let options = BackupVerifyOptions {
        backup_ref: None,
        dir: Some(root.clone()),
        out: None,
    };
    let err = verify_backup(&options).expect_err("incomplete execution rejects");

    fs::remove_dir_all(root).expect("remove temp root");
    std::assert_matches!(
        err,
        BackupCommandError::DryRunNotComplete { plan_id } if plan_id == "plan-test"
    );
}

// Ensure verification rejects execution-backed layouts whose plan and execution journal drift.
#[test]
fn verify_backup_rejects_execution_plan_journal_mismatch() {
    let root = temp_dir("canic-cli-backup-verify-execution-mismatch");
    let layout = BackupLayout::new(root.clone());
    let checksum = write_artifact(&root, b"root artifact");
    let mut journal = accepted_execution_journal();
    journal.operations[0].operation_id = "different-operation".to_string();

    layout
        .publish_manifest(&valid_manifest())
        .expect("write manifest");
    layout
        .write_journal(&journal_with_checksum(checksum.hash))
        .expect("write journal");
    layout
        .write_backup_plan(&valid_backup_plan())
        .expect("write backup plan");
    layout
        .write_execution_journal(&journal)
        .expect("write mismatched execution journal");

    let options = BackupVerifyOptions {
        backup_ref: None,
        dir: Some(root.clone()),
        out: None,
    };
    let err = verify_backup(&options).expect_err("mismatched execution rejects");

    fs::remove_dir_all(root).expect("remove temp root");
    std::assert_matches!(
        err,
        BackupCommandError::Persistence(
            canic_backup::persistence::PersistenceError::PlanJournalOperationMismatch {
                field: "operation_id",
                ..
            }
        )
    );
}
