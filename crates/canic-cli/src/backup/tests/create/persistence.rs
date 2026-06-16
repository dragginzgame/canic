//! Module: backup::tests::create::persistence
//!
//! Responsibility: backup create persistence success-path tests.
//! Does not own: resume conflict behavior.
//! Boundary: dry-run layout files written by backup create.

use super::super::super::*;
use super::super::fixtures::*;
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
