//! Module: backup::reference::resolve
//!
//! Responsibility: resolve backup references into filesystem paths.
//! Does not own: list entry construction or output rendering.
//! Boundary: row-number and backup-id selectors for backup commands.

use super::list::backup_list;
use crate::backup::{BackupCommandError, BackupListOptions};
use std::path::{Path, PathBuf};

pub(in crate::backup) fn resolve_backup_dir(
    dir: Option<&Path>,
    backup_ref: Option<&str>,
) -> Result<PathBuf, BackupCommandError> {
    if let Some(dir) = dir {
        return Ok(dir.to_path_buf());
    }
    if let Some(backup_ref) = backup_ref {
        return resolve_backup_reference(backup_ref);
    }

    Err(BackupCommandError::Usage(
        "backup target required; pass <backup-ref> or --dir <dir>".to_string(),
    ))
}

pub fn resolve_backup_reference(reference: &str) -> Result<PathBuf, BackupCommandError> {
    resolve_backup_reference_in(Path::new("backups"), reference)
}

pub(in crate::backup) fn resolve_backup_reference_in(
    root: &Path,
    reference: &str,
) -> Result<PathBuf, BackupCommandError> {
    let entries = backup_list(&BackupListOptions {
        dir: root.to_path_buf(),
        out: None,
    })?;

    if reference.bytes().all(|byte| byte.is_ascii_digit()) {
        let index = reference.parse::<usize>().unwrap_or(0);
        return entries
            .get(index.saturating_sub(1))
            .map(|entry| entry.dir.clone())
            .ok_or_else(|| BackupCommandError::BackupReferenceNotFound {
                reference: reference.to_string(),
            });
    }

    let mut matches = entries
        .into_iter()
        .filter(|entry| entry.backup_id == reference)
        .map(|entry| entry.dir)
        .collect::<Vec<_>>();
    match matches.len() {
        0 => Err(BackupCommandError::BackupReferenceNotFound {
            reference: reference.to_string(),
        }),
        1 => Ok(matches.remove(0)),
        _ => Err(BackupCommandError::BackupReferenceAmbiguous {
            reference: reference.to_string(),
        }),
    }
}
