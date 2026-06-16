//! Module: backup::tests::list::scan
//!
//! Responsibility: backup list directory scanning tests.
//! Does not own: execution-backed status classification details.
//! Boundary: list entries produced from manifest and planned backup layouts.

use super::super::super::*;
use super::super::fixtures::*;
use crate::test_support::temp_dir;
use canic_backup::{execution::BackupExecutionJournal, persistence::BackupLayout};
use std::fs;

// Ensure backup list scans manifest-bearing directories and renders reusable paths.
#[test]
fn backup_list_reads_backup_directories() {
    let root = temp_dir("canic-cli-backup-list");
    let first = root.join("deployment-demo-20260507-120000");
    let second = root.join("deployment-demo-20260507-130000");
    let planned = root.join("deployment-demo-20260511-001234");
    let ignored = root.join("not-a-backup");

    BackupLayout::new(first)
        .write_manifest(&valid_manifest_with("backup-old", "2026-05-07T12:00:00Z"))
        .expect("write first manifest");
    BackupLayout::new(second)
        .write_manifest(&valid_manifest_with("backup-new", "2026-05-07T13:00:00Z"))
        .expect("write second manifest");
    let mut plan = valid_backup_plan();
    plan.plan_id = "plan-demo-20260511-001234".to_string();
    plan.run_id = "run-demo-20260511-001234".to_string();
    let planned_layout = BackupLayout::new(planned);
    planned_layout
        .write_backup_plan(&plan)
        .expect("write planned backup");
    planned_layout
        .write_execution_journal(
            &BackupExecutionJournal::from_plan(&plan).expect("execution journal"),
        )
        .expect("write planned journal");
    fs::create_dir_all(&ignored).expect("create ignored dir");

    let options = BackupListOptions {
        dir: root.clone(),
        out: None,
    };
    let entries = backup_list(&options).expect("list backups");

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(entries.len(), 3);
    assert!(entries.iter().any(|entry| entry.backup_id == "backup-new"));
    assert!(entries.iter().any(|entry| entry.backup_id == "backup-old"));
    let dry_run = entries
        .iter()
        .find(|entry| entry.backup_id == "plan-demo-20260511-001234")
        .expect("dry-run entry");
    assert_eq!(dry_run.status, "dry-run");
    assert_eq!(dry_run.members, 1);
    assert_eq!(dry_run.created_at, unix_marker_for_stamp("20260511-001234"));
}
