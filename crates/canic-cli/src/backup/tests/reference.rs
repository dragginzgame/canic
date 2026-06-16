//! Module: backup::tests::reference
//!
//! Responsibility: backup reference resolution behavior tests.
//! Does not own: backup listing or persistence fixtures.
//! Boundary: CLI backup-ref row/id lookup and ambiguity behavior.

use super::super::*;
use super::fixtures::*;
use crate::test_support::temp_dir;
use canic_backup::persistence::BackupLayout;
use std::fs;

// Ensure short backup references resolve through the same ordering as backup list.
#[test]
fn backup_reference_resolves_rows_and_backup_ids() {
    let root = temp_dir("canic-cli-backup-reference");
    let first = root.join("deployment-demo-20260507-120000");
    let second = root.join("deployment-demo-20260507-130000");

    BackupLayout::new(first.clone())
        .write_manifest(&valid_manifest_with("backup-old", "2026-05-07T12:00:00Z"))
        .expect("write first manifest");
    BackupLayout::new(second.clone())
        .write_manifest(&valid_manifest_with("backup-new", "2026-05-07T13:00:00Z"))
        .expect("write second manifest");

    let by_row = resolve_backup_reference_in(&root, "1").expect("resolve row");
    let by_id = resolve_backup_reference_in(&root, "backup-old").expect("resolve id");
    let missing = resolve_backup_reference_in(&root, "99").expect_err("missing row rejects");

    fs::remove_dir_all(root).expect("remove temp root");
    assert_eq!(by_row, second);
    assert_eq!(by_id, first);
    std::assert_matches!(missing, BackupCommandError::BackupReferenceNotFound { .. });
}

// Ensure duplicate backup ids fail closed instead of resolving arbitrarily.
#[test]
fn backup_reference_rejects_ambiguous_backup_ids() {
    let root = temp_dir("canic-cli-backup-reference-ambiguous");
    let first = root.join("deployment-demo-20260507-120000");
    let second = root.join("deployment-demo-20260507-130000");

    BackupLayout::new(first)
        .write_manifest(&valid_manifest_with("backup-same", "2026-05-07T12:00:00Z"))
        .expect("write first manifest");
    BackupLayout::new(second)
        .write_manifest(&valid_manifest_with("backup-same", "2026-05-07T13:00:00Z"))
        .expect("write second manifest");

    let err = resolve_backup_reference_in(&root, "backup-same").expect_err("ambiguous rejects");

    fs::remove_dir_all(root).expect("remove temp root");
    std::assert_matches!(err, BackupCommandError::BackupReferenceAmbiguous { .. });
}
