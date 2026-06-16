//! Module: backup::tests::create
//!
//! Responsibility: backup create dry-run persistence tests.
//! Does not own: backup planning, execution, or shared fixture construction.
//! Boundary: CLI dry-run layout persistence behavior.

use super::super::*;
use super::fixtures::*;
use crate::test_support::temp_dir;
use canic_backup::persistence::BackupLayout;
use std::fs;

// Ensure dry-run persistence writes a plan and matching execution journal.
#[test]
fn backup_create_dry_run_persists_plan_and_execution_journal() {
    let root = temp_dir("canic-cli-backup-create-plan");
    let plan = valid_backup_plan();

    let persisted = persist_backup_create_dry_run(&root, &plan).expect("persist dry-run plan");

    let layout = BackupLayout::new(root.clone());
    let read_plan = layout.read_backup_plan().expect("read backup plan");
    let journal = layout
        .read_execution_journal()
        .expect("read execution journal");
    let report = layout
        .verify_execution_integrity()
        .expect("verify execution integrity");

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(persisted.plan_id, plan.plan_id);
    assert_eq!(read_plan.plan_id, plan.plan_id);
    assert_eq!(journal.plan_id, plan.plan_id);
    assert!(report.verified);
}

// Ensure dry-run persistence reports whether it created or reused a layout.
#[test]
fn backup_create_persistence_reports_layout_source() {
    let root = temp_dir("canic-cli-backup-create-layout-source");
    let plan = valid_backup_plan();

    let (created, created_from_existing) =
        persist_backup_create_dry_run_with_layout(&root, &plan).expect("persist new layout");
    let (resumed, resumed_from_existing) =
        persist_backup_create_dry_run_with_layout(&root, &plan).expect("reuse existing layout");

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(created.plan_id, plan.plan_id);
    assert_eq!(resumed.plan_id, plan.plan_id);
    assert!(!created_from_existing);
    assert!(resumed_from_existing);
}

// Ensure backup create uses an existing output layout as the resume boundary.
#[test]
fn backup_create_persistence_preserves_existing_execution_layout() {
    let root = temp_dir("canic-cli-backup-create-resume");
    let plan = valid_backup_plan();
    persist_backup_create_dry_run(&root, &plan).expect("persist initial plan");
    let layout = BackupLayout::new(root.clone());
    let mut journal = accepted_execution_journal();
    complete_execution_operation(&mut journal, 4);
    layout
        .write_execution_journal(&journal)
        .expect("write progressed execution journal");
    let mut replacement = valid_backup_plan();
    replacement.plan_id = "plan-replacement".to_string();
    replacement.run_id = "run-replacement".to_string();

    let resumed =
        persist_backup_create_dry_run(&root, &replacement).expect("reuse existing layout");
    let read_plan = layout.read_backup_plan().expect("read backup plan");
    let read_journal = layout
        .read_execution_journal()
        .expect("read execution journal");
    let summary = read_journal.resume_summary();

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(resumed.plan_id, plan.plan_id);
    assert_eq!(read_plan.plan_id, plan.plan_id);
    assert_eq!(summary.completed_operations, 5);
    assert_eq!(summary.next_operation.expect("next operation").sequence, 5);
}

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
