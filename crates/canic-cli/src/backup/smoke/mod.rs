use crate::restore as cli_restore;
use canic_backup::restore::RestoreApplyJournal;
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
};

use super::{
    BackupCommandError, BackupPreflightOptions, BackupPreflightReport, BackupSmokeOptions,
    backup_preflight, write_json_file,
};

///
/// BackupSmokeReport
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BackupSmokeReport {
    pub status: String,
    pub backup_id: String,
    pub backup_dir: String,
    pub out_dir: String,
    pub preflight_dir: String,
    pub preflight_summary_path: String,
    pub restore_apply_dry_run_path: String,
    pub restore_apply_journal_path: String,
    pub restore_run_dry_run_path: String,
    pub smoke_summary_path: String,
    pub manifest_design_v1_ready: bool,
    pub restore_ready: bool,
    pub restore_readiness_reasons: Vec<String>,
    pub restore_planned_operations: usize,
    pub runner_preview_written: bool,
}

///
/// SmokeArtifactPaths
///

struct SmokeArtifactPaths {
    preflight_dir: PathBuf,
    restore_apply_dry_run: PathBuf,
    restore_apply_journal: PathBuf,
    restore_run_dry_run: PathBuf,
    smoke_summary: PathBuf,
}

/// Run the post-capture backup/restore smoke path and write all release artifacts.
pub fn backup_smoke(options: &BackupSmokeOptions) -> Result<BackupSmokeReport, BackupCommandError> {
    fs::create_dir_all(&options.out_dir)?;

    let paths = smoke_artifact_paths(&options.out_dir);
    let preflight = backup_preflight(&BackupPreflightOptions {
        dir: options.dir.clone(),
        out_dir: paths.preflight_dir.clone(),
        mapping: options.mapping.clone(),
        require_design_v1: options.require_design_v1,
        require_restore_ready: options.require_restore_ready,
    })?;

    let apply_options = smoke_restore_apply_options(options, &paths);
    let dry_run = cli_restore::restore_apply_dry_run(&apply_options)?;
    write_json_file(&paths.restore_apply_dry_run, &dry_run)?;
    let journal = RestoreApplyJournal::from_dry_run(&dry_run);
    write_json_file(&paths.restore_apply_journal, &journal)?;

    let run_options = smoke_restore_run_options(options, &paths);
    let runner_preview = cli_restore::restore_run_dry_run(&run_options)?;
    write_json_file(&paths.restore_run_dry_run, &runner_preview)?;

    let report = build_smoke_report(options, &paths, &preflight);
    write_json_file(&paths.smoke_summary, &report)?;
    Ok(report)
}

// Build the canonical smoke artifact path set under one output directory.
fn smoke_artifact_paths(out_dir: &Path) -> SmokeArtifactPaths {
    SmokeArtifactPaths {
        preflight_dir: out_dir.join("preflight"),
        restore_apply_dry_run: out_dir.join("restore-apply-dry-run.json"),
        restore_apply_journal: out_dir.join("restore-apply-journal.json"),
        restore_run_dry_run: out_dir.join("restore-run-dry-run.json"),
        smoke_summary: out_dir.join("smoke-summary.json"),
    }
}

// Build restore apply dry-run options for the smoke wrapper.
fn smoke_restore_apply_options(
    options: &BackupSmokeOptions,
    paths: &SmokeArtifactPaths,
) -> cli_restore::RestoreApplyOptions {
    cli_restore::RestoreApplyOptions {
        plan: paths.preflight_dir.join("restore-plan.json"),
        status: Some(paths.preflight_dir.join("restore-status.json")),
        backup_dir: Some(options.dir.clone()),
        out: Some(paths.restore_apply_dry_run.clone()),
        journal_out: Some(paths.restore_apply_journal.clone()),
        dry_run: true,
    }
}

// Build restore runner preview options for the smoke wrapper.
fn smoke_restore_run_options(
    options: &BackupSmokeOptions,
    paths: &SmokeArtifactPaths,
) -> cli_restore::RestoreRunOptions {
    cli_restore::RestoreRunOptions {
        journal: paths.restore_apply_journal.clone(),
        dfx: options.dfx.clone(),
        network: options.network.clone(),
        out: Some(paths.restore_run_dry_run.clone()),
        dry_run: true,
        execute: false,
        unclaim_pending: false,
        max_steps: None,
        updated_at: None,
        require_complete: false,
        require_no_attention: false,
        require_run_mode: None,
        require_stopped_reason: None,
        require_next_action: None,
        require_executed_count: None,
        require_receipt_count: None,
        require_completed_receipt_count: None,
        require_failed_receipt_count: None,
        require_recovered_receipt_count: None,
        require_receipt_updated_at: None,
        require_state_updated_at: None,
        require_remaining_count: None,
        require_attention_count: None,
        require_completion_basis_points: None,
        require_no_pending_before: None,
    }
}

// Build the compact smoke summary mirrored by smoke-summary.json.
fn build_smoke_report(
    options: &BackupSmokeOptions,
    paths: &SmokeArtifactPaths,
    preflight: &BackupPreflightReport,
) -> BackupSmokeReport {
    BackupSmokeReport {
        status: "ready".to_string(),
        backup_id: preflight.backup_id.clone(),
        backup_dir: options.dir.display().to_string(),
        out_dir: options.out_dir.display().to_string(),
        preflight_dir: paths.preflight_dir.display().to_string(),
        preflight_summary_path: paths
            .preflight_dir
            .join("preflight-summary.json")
            .display()
            .to_string(),
        restore_apply_dry_run_path: paths.restore_apply_dry_run.display().to_string(),
        restore_apply_journal_path: paths.restore_apply_journal.display().to_string(),
        restore_run_dry_run_path: paths.restore_run_dry_run.display().to_string(),
        smoke_summary_path: paths.smoke_summary.display().to_string(),
        manifest_design_v1_ready: preflight.manifest_design_v1_ready,
        restore_ready: preflight.restore_ready,
        restore_readiness_reasons: preflight.restore_readiness_reasons.clone(),
        restore_planned_operations: preflight.restore_planned_operations,
        runner_preview_written: true,
    }
}
