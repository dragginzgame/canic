use super::{
    BackupCommandError, BackupListOptions, BackupPruneEntry, BackupPruneOptions, BackupPruneReport,
    reference::backup_list,
};
use std::fs;

pub(super) fn backup_prune(
    options: &BackupPruneOptions,
) -> Result<BackupPruneReport, BackupCommandError> {
    if !options.failed && options.keep.is_none() {
        return Err(BackupCommandError::Usage(
            "backup prune requires --failed or --keep <count>".to_string(),
        ));
    }

    let entries = backup_list(&BackupListOptions {
        dir: options.dir.clone(),
        out: None,
    })?;
    let selected = entries
        .iter()
        .enumerate()
        .filter(|(index, entry)| {
            (options.failed && entry.status == "failed")
                || options.keep.is_some_and(|keep| *index >= keep)
        })
        .map(|(index, entry)| BackupPruneEntry {
            index: index + 1,
            dir: entry.dir.clone(),
            backup_id: entry.backup_id.clone(),
            status: entry.status.clone(),
            action: if options.dry_run {
                "would-remove".to_string()
            } else {
                "removed".to_string()
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
