//! Module: backup::tests::list::execution_status
//!
//! Responsibility: backup list execution status tests.
//! Does not own: generic directory scanning behavior.
//! Boundary: status labels for execution-backed manifest layouts.

use super::super::super::*;
use super::super::fixtures::*;
use crate::test_support::temp_dir;
use canic_backup::persistence::BackupLayout;
use std::{fs, path::Path};

// Ensure backup list reports execution-backed manifest layouts by execution state.
#[test]
fn backup_list_reports_execution_backed_manifest_status() {
    let root = temp_dir("canic-cli-backup-list-execution-status");
    let running = root.join("deployment-demo-20260507-140000");
    let complete = root.join("deployment-demo-20260507-150000");
    let invalid = root.join("deployment-demo-20260507-160000");
    let missing_journal = root.join("deployment-demo-20260507-170000");
    let checksum = write_artifact(&complete, b"root artifact");

    write_manifest_plan_journal(&running, accepted_execution_journal());

    let mut complete_journal = accepted_execution_journal();
    for sequence in 4..=9 {
        complete_execution_operation(&mut complete_journal, sequence);
    }
    write_manifest_plan_journal(&complete, complete_journal);
    BackupLayout::new(complete.clone())
        .write_journal(&journal_with_checksum(checksum.hash))
        .expect("write download journal");

    let mut invalid_journal = accepted_execution_journal();
    invalid_journal.operations[0].operation_id = "different-operation".to_string();
    write_manifest_plan_journal(&invalid, invalid_journal);
    let missing_layout = BackupLayout::new(missing_journal.clone());
    missing_layout
        .write_manifest(&valid_manifest())
        .expect("write missing-journal manifest");
    missing_layout
        .write_backup_plan(&valid_backup_plan())
        .expect("write missing-journal plan");

    let entries = backup_list(&BackupListOptions {
        dir: root.clone(),
        out: None,
    })
    .expect("list backups");

    fs::remove_dir_all(root).expect("remove temp root");
    let status_for = |dir: &Path| {
        entries
            .iter()
            .find(|entry| entry.dir == dir)
            .map(|entry| entry.status.as_str())
            .expect("entry exists")
    };
    assert_eq!(status_for(&running), "running");
    assert_eq!(status_for(&complete), "complete");
    assert_eq!(status_for(&invalid), "invalid-plan-journal");
    assert_eq!(status_for(&missing_journal), "invalid-plan-journal");
}
