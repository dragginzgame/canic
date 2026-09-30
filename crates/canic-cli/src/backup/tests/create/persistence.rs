//! Module: backup::tests::create::persistence
//!
//! Responsibility: backup create persistence success-path tests.
//! Does not own: resume conflict behavior.
//! Boundary: dry-run layout files written by backup create.

use super::super::super::*;
use super::super::fixtures::*;
use crate::test_support::temp_dir;
use canic_backup::{persistence::BackupLayout, runner::BackupRunnerError};
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

#[test]
fn backup_create_rejects_concurrent_execution_before_reading_or_publishing_layout() {
    for existing in [false, true] {
        let root = temp_dir("canic-cli-backup-create-locked");
        fs::create_dir_all(&root).expect("create backup directory");
        let plan = valid_backup_plan();
        if existing {
            persist_backup_create_dry_run(&root, &plan).expect("prepare existing layout");
        }
        let layout = BackupLayout::new(root.clone());
        let plan_before = fs::read(layout.backup_plan_path()).ok();
        let journal_before = fs::read(layout.execution_journal_path()).ok();
        let owner = layout
            .lock_execution()
            .expect("hold runner's execution lock");
        let contender_root = root.clone();
        let contender_plan = plan.clone();
        let error = std::thread::spawn(move || {
            persist_backup_create_dry_run(&contender_root, &contender_plan)
        })
        .join()
        .expect("join concurrent create")
        .expect_err("active execution owner blocks layout publication");

        std::assert_matches!(
            error,
            BackupCommandError::BackupRunner(BackupRunnerError::JournalLocked { .. })
        );
        assert_eq!(fs::read(layout.backup_plan_path()).ok(), plan_before);
        assert_eq!(
            fs::read(layout.execution_journal_path()).ok(),
            journal_before
        );
        drop(owner);
        let (_, reused) = persist_backup_create_dry_run_with_layout(&root, &plan)
            .expect("create or resume after the owner releases its lock");
        assert_eq!(reused, existing);
        assert!(
            layout
                .verify_execution_integrity()
                .expect("verify layout")
                .verified
        );
        fs::remove_dir_all(root).expect("remove locked layout");
    }
}
