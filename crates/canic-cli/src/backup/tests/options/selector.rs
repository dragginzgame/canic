//! Module: backup::tests::options::selector
//!
//! Responsibility: backup selector validation tests.
//! Does not own: option parser success paths.
//! Boundary: invalid reference/path selector combinations.

use super::super::super::*;
use std::ffi::OsString;

// Ensure commands require one backup selector path, either by reference or explicit dir.
#[test]
fn backup_target_options_reject_missing_or_duplicate_selectors() {
    let missing = BackupInspectOptions::parse([]).expect_err("missing selector rejects");
    std::assert_matches!(missing, BackupCommandError::Usage(_));

    let duplicate = BackupInspectOptions::parse([
        OsString::from("1"),
        OsString::from("--dir"),
        OsString::from("backups/run"),
    ])
    .expect_err("duplicate selector rejects");
    std::assert_matches!(duplicate, BackupCommandError::Usage(_));

    let invalid_keep =
        BackupPruneOptions::parse([OsString::from("--keep"), OsString::from("banana")])
            .expect_err("invalid keep rejects");
    std::assert_matches!(invalid_keep, BackupCommandError::Usage(_));

    let missing_prune_selector =
        BackupPruneOptions::parse([]).expect_err("missing prune selector rejects");
    std::assert_matches!(missing_prune_selector, BackupCommandError::Usage(_));
}
