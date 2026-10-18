//! Module: backup::reference::timestamp
//!
//! Responsibility: derive sortable backup list timestamps.
//! Does not own: list scanning or entry classification.
//! Boundary: `unix:<seconds>` timestamp extraction for list entries.

use crate::support::path_stamp::backup_directory_stamp_to_unix;
use canic_backup::{execution::BackupExecutionJournal, timestamp::timestamp_seconds};

pub(super) fn execution_journal_created_at(journal: &BackupExecutionJournal) -> Option<String> {
    journal
        .operations
        .iter()
        .filter_map(|operation| operation.state_updated_at.as_deref())
        .chain(
            journal
                .operation_receipts
                .iter()
                .filter_map(|receipt| receipt.updated_at.as_deref()),
        )
        .filter_map(timestamp_seconds)
        .min()
        .map(|seconds| format!("unix:{seconds}"))
}

pub(super) fn run_id_created_at(run_id: &str) -> Option<String> {
    let mut parts = run_id.rsplit('-');
    let time = parts.next()?;
    let date = parts.next()?;
    backup_directory_stamp_to_unix(&format!("{date}-{time}"))
        .map(|seconds| format!("unix:{seconds}"))
}

pub(super) fn created_at_sort_key(created_at: &str) -> Option<u64> {
    timestamp_seconds(created_at)
}
