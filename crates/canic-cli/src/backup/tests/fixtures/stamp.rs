//! Module: backup::tests::fixtures::stamp
//!
//! Responsibility: convert backup directory stamps for backup tests.
//! Does not own: production path stamping or backup-list timestamp formatting.
//! Boundary: test helper for expected `unix:` marker values.

use crate::support::path_stamp::backup_directory_stamp_to_unix;

pub(in crate::backup::tests) fn unix_marker_for_stamp(stamp: &str) -> String {
    format!(
        "unix:{}",
        backup_directory_stamp_to_unix(stamp).expect("valid backup directory stamp")
    )
}
