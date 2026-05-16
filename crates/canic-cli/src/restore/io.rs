use crate::{backup::resolve_backup_reference, output};
use canic_backup::{
    manifest::FleetBackupManifest,
    persistence::BackupLayout,
    restore::{
        RestoreApplyDryRun, RestoreApplyJournal, RestoreMapping, RestorePlan, RestoreRunResponse,
    },
};
use serde::Serialize;
use std::path::{Path, PathBuf};

use super::{
    RestoreApplyOptions, RestoreCommandError, RestorePlanOptions, RestorePrepareOptions,
    RestoreRunOptions, RestoreStatusOptions,
};

pub(super) const RESTORE_PLAN_FILE: &str = "restore-plan.json";
pub(super) const RESTORE_APPLY_JOURNAL_FILE: &str = "restore-apply-journal.json";

///
/// RestorePrepareReport
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct RestorePrepareReport {
    pub backup_dir: String,
    pub plan_path: String,
    pub journal_path: String,
    pub backup_id: String,
    pub ready: bool,
    pub readiness_reasons: Vec<String>,
    pub members: usize,
    pub operations: usize,
}

// Verify backup layout integrity before restore planning when requested.
pub(super) fn verify_backup_layout_if_required(
    options: &RestorePlanOptions,
) -> Result<(), RestoreCommandError> {
    if !options.require_verified {
        return Ok(());
    }

    let Some(dir) = restore_plan_backup_dir(options)? else {
        return Err(RestoreCommandError::RequireVerifiedNeedsBackupDir);
    };

    BackupLayout::new(dir).verify_integrity()?;
    Ok(())
}

// Read the manifest from a direct path or canonical backup layout.
pub(super) fn read_manifest_source(
    options: &RestorePlanOptions,
) -> Result<FleetBackupManifest, RestoreCommandError> {
    if let Some(path) = &options.manifest {
        return read_manifest(path);
    }

    let Some(dir) = restore_plan_backup_dir(options)? else {
        return Err(RestoreCommandError::MissingOption(
            "--manifest or --backup-dir",
        ));
    };

    BackupLayout::new(dir)
        .read_manifest()
        .map_err(RestoreCommandError::from)
}

pub(super) fn restore_plan_backup_dir(
    options: &RestorePlanOptions,
) -> Result<Option<PathBuf>, RestoreCommandError> {
    restore_backup_dir(options.backup_ref.as_deref(), options.backup_dir.as_deref())
}

pub(super) fn restore_prepare_backup_dir(
    options: &RestorePrepareOptions,
) -> Result<PathBuf, RestoreCommandError> {
    restore_backup_dir(options.backup_ref.as_deref(), options.backup_dir.as_deref())?.ok_or(
        RestoreCommandError::MissingOption("backup-ref or --backup-dir"),
    )
}

pub(super) fn restore_apply_plan_path(
    options: &RestoreApplyOptions,
) -> Result<PathBuf, RestoreCommandError> {
    if let Some(plan) = &options.plan {
        return Ok(plan.clone());
    }
    let backup_dir = restore_backup_dir(options.backup_ref.as_deref(), None)?
        .ok_or(RestoreCommandError::MissingOption("backup-ref or --plan"))?;
    require_prepared_plan_path(
        options.backup_ref.as_deref().unwrap_or_default(),
        default_restore_plan_path(&backup_dir),
    )
}

pub(super) fn restore_apply_backup_dir(
    options: &RestoreApplyOptions,
) -> Result<Option<PathBuf>, RestoreCommandError> {
    if let Some(backup_dir) = &options.backup_dir {
        return Ok(Some(backup_dir.clone()));
    }
    restore_backup_dir(options.backup_ref.as_deref(), None)
}

pub(super) fn restore_run_journal_path(
    options: &RestoreRunOptions,
) -> Result<PathBuf, RestoreCommandError> {
    restore_journal_path(options.backup_ref.as_deref(), options.journal.as_deref())
}

pub(super) fn restore_status_journal_path(
    options: &RestoreStatusOptions,
) -> Result<PathBuf, RestoreCommandError> {
    restore_journal_path(options.backup_ref.as_deref(), options.journal.as_deref())
}

pub(super) fn verify_prepared_journal_backup_root(
    backup_ref: Option<&str>,
    journal_path: &Path,
) -> Result<(), RestoreCommandError> {
    let Some(backup_ref) = backup_ref else {
        return Ok(());
    };
    let Some(backup_dir) = restore_backup_dir(Some(backup_ref), None)? else {
        return Err(RestoreCommandError::MissingOption("backup-ref"));
    };

    verify_selected_journal_backup_root(backup_ref, &backup_dir, journal_path)
}

pub(super) fn default_restore_plan_path(backup_dir: &Path) -> PathBuf {
    backup_dir.join(RESTORE_PLAN_FILE)
}

pub(super) fn default_restore_apply_journal_path(backup_dir: &Path) -> PathBuf {
    backup_dir.join(RESTORE_APPLY_JOURNAL_FILE)
}

fn restore_journal_path(
    backup_ref: Option<&str>,
    journal: Option<&Path>,
) -> Result<PathBuf, RestoreCommandError> {
    if let Some(journal) = journal {
        return Ok(journal.to_path_buf());
    }
    let backup_dir = restore_backup_dir(backup_ref, None)?.ok_or(
        RestoreCommandError::MissingOption("backup-ref or --journal"),
    )?;
    require_prepared_journal_path(
        backup_ref.unwrap_or_default(),
        default_restore_apply_journal_path(&backup_dir),
    )
}

