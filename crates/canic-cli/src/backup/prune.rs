use super::{
    BackupCommandError, BackupListOptions, BackupListStatus, BackupPruneAction, BackupPruneEntry,
    BackupPruneOptions, BackupPruneReport, reference::backup_list,
};
use std::fs;

pub(super) fn backup_prune(
    options: &BackupPruneOptions,
) -> Result<BackupPruneReport, BackupCommandError> {
    let entries = backup_list(&BackupListOptions {
        dir: options.dir.clone(),
        out: None,
    })?;
    let selected = entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry.status == BackupListStatus::Complete)
        .skip(options.keep)
        .map(|(index, entry)| BackupPruneEntry {
            index: index + 1,
            dir: entry.dir.clone(),
            backup_id: entry.backup_id.clone(),
            status: entry.status,
            action: if options.dry_run {
                BackupPruneAction::WouldRemove
            } else {
                BackupPruneAction::Removed
            },
        })
        .collect::<Vec<_>>();

    if !options.dry_run {
        for entry in &selected {
            fs::remove_dir_all(&entry.dir)?;
        }
    }

    Ok(BackupPruneReport {
        dry_run: options.dry_run,
        scanned: entries.len(),
        selected: selected.len(),
        pruned: if options.dry_run { 0 } else { selected.len() },
        entries: selected,
    })
}
