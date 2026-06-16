//! Module: backup::reference::list
//!
//! Responsibility: scan backup roots and sort list entries.
//! Does not own: layout-specific entry classification.
//! Boundary: backup root filesystem traversal for `canic backup list`.

use super::{entry::backup_list_entry, timestamp::created_at_sort_key};
use crate::backup::{BackupCommandError, BackupListEntry, BackupListOptions};
use std::fs;

pub(in crate::backup) fn backup_list(
    options: &BackupListOptions,
) -> Result<Vec<BackupListEntry>, BackupCommandError> {
    if !options.dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut entries = fs::read_dir(&options.dir)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|path| path.is_dir())
        .filter_map(backup_list_entry)
        .collect::<Vec<_>>();

    entries.sort_by(|left, right| {
        created_at_sort_key(&right.created_at)
            .cmp(&created_at_sort_key(&left.created_at))
            .then_with(|| right.created_at.cmp(&left.created_at))
            .then_with(|| right.dir.cmp(&left.dir))
    });
    Ok(entries)
}
