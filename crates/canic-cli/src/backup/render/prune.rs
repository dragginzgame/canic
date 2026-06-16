//! Module: backup::render::prune
//!
//! Responsibility: render backup prune reports.
//! Does not own: prune selection, filesystem deletion, or output writing.
//! Boundary: table formatting for prune summary and selected entries.

use super::super::BackupPruneReport;
use canic_host::table::{ColumnAlign, render_table};

pub(super) fn render_prune_report(report: &BackupPruneReport) -> String {
    let summary_rows = [[
        if report.dry_run { "dry-run" } else { "execute" }.to_string(),
        report.scanned.to_string(),
        report.selected.to_string(),
        report.pruned.to_string(),
    ]];
    let entry_rows = report
        .entries
        .iter()
        .map(|entry| {
            [
                entry.index.to_string(),
                entry.dir.display().to_string(),
                entry.backup_id.clone(),
                entry.status.clone(),
                entry.action.clone(),
            ]
        })
        .collect::<Vec<_>>();

    [
        render_table(
            &["MODE", "SCANNED", "SELECTED", "PRUNED"],
            &summary_rows,
            &[ColumnAlign::Left; 4],
        ),
        String::new(),
        render_table(
            &["#", "DIR", "BACKUP_ID", "STATUS", "ACTION"],
            &entry_rows,
            &[
                ColumnAlign::Right,
                ColumnAlign::Left,
                ColumnAlign::Left,
                ColumnAlign::Left,
                ColumnAlign::Left,
            ],
        ),
    ]
    .join("\n")
}
