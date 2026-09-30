//! Module: backup::prune
//!
//! Responsibility: select and remove verified, unreferenced backup layouts.
//! Does not own: restore progress, artifact verification rules, or report rendering.
//! Boundary: holds layout lifetime authority through retention decisions and deletion.

use crate::backup::{
    BackupCommandError, BackupListOptions, BackupListStatus, BackupPruneAction, BackupPruneEntry,
    BackupPruneOptions, BackupPruneReport,
    reference::{backup_list, backup_list_entry},
};

use std::{fs, io, path::Path};

use canic_backup::persistence::{BackupLayout, BackupLayoutGuard, JournalLockError};

pub(super) fn backup_prune(
    options: &BackupPruneOptions,
) -> Result<BackupPruneReport, BackupCommandError> {
    backup_prune_with_remove(options, &mut |path| fs::remove_dir_all(path))
}

pub(super) fn backup_prune_with_remove(
    options: &BackupPruneOptions,
    remove: &mut impl FnMut(&Path) -> io::Result<()>,
) -> Result<BackupPruneReport, BackupCommandError> {
    let entries = backup_list(&BackupListOptions {
        dir: options.dir.clone(),
        out: None,
    })?;
    let mut report = BackupPruneReport {
        dry_run: options.dry_run,
        scanned: entries.len(),
        selected: 0,
        pruned: 0,
        entries: Vec::new(),
    };
    // Keep retained copies locked until all deletion decisions finish. A competing
    // prune must not remove the copies on which this invocation's retention relies.
    let mut retained = Vec::new();
    for (index, entry) in entries.into_iter().enumerate() {
        if entry.status != BackupListStatus::Complete {
            continue;
        }
        let mut result = BackupPruneEntry {
            index: index + 1,
            dir: entry.dir.clone(),
            backup_id: entry.backup_id,
            status: entry.status,
            action: BackupPruneAction::SkippedInvalid,
            detail: None,
        };
        let guard = match inspect_candidate(&entry.dir) {
            Ok(guard) => guard,
            Err((action, detail)) => {
                result.action = action;
                result.detail = Some(detail);
                report.entries.push(result);
                continue;
            }
        };
        if retained.len() < options.keep {
            retained.push(guard);
            continue;
        }
        match guard.has_restore_references() {
            Ok(false) => {}
            Ok(true) => {
                result.action = BackupPruneAction::SkippedRestore;
                report.entries.push(result);
                continue;
            }
            Err(error) => {
                result.detail = Some(error.to_string());
                report.entries.push(result);
                continue;
            }
        }
        report.selected += 1;
        if options.dry_run {
            result.action = BackupPruneAction::WouldRemove;
        } else {
            match remove(guard.root()) {
                Ok(()) => {
                    result.action = BackupPruneAction::Removed;
                    report.pruned += 1;
                }
                Err(error) => {
                    result.action = BackupPruneAction::Failed;
                    result.detail = Some(error.to_string());
                }
            }
        }
        report.entries.push(result);
    }
    Ok(report)
}

fn inspect_candidate(path: &Path) -> Result<BackupLayoutGuard, (BackupPruneAction, String)> {
    let invalid = |error: String| (BackupPruneAction::SkippedInvalid, error);
    let metadata = fs::symlink_metadata(path).map_err(|error| invalid(error.to_string()))?;
    if !metadata.is_dir() {
        return Err(invalid(
            "backup entry must be a direct directory".to_string(),
        ));
    }
    let layout = BackupLayout::new(path.to_path_buf());
    let guard = layout.lock_lifetime().map_err(|error| match error {
        JournalLockError::Locked { lock_path } => (BackupPruneAction::SkippedBusy, lock_path),
        error => invalid(format!("{error:?}")),
    })?;
    let current = backup_list_entry(guard.root().to_path_buf());
    if !current.is_some_and(|entry| entry.status == BackupListStatus::Complete) {
        return Err(invalid("backup is no longer complete".to_string()));
    }
    layout
        .verify_integrity()
        .map_err(|error| invalid(error.to_string()))?;
    Ok(guard)
}
