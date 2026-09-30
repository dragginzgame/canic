//! Module: persistence::layout_lifetime::tests
//!
//! Responsibility: qualify stable lock identity and durable restore references.
//! Does not own: restore execution or CLI retention policy.
//! Boundary: real host filesystem exclusion, including removed and recreated roots.

use super::*;
use crate::test_support::temp_dir;

#[test]
fn removed_and_recreated_layout_keeps_the_same_lock_identity() {
    let parent = temp_dir("canic-layout-lifetime-recreate");
    let root = parent.join("backup");
    fs::create_dir_all(&root).expect("create backup");
    let guard = BackupLayoutGuard::acquire(&root).expect("own deletion");
    fs::remove_dir_all(&root).expect("remove backup");
    fs::create_dir(&root).expect("recreate backup");
    std::assert_matches!(
        BackupLayoutGuard::acquire(&root),
        Err(JournalLockError::Locked { .. })
    );
    drop(guard);
    let guard = BackupLayoutGuard::acquire(&root).expect("same sidecar released");
    assert_eq!(guard.root(), root);
    fs::remove_dir_all(parent).expect("clean fixture");
}

#[test]
fn reference_release_requires_exact_restore_authority() {
    let parent = temp_dir("canic-layout-lifetime-authority");
    let root = parent.join("backup");
    fs::create_dir_all(&root).expect("create backup");
    let guard = BackupLayoutGuard::acquire(&root).expect("lock backup");
    let journal = parent.join("custom.json");
    guard
        .retain_restore(&journal, "original")
        .expect("retain restore");
    std::assert_matches!(
        guard.release_restore(&journal, "different"),
        Err(PersistenceError::RestoreReferenceConflict { .. })
    );
    std::assert_matches!(
        guard.retain_restore(&journal, "different"),
        Err(PersistenceError::RestoreReferenceConflict { .. })
    );
    assert!(
        guard
            .has_restore_references()
            .expect("read retained authority")
    );
    drop(guard);
    let guard = BackupLayoutGuard::acquire(&root).expect("reopen backup");
    assert!(
        guard
            .has_restore_references()
            .expect("reference survived reopen")
    );
    guard
        .release_restore(&journal, "original")
        .expect("complete original restore");
    assert!(!guard.has_restore_references().expect("reference released"));
    fs::remove_dir_all(parent).expect("clean fixture");
}

#[cfg(unix)]
#[test]
fn unsafe_reference_entries_fail_closed_without_following_links() {
    let parent = temp_dir("canic-layout-lifetime-unsafe-reference");
    let root = parent.join("backup");
    fs::create_dir_all(&root).expect("create backup");
    let guard = BackupLayoutGuard::acquire(&root).expect("lock backup");
    let outside = parent.join("outside.json");
    fs::write(&outside, br#"{"version":1,"restores":[]}"#).expect("write outside document");
    std::os::unix::fs::symlink(&outside, root.join(REFERENCES_FILE)).expect("link reference file");
    std::assert_matches!(
        guard.has_restore_references(),
        Err(PersistenceError::InvalidRestoreReferences { .. })
    );
    fs::remove_dir_all(parent).expect("clean fixture");
}
