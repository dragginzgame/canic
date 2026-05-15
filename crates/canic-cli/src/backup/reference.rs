use super::{BackupCommandError, BackupListEntry, BackupListOptions};
use crate::backup::labels::execution_layout_status;
use canic_backup::persistence::BackupLayout;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(super) fn backup_list(
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
        right
            .created_at
            .cmp(&left.created_at)
            .then_with(|| right.dir.cmp(&left.dir))
    });
    Ok(entries)
}

pub(super) fn resolve_backup_dir(
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

fn resolve_backup_reference(reference: &str) -> Result<PathBuf, BackupCommandError> {
    resolve_backup_reference_in(Path::new("backups"), reference)
}

pub(super) fn resolve_backup_reference_in(
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

fn backup_list_entry(dir: PathBuf) -> Option<BackupListEntry> {
    let layout = BackupLayout::new(dir.clone());
    if layout.manifest_path().is_file() {
        return Some(manifest_backup_list_entry(dir, &layout));
    }
    if layout.backup_plan_path().is_file() {
        return Some(planned_backup_list_entry(dir, &layout));
    }

    None
}

fn manifest_backup_list_entry(dir: PathBuf, layout: &BackupLayout) -> BackupListEntry {
    let Ok(manifest) = layout.read_manifest() else {
        return BackupListEntry {
            dir,
            backup_id: "-".to_string(),
            created_at: "-".to_string(),
            members: 0,
            status: "invalid-manifest".to_string(),
        };
    };
    let status = if layout.backup_plan_path().is_file() {
        execution_backed_layout_status(layout)
    } else {
        "ok".to_string()
    };

    BackupListEntry {
        dir,
        backup_id: manifest.backup_id,
        created_at: manifest.created_at,
        members: manifest.fleet.members.len(),
        status,
    }
}

fn planned_backup_list_entry(dir: PathBuf, layout: &BackupLayout) -> BackupListEntry {
    let Ok(plan) = layout.read_backup_plan() else {
        return BackupListEntry {
            dir,
            backup_id: "-".to_string(),
            created_at: "-".to_string(),
            members: 0,
            status: "invalid-plan".to_string(),
        };
    };
    let status = execution_backed_layout_status(layout);

    BackupListEntry {
        dir,
        backup_id: plan.plan_id,
        created_at: planned_backup_created_at(&plan.run_id),
        members: plan.targets.len(),
        status,
    }
}

fn execution_backed_layout_status(layout: &BackupLayout) -> String {
    if layout.read_backup_plan().is_err() {
        return "invalid-plan".to_string();
    }
    if layout.execution_journal_path().is_file() && layout.verify_execution_integrity().is_err() {
        return "invalid-plan-journal".to_string();
    }
    if !layout.execution_journal_path().is_file() && layout.manifest_path().is_file() {
        return "invalid-plan-journal".to_string();
    }
    if let Ok(journal) = layout.read_execution_journal() {
        execution_layout_status(&journal, layout.manifest_path().is_file())
    } else {
        "dry-run".to_string()
    }
}

fn planned_backup_created_at(run_id: &str) -> String {
    let mut parts = run_id.rsplit('-');
    let Some(time) = parts.next() else {
        return "-".to_string();
    };
    let Some(date) = parts.next() else {
        return "-".to_string();
    };
    let valid = date.len() == 8
        && time.len() == 6
        && date.bytes().all(|byte| byte.is_ascii_digit())
        && time.bytes().all(|byte| byte.is_ascii_digit());
    if valid {
        format!("{date}-{time}")
    } else {
        "-".to_string()
    }
}
