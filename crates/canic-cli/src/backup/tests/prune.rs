//! Module: backup::tests::prune
//!
//! Responsibility: backup prune command behavior tests.
//! Does not own: backup listing, status classification, or fixture construction.
//! Boundary: CLI prune selection and deletion behavior.

use super::super::*;
use super::fixtures::*;
use crate::test_support::temp_dir;
use canic_backup::persistence::BackupLayout;
use std::fs;

// Ensure prune never deletes failed recovery evidence.
#[test]
fn backup_prune_removes_only_completed_layouts() {
    let root = temp_dir("canic-cli-backup-prune-failed");
    let failed = root.join("deployment-demo-20260511-001234");
    let complete = root.join("deployment-demo-20260511-010000");
    let failed_layout = BackupLayout::new(failed.clone());
    let mut journal = accepted_execution_journal();
    fail_execution_operation(&mut journal, 4, "simulated failure");
    failed_layout
        .write_backup_plan(&valid_backup_plan())
        .expect("write failed plan");
    failed_layout
        .write_execution_journal(&journal)
        .expect("write failed journal");
    write_complete_layout(&complete, "backup-complete", "unix:1778457600");

    let dry_run = backup_prune(&BackupPruneOptions {
        dir: root.clone(),
        keep: 0,
        dry_run: true,
        out: None,
    })
    .expect("dry-run prune");
    assert_eq!(dry_run.scanned, 2);
    assert_eq!(dry_run.selected, 1);
    assert_eq!(dry_run.pruned, 0);
    assert_eq!(dry_run.entries[0].index, 1);
    assert_eq!(dry_run.entries[0].action, BackupPruneAction::WouldRemove);
    assert!(failed.is_dir());
    assert!(complete.is_dir());

    let report = backup_prune(&BackupPruneOptions {
        dir: root.clone(),
        keep: 0,
        dry_run: false,
        out: None,
    })
    .expect("execute prune");

    assert_eq!(report.pruned, 1);
    assert!(failed.is_dir());
    assert!(!complete.is_dir());
    fs::remove_dir_all(root).expect("remove temp root");
}

// Ensure prune reports the maintained backup-list row when retained evidence is interleaved.
#[test]
fn backup_prune_preserves_list_indices_across_failed_layouts() {
    let root = temp_dir("canic-cli-backup-prune-indices");
    let newest_complete = root.join("deployment-demo-20260511-030000");
    let failed = root.join("deployment-demo-20260511-020000");
    let oldest_complete = root.join("deployment-demo-20260511-010000");
    write_complete_layout(&newest_complete, "backup-newest-complete", "unix:30");
    let failed_layout = BackupLayout::new(failed);
    let mut journal = accepted_execution_journal();
    fail_execution_operation(&mut journal, 4, "simulated failure");
    failed_layout
        .write_backup_plan(&valid_backup_plan())
        .expect("write failed plan");
    failed_layout
        .write_execution_journal(&journal)
        .expect("write failed journal");
    write_complete_layout(&oldest_complete, "backup-oldest-complete", "unix:5");

    let report = backup_prune(&BackupPruneOptions {
        dir: root.clone(),
        keep: 1,
        dry_run: true,
        out: None,
    })
    .expect("preview old completed backup");

    assert_eq!(report.scanned, 3);
    assert_eq!(report.entries.len(), 1);
    assert_eq!(report.entries[0].index, 3);
    assert_eq!(report.entries[0].backup_id, "backup-oldest-complete");
    fs::remove_dir_all(root).expect("remove temp root");
}

// Ensure keep-based pruning uses the same newest-first ordering as backup list.
#[test]
fn backup_prune_keep_removes_older_entries() {
    let root = temp_dir("canic-cli-backup-prune-keep");
    let newest = root.join("deployment-demo-20260511-020000");
    let middle = root.join("deployment-demo-20260511-010000");
    let oldest = root.join("deployment-demo-20260511-000000");
    write_complete_layout(&newest, "backup-newest", "unix:1778464800");
    write_complete_layout(&middle, "backup-middle", "unix:1778461200");
    write_complete_layout(&oldest, "backup-oldest", "unix:1778457600");

    let report = backup_prune(&BackupPruneOptions {
        dir: root.clone(),
        keep: 2,
        dry_run: false,
        out: None,
    })
    .expect("prune old backups");

    assert_eq!(report.scanned, 3);
    assert_eq!(report.pruned, 1);
    assert_eq!(report.entries[0].backup_id, "backup-oldest");
    assert!(newest.is_dir());
    assert!(middle.is_dir());
    assert!(!oldest.is_dir());
    fs::remove_dir_all(root).expect("remove temp root");
}

fn write_complete_layout(path: &std::path::Path, backup_id: &str, created_at: &str) {
    let mut journal = accepted_execution_journal();
    for sequence in 4..=9 {
        complete_execution_operation(&mut journal, sequence);
    }
    let layout = BackupLayout::new(path.to_path_buf());
    layout
        .write_backup_plan(&valid_backup_plan())
        .expect("write complete plan");
    layout
        .write_execution_journal(&journal)
        .expect("write complete journal");
    layout
        .publish_manifest(&valid_manifest_with(backup_id, created_at))
        .expect("write complete manifest");
}
