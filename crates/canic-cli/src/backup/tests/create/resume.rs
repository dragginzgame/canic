//! Module: backup::tests::create::resume
//!
//! Responsibility: backup create resume boundary tests.
//! Does not own: incompatible layout rejection cases.
//! Boundary: reuse of existing execution-backed dry-run layouts.

use super::super::super::*;
use super::super::fixtures::*;
use crate::test_support::temp_dir;
use canic_backup::persistence::BackupLayout;
use std::fs;

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
