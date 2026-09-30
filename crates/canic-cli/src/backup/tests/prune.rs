//! Module: backup::tests::prune
//!
//! Responsibility: backup prune command behavior tests.
//! Does not own: backup listing, status classification, or fixture construction.
//! Boundary: CLI prune selection and deletion behavior.

use super::super::*;
use super::fixtures::*;
use crate::test_support::temp_dir;
use canic_backup::persistence::BackupLayout;
use std::{ffi::OsString, fs};

#[test]
fn backup_prune_keeps_default_and_external_unfinished_restore_journals() {
    for external in [false, true] {
        let root = temp_dir("canic-cli-prune-restore-reference");
        let backup = root.join("backup");
        write_complete_layout(&backup, "backup-test", "unix:30");
        let journal_path = if external {
            root.join("recovery/custom.json")
        } else {
            backup.join("restore-apply-journal.json")
        };
        let mut args = vec![
            OsString::from("prepare"),
            OsString::from("--backup-dir"),
            backup.as_os_str().to_owned(),
            OsString::from("--out"),
            root.join("prepare-report.json").into_os_string(),
        ];
        if external {
            args.extend([
                OsString::from("--journal-out"),
                journal_path.as_os_str().to_owned(),
            ]);
        }
        crate::restore::run(args).expect("prepare pinned journal through public command");
        assert!(journal_path.is_file());
        for dry_run in [true, false] {
            let report =
                backup_prune(&prune_options(&root, 0, dry_run)).expect("prune with restore");
            assert_eq!(report.pruned, 0);
            assert_eq!(report.selected, 0);
            assert_eq!(report.entries[0].action, BackupPruneAction::SkippedRestore);
            assert!(backup.join("artifacts/root").is_file());
        }
        fs::remove_file(journal_path).expect("simulate lost journal location");
        let report = backup_prune(&prune_options(&root, 0, false))
            .expect("retain missing external authority");
        assert_eq!(report.entries[0].action, BackupPruneAction::SkippedRestore);
        fs::remove_dir_all(root).expect("clean fixture");
    }
}

