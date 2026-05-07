use crate::output;
use canic_backup::{
    manifest::FleetBackupManifest,
    persistence::BackupLayout,
    restore::{
        RestoreApplyDryRun, RestoreApplyJournal, RestoreMapping, RestorePlan, RestoreRunResponse,
    },
};
use std::path::PathBuf;

use super::{RestoreApplyOptions, RestoreCommandError, RestorePlanOptions, RestoreRunOptions};

// Verify backup layout integrity before restore planning when requested.
pub(super) fn verify_backup_layout_if_required(
    options: &RestorePlanOptions,
) -> Result<(), RestoreCommandError> {
    if !options.require_verified {
        return Ok(());
    }

    let Some(dir) = &options.backup_dir else {
        return Err(RestoreCommandError::RequireVerifiedNeedsBackupDir);
    };

    BackupLayout::new(dir.clone()).verify_integrity()?;
    Ok(())
}

// Read the manifest from a direct path or canonical backup layout.
pub(super) fn read_manifest_source(
    options: &RestorePlanOptions,
) -> Result<FleetBackupManifest, RestoreCommandError> {
    if let Some(path) = &options.manifest {
        return read_manifest(path);
    }

    let Some(dir) = &options.backup_dir else {
        return Err(RestoreCommandError::MissingOption(
            "--manifest or --backup-dir",
        ));
    };

    BackupLayout::new(dir.clone())
        .read_manifest()
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
