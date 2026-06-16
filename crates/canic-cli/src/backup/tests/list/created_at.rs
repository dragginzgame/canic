//! Module: backup::tests::list::created_at
//!
//! Responsibility: backup list timestamp source tests.
//! Does not own: status classification or scan breadth.
//! Boundary: created-at values for planned execution layouts.

use super::super::super::*;
use super::super::fixtures::*;
use crate::test_support::temp_dir;
use canic_backup::persistence::BackupLayout;
use std::fs;

// Ensure unfinished execution layouts use the journal timestamp, not a raw run-id stamp.
#[test]
fn backup_list_uses_execution_journal_timestamp_for_planned_layouts() {
    let root = temp_dir("canic-cli-backup-list-created-at-journal");
    let planned = root.join("deployment-demo-20260511-001234");
    let mut plan = valid_backup_plan();
    plan.plan_id = "plan-demo-20260511-001234".to_string();
    plan.run_id = "run-demo-20260511-001234".to_string();
    let layout = BackupLayout::new(planned);
    layout.write_backup_plan(&plan).expect("write backup plan");
    layout
        .write_execution_journal(&accepted_execution_journal())
        .expect("write execution journal");

    let entries = backup_list(&BackupListOptions {
        dir: root.clone(),
        out: None,
    })
    .expect("list backups");

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].created_at, "unix:10");
}
