//! Module: backup::render::list
//!
//! Responsibility: render backup list reports.
//! Does not own: backup discovery, reference resolution, or output writing.
//! Boundary: table formatting and display timestamp conversion for backup list rows.

use super::super::BackupListEntry;
use crate::support::path_stamp::backup_list_timestamp;
use canic_host::table::{ColumnAlign, render_table};

pub(super) fn render_backup_list(entries: &[BackupListEntry]) -> String {
    let rows = entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            [
                (index + 1).to_string(),
                entry.dir.display().to_string(),
                entry.backup_id.clone(),
                display_created_at(&entry.created_at),
                entry.members.to_string(),
                entry.status.label().to_string(),
            ]
        })
        .collect::<Vec<_>>();
    render_table(
        &["#", "DIR", "BACKUP_ID", "CREATED_AT", "MEMBERS", "STATUS"],
        &rows,
        &[
            ColumnAlign::Right,
            ColumnAlign::Left,
            ColumnAlign::Left,
            ColumnAlign::Left,
            ColumnAlign::Left,
            ColumnAlign::Left,
        ],
    )
}

fn display_created_at(created_at: &str) -> String {
    created_at
        .strip_prefix("unix:")
        .and_then(|seconds| seconds.parse::<u64>().ok())
        .map_or_else(|| created_at.to_string(), backup_list_timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // Ensure backup list text output includes table headers and display timestamps.
    #[test]
    fn render_backup_list_formats_unix_created_at() {
        let entries = vec![BackupListEntry {
            dir: PathBuf::from("backups/deployment-demo-20240507-140000"),
            backup_id: "backup".to_string(),
            created_at: "unix:1715090400".to_string(),
            members: 7,
            status: crate::backup::BackupListStatus::Ok,
        }];

        let rendered = render_backup_list(&entries);

        assert!(rendered.contains('#'));
        assert!(rendered.contains("DIR"));
        assert!(rendered.contains("07/05/2024 14:00"));
        assert!(!rendered.contains("unix:"));
    }
}