pub(super) fn require_prepared_plan_path(
    backup_ref: &str,
    path: PathBuf,
) -> Result<PathBuf, RestoreCommandError> {
    if !path.is_file() {
        return Err(RestoreCommandError::PreparedPlanMissing {
            backup_ref: backup_ref.to_string(),
            path: path.display().to_string(),
        });
    }

    Ok(path)
}

pub(super) fn require_prepared_journal_path(
    backup_ref: &str,
    path: PathBuf,
) -> Result<PathBuf, RestoreCommandError> {
    if !path.is_file() {
        return Err(RestoreCommandError::PreparedJournalMissing {
            backup_ref: backup_ref.to_string(),
            path: path.display().to_string(),
        });
    }

    Ok(path)
}

pub(super) fn verify_selected_journal_backup_root(
    backup_ref: &str,
    backup_dir: &Path,
    journal_path: &Path,
) -> Result<(), RestoreCommandError> {
    let journal_path = journal_path.to_path_buf();
    let journal = read_apply_journal(&journal_path)?;
    let Some(actual) = journal.backup_root.as_deref() else {
        return Err(RestoreCommandError::PreparedJournalBackupRootMissing {
            backup_ref: backup_ref.to_string(),
            path: journal_path.display().to_string(),
        });
    };
    if same_path(Path::new(actual), backup_dir) {
        return Ok(());
    }

    Err(RestoreCommandError::PreparedJournalBackupRootMismatch {
        backup_ref: backup_ref.to_string(),
        path: journal_path.display().to_string(),
        expected: backup_dir.display().to_string(),
        actual: actual.to_string(),
    })
}

fn restore_backup_dir(
    backup_ref: Option<&str>,
    backup_dir: Option<&Path>,
) -> Result<Option<PathBuf>, RestoreCommandError> {
    if let Some(backup_dir) = backup_dir {
        return Ok(Some(backup_dir.to_path_buf()));
    }
    backup_ref
        .map(resolve_backup_reference)
        .transpose()
        .map_err(RestoreCommandError::from)
}

// Read and decode a fleet backup manifest from disk.
fn read_manifest(path: &PathBuf) -> Result<FleetBackupManifest, RestoreCommandError> {
    output::read_json_file::<FleetBackupManifest, RestoreCommandError>(path)
}

// Read and decode an optional source-to-target restore mapping from disk.
pub(super) fn read_mapping(path: &PathBuf) -> Result<RestoreMapping, RestoreCommandError> {
    output::read_json_file::<RestoreMapping, RestoreCommandError>(path)
}

// Read and decode a restore plan from disk.
pub(super) fn read_plan(path: &PathBuf) -> Result<RestorePlan, RestoreCommandError> {
    output::read_json_file::<RestorePlan, RestoreCommandError>(path)
}

fn read_apply_journal(path: &PathBuf) -> Result<RestoreApplyJournal, RestoreCommandError> {
    output::read_json_file::<RestoreApplyJournal, RestoreCommandError>(path)
}

fn same_path(left: &Path, right: &Path) -> bool {
    comparable_path(left) == comparable_path(right)
}

fn comparable_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

pub(super) fn write_plan_file(
    path: &PathBuf,
    plan: &RestorePlan,
) -> Result<(), RestoreCommandError> {
    output::write_pretty_json_file(path, plan)
}

pub(super) fn write_apply_journal_file(
    path: &PathBuf,
    dry_run: &RestoreApplyDryRun,
) -> Result<(), RestoreCommandError> {
    output::write_pretty_json_file(path, &RestoreApplyJournal::from_dry_run(dry_run))
}

pub(super) fn write_prepare_report(
    options: &RestorePrepareOptions,
    report: &RestorePrepareReport,
) -> Result<(), RestoreCommandError> {
    output::write_pretty_json(options.out.as_ref(), report)
}

// Write the computed plan to stdout or a requested output file.
pub(super) fn write_plan(
    options: &RestorePlanOptions,
    plan: &RestorePlan,
) -> Result<(), RestoreCommandError> {
    output::write_pretty_json(options.out.as_ref(), plan)
}

// Write the computed apply dry-run to stdout or a requested output file.
pub(super) fn write_apply_dry_run(
    options: &RestoreApplyOptions,
    dry_run: &RestoreApplyDryRun,
) -> Result<(), RestoreCommandError> {
    output::write_pretty_json(options.out.as_ref(), dry_run)
}

// Write the initial apply journal when the caller requests one.
pub(super) fn write_apply_journal_if_requested(
    options: &RestoreApplyOptions,
    dry_run: &RestoreApplyDryRun,
) -> Result<(), RestoreCommandError> {
    let Some(path) = &options.journal_out else {
        return Ok(());
    };

    output::write_pretty_json_file(path, &RestoreApplyJournal::from_dry_run(dry_run))
}

// Write the restore runner response to stdout or a requested output file.
pub(super) fn write_restore_run(
    options: &RestoreRunOptions,
    run: &RestoreRunResponse,
) -> Result<(), RestoreCommandError> {
    output::write_pretty_json(options.out.as_ref(), run)
}

pub(super) fn write_restore_status(
    options: &RestoreStatusOptions,
    run: &RestoreRunResponse,
) -> Result<(), RestoreCommandError> {
    output::write_pretty_json(options.out.as_ref(), run)
}