#[test]
fn backup_prune_excludes_active_layout_owners_and_restore_preparation() {
    let root = temp_dir("canic-cli-prune-active-layout");
    let backup = root.join("backup");
    write_complete_layout(&backup, "backup-test", "unix:30");
    let layout = BackupLayout::new(backup.clone());
    let execution = layout.lock_execution().expect("hold active backup");
    let report = backup_prune(&prune_options(&root, 0, false)).expect("skip active owner");
    assert_eq!(report.entries[0].action, BackupPruneAction::SkippedBusy);
    assert!(backup.is_dir());
    drop(execution);
    let report = super::super::prune::backup_prune_with_remove(
        &prune_options(&root, 0, false),
        &mut |path| {
            std::assert_matches!(
                BackupLayout::new(path.to_path_buf()).lock_lifetime(),
                Err(canic_backup::persistence::JournalLockError::Locked { .. })
            );
            fs::remove_dir_all(path)
        },
    )
    .expect("prune owns preparation exclusion");
    assert_eq!(report.pruned, 1);
    std::assert_matches!(layout.lock_lifetime(), Err(canic_backup::persistence::JournalLockError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound);
    fs::remove_dir_all(root).expect("clean fixture");
}

#[test]
fn backup_prune_retention_counts_only_verified_backups() {
    let root = temp_dir("canic-cli-prune-corrupt-newest");
    let newest = root.join("newest");
    let middle = root.join("middle");
    let oldest = root.join("oldest");
    write_complete_layout(&newest, "backup-newest", "unix:30");
    write_complete_layout(&middle, "backup-middle", "unix:20");
    write_complete_layout(&oldest, "backup-oldest", "unix:10");
    fs::write(newest.join("artifacts/root"), b"corrupted").expect("corrupt newest copy");
    let report = backup_prune(&prune_options(&root, 1, false)).expect("retain newest valid copy");
    assert_eq!(report.entries[0].action, BackupPruneAction::SkippedInvalid);
    assert_eq!(report.entries[1].action, BackupPruneAction::Removed);
    assert_eq!(report.pruned, 1);
    assert!(newest.is_dir());
    assert!(middle.is_dir());
    assert!(!oldest.exists());
    fs::remove_dir_all(root).expect("clean fixture");
}

#[test]
fn backup_prune_retained_copies_are_protected_from_a_competing_prune() {
    let root = temp_dir("canic-cli-prune-retained-locks");
    let newest = root.join("newest");
    let oldest = root.join("oldest");
    write_complete_layout(&newest, "backup-newest", "unix:30");
    write_complete_layout(&oldest, "backup-oldest", "unix:10");
    let report = super::super::prune::backup_prune_with_remove(
        &prune_options(&root, 1, false),
        &mut |path| {
            let competing = backup_prune(&prune_options(&root, 0, false)).expect("competing prune");
            assert_eq!(competing.pruned, 0);
            assert!(
                competing
                    .entries
                    .iter()
                    .all(|entry| entry.action == BackupPruneAction::SkippedBusy)
            );
            fs::remove_dir_all(path)
        },
    )
    .expect("prune retains its protected newest copy");
    assert_eq!(report.pruned, 1);
    assert!(newest.is_dir());
    assert!(!oldest.exists());
    fs::remove_dir_all(root).expect("clean fixture");
}

#[test]
fn backup_prune_reports_deletions_before_a_partial_failure() {
    let root = temp_dir("canic-cli-prune-partial-failure");
    let first = root.join("first");
    let second = root.join("second");
    write_complete_layout(&first, "backup-first", "unix:30");
    write_complete_layout(&second, "backup-second", "unix:20");
    let report = super::super::prune::backup_prune_with_remove(
        &prune_options(&root, 0, false),
        &mut |path| {
            if path == second {
                fs::remove_file(path.join("artifacts/root"))?;
                Err(std::io::ErrorKind::PermissionDenied.into())
            } else {
                fs::remove_dir_all(path)
            }
        },
    )
    .expect("retain per-directory outcomes");
    assert_eq!(report.selected, 2);
    assert_eq!(report.pruned, 1);
    assert_eq!(report.entries[0].action, BackupPruneAction::Removed);
    assert_eq!(report.entries[1].action, BackupPruneAction::Failed);
    assert!(report.entries[1].detail.is_some());
    assert!(!first.exists());
    assert!(second.is_dir());
    fs::remove_dir_all(root).expect("clean fixture");
}

#[cfg(unix)]
#[test]
fn backup_prune_rejects_linked_backup_entries() {
    let root = temp_dir("canic-cli-prune-linked-root");
    let outside = temp_dir("canic-cli-prune-external-layout");
    write_complete_layout(&outside, "backup-test", "unix:30");
    fs::create_dir_all(&root).expect("create scan root");
    std::os::unix::fs::symlink(&outside, root.join("linked")).expect("link external layout");
    let report = backup_prune(&prune_options(&root, 0, false)).expect("skip link");
    assert_eq!(report.entries[0].action, BackupPruneAction::SkippedInvalid);
    assert_eq!(report.pruned, 0);
    assert!(outside.join("artifacts/root").is_file());
    fs::remove_dir_all(root).expect("clean fixture");
    fs::remove_dir_all(outside).expect("clean external fixture");
}

fn prune_options(root: &std::path::Path, keep: usize, dry_run: bool) -> BackupPruneOptions {
    BackupPruneOptions {
        dir: root.to_path_buf(),
        keep,
        dry_run,
        out: None,
    }
}

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
    let checksum = write_artifact(path, b"restorable snapshot");
    let mut manifest = valid_manifest_with(backup_id, created_at);
    manifest.deployment.members[0].source_snapshot.checksum = Some(checksum.hash.clone());
    let mut download = journal_with_checksum(checksum.hash);
    download.backup_id = backup_id.to_string();
    layout
        .write_journal(&download)
        .expect("write durable downloads");
    layout
        .write_backup_plan(&valid_backup_plan())
        .expect("write complete plan");
    layout
        .write_execution_journal(&journal)
        .expect("write complete journal");
    layout
        .publish_manifest(&manifest)
        .expect("write complete manifest");
}
