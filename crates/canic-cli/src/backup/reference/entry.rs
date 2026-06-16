//! Module: backup::reference::entry
//!
//! Responsibility: classify persisted backup layouts into list entries.
//! Does not own: backup root scanning or reference lookup.
//! Boundary: manifest, dry-run plan, and execution-backed list row fields.

use super::timestamp::{execution_journal_created_at, run_id_created_at};
use crate::backup::{BackupListEntry, labels::execution_layout_status};
use canic_backup::persistence::BackupLayout;
use std::path::PathBuf;

pub(super) fn backup_list_entry(dir: PathBuf) -> Option<BackupListEntry> {
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
        members: manifest.deployment.members.len(),
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
    let created_at = layout
        .read_execution_journal()
        .ok()
        .and_then(|journal| execution_journal_created_at(&journal))
        .or_else(|| run_id_created_at(&plan.run_id))
        .unwrap_or_else(|| "-".to_string());

    BackupListEntry {
        dir,
        backup_id: plan.plan_id,
        created_at,
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
