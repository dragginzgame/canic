//! Module: backup::tests::create::rejection
//!
//! Responsibility: backup create persistence rejection tests.
//! Does not own: successful persistence or resume behavior.
//! Boundary: incompatible or incomplete existing output layouts.

use super::super::super::*;
use super::super::fixtures::*;
use crate::test_support::temp_dir;
use std::fs;

// Ensure backup create does not reuse an output layout for a different request.
#[test]
fn backup_create_persistence_rejects_incompatible_existing_layout() {
    let root = temp_dir("canic-cli-backup-create-incompatible-resume");
    let plan = valid_backup_plan();
    persist_backup_create_dry_run(&root, &plan).expect("persist initial plan");
    let mut requested = valid_backup_plan();
    requested.network = "ic".to_string();

    let err = persist_backup_create_dry_run(&root, &requested)
        .expect_err("incompatible existing layout rejects");

    fs::remove_dir_all(root).expect("remove temp root");
    std::assert_matches!(
        err,
        BackupCommandError::BackupLayoutMismatch {
            field: "network",
            existing,
            requested,
        } if existing == "local" && requested == "ic"
    );
}

// Ensure dry-run layouts cannot be reused as executable backup layouts.
#[test]
fn backup_create_persistence_rejects_dry_run_layout_for_execute_request() {
    let root = temp_dir("canic-cli-backup-create-dry-run-execute-mismatch");
    let plan = valid_backup_plan();
    persist_backup_create_dry_run(&root, &plan).expect("persist dry-run plan");
    let requested = valid_executable_backup_plan();

    let err = persist_backup_create_dry_run(&root, &requested)
        .expect_err("dry-run layout rejects execute request");

    fs::remove_dir_all(root).expect("remove temp root");
    std::assert_matches!(
        err,
        BackupCommandError::BackupLayoutMismatch {
            field: "requires_root_controller",
            existing,
            requested,
        } if existing == "true" && requested == "false"
    );
}

// Ensure completed execution layouts do not synthesize a missing execution journal.
#[test]
fn backup_create_persistence_rejects_manifest_layout_missing_execution_journal() {
    let root = temp_dir("canic-cli-backup-create-missing-execution-journal");
    let plan = valid_backup_plan();
    write_manifest_plan_without_execution_journal(&root);

    let err = persist_backup_create_dry_run(&root, &plan)
        .expect_err("manifest layout missing execution journal rejects");

    fs::remove_dir_all(root).expect("remove temp root");
    std::assert_matches!(
        err,
        BackupCommandError::BackupLayoutIncomplete {
            missing: "backup-execution-journal.json"
        }
    );
}
