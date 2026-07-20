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

// Ensure prune previews and removes only explicitly selected failed backup directories.
#[test]
fn backup_prune_removes_failed_layouts() {
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
    BackupLayout::new(complete.clone())
        .publish_manifest(&valid_manifest_with("backup-complete", "unix:1778457600"))
        .expect("write complete manifest");

    let dry_run = backup_prune(&BackupPruneOptions {
        dir: root.clone(),
        failed: true,
        keep: None,
        dry_run: true,
        out: None,
    })
    .expect("dry-run prune");
    assert_eq!(dry_run.scanned, 2);
    assert_eq!(dry_run.selected, 1);
    assert_eq!(dry_run.pruned, 0);
    assert_eq!(dry_run.entries[0].action, BackupPruneAction::WouldRemove);
    assert!(failed.is_dir());

    let report = backup_prune(&BackupPruneOptions {
        dir: root.clone(),
        failed: true,
        keep: None,
        dry_run: false,
        out: None,
    })
    .expect("execute prune");

    assert_eq!(report.pruned, 1);
    assert!(!failed.is_dir());
    assert!(complete.is_dir());
    fs::remove_dir_all(root).expect("remove temp root");
}

// Ensure keep-based pruning uses the same newest-first ordering as backup list.
#[test]
fn backup_prune_keep_removes_older_entries() {
    let root = temp_dir("canic-cli-backup-prune-keep");
    let newest = root.join("deployment-demo-20260511-020000");
    let middle = root.join("deployment-demo-20260511-010000");
    let oldest = root.join("deployment-demo-20260511-000000");
    BackupLayout::new(newest.clone())
        .publish_manifest(&valid_manifest_with("backup-newest", "unix:1778464800"))
        .expect("write newest manifest");
    BackupLayout::new(middle.clone())
        .publish_manifest(&valid_manifest_with("backup-middle", "unix:1778461200"))
        .expect("write middle manifest");
    BackupLayout::new(oldest.clone())
        .publish_manifest(&valid_manifest_with("backup-oldest", "unix:1778457600"))
        .expect("write oldest manifest");

    let report = backup_prune(&BackupPruneOptions {
        dir: root.clone(),
        failed: false,
        keep: Some(2),
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
