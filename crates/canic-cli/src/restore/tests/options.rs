use super::*;
use canic_backup::restore::parse_uploaded_snapshot_id;
use std::{ffi::OsString, path::PathBuf};

// Ensure restore plan options parse the intended no-mutation command.
#[test]
fn parses_restore_plan_options() {
    let options = RestorePlanOptions::parse([
        OsString::from("--manifest"),
        OsString::from("manifest.json"),
        OsString::from("--mapping"),
        OsString::from("mapping.json"),
        OsString::from("--out"),
        OsString::from("plan.json"),
        OsString::from("--require-restore-ready"),
    ])
    .expect("parse options");

    assert_eq!(options.manifest, Some(PathBuf::from("manifest.json")));
    assert_eq!(options.backup_dir, None);
    assert_eq!(options.mapping, Some(PathBuf::from("mapping.json")));
    assert_eq!(options.out, Some(PathBuf::from("plan.json")));
    assert!(!options.require_verified);
    assert!(options.require_restore_ready);
}

// Ensure restore help stays at command-family level.
#[test]
fn restore_usage_lists_command_family() {
    let text = usage();

    assert!(text.contains("Usage: canic restore"));
    assert!(text.contains("plan"));
    assert!(text.contains("run"));
}

// Ensure uploaded snapshot IDs are parsed from command upload output.
#[test]
fn parses_uploaded_snapshot_id_from_icp_output() {
    let snapshot_id = parse_uploaded_snapshot_id("Uploaded snapshot: target-snap-001\n");

    assert_eq!(snapshot_id.as_deref(), Some("target-snap-001"));
}

// Ensure verified restore plan options parse with the canonical backup source.
#[test]
fn parses_verified_restore_plan_options() {
    let options = RestorePlanOptions::parse([
        OsString::from("--backup-dir"),
        OsString::from("backups/run"),
        OsString::from("--require-verified"),
    ])
    .expect("parse verified options");

    assert_eq!(options.manifest, None);
    assert_eq!(options.backup_dir, Some(PathBuf::from("backups/run")));
    assert_eq!(options.mapping, None);
    assert_eq!(options.out, None);
    assert!(options.require_verified);
    assert!(!options.require_restore_ready);
}

// Ensure restore apply options require the explicit dry-run mode.
#[test]
fn parses_restore_apply_dry_run_options() {
    let options = RestoreApplyOptions::parse([
        OsString::from("--plan"),
        OsString::from("restore-plan.json"),
        OsString::from("--backup-dir"),
        OsString::from("backups/run"),
        OsString::from("--dry-run"),
        OsString::from("--out"),
        OsString::from("restore-apply-dry-run.json"),
        OsString::from("--journal-out"),
        OsString::from("restore-apply-journal.json"),
    ])
    .expect("parse apply options");

    assert_eq!(options.plan, PathBuf::from("restore-plan.json"));
    assert_eq!(options.backup_dir, Some(PathBuf::from("backups/run")));
    assert_eq!(
        options.out,
        Some(PathBuf::from("restore-apply-dry-run.json"))
    );
    assert_eq!(
        options.journal_out,
        Some(PathBuf::from("restore-apply-journal.json"))
    );
    assert!(options.dry_run);
}

// Ensure restore run options parse the native runner dry-run command.
#[test]
fn parses_restore_run_dry_run_options() {
    let options = RestoreRunOptions::parse([
        OsString::from("--journal"),
        OsString::from("restore-apply-journal.json"),
        OsString::from("--dry-run"),
        OsString::from(crate::cli::globals::INTERNAL_ICP_OPTION),
        OsString::from("/tmp/icp"),
        OsString::from(crate::cli::globals::INTERNAL_NETWORK_OPTION),
        OsString::from("local"),
        OsString::from("--out"),
        OsString::from("restore-run-dry-run.json"),
        OsString::from("--max-steps"),
        OsString::from("1"),
        OsString::from("--require-complete"),
        OsString::from("--require-no-attention"),
    ])
    .expect("parse restore run options");

    assert_eq!(options.journal, PathBuf::from("restore-apply-journal.json"));
    assert_eq!(options.icp, "/tmp/icp");
    assert_eq!(options.network.as_deref(), Some("local"));
    assert_eq!(options.out, Some(PathBuf::from("restore-run-dry-run.json")));
    assert!(options.dry_run);
    assert!(!options.execute);
    assert!(!options.retry_failed);
    assert!(!options.unclaim_pending);
    assert_eq!(options.max_steps, Some(1));
    assert!(options.require_complete);
    assert!(options.require_no_attention);
}

// Ensure restore run options parse the native execute command.
#[test]
fn parses_restore_run_execute_options() {
    let options = RestoreRunOptions::parse([
        OsString::from("--journal"),
        OsString::from("restore-apply-journal.json"),
        OsString::from("--execute"),
        OsString::from(crate::cli::globals::INTERNAL_ICP_OPTION),
        OsString::from("/bin/true"),
        OsString::from("--max-steps"),
        OsString::from("4"),
    ])
    .expect("parse restore run execute options");

    assert_eq!(options.journal, PathBuf::from("restore-apply-journal.json"));
    assert_eq!(options.icp, "/bin/true");
    assert_eq!(options.network, None);
    assert_eq!(options.out, None);
    assert!(!options.dry_run);
    assert!(options.execute);
    assert!(!options.retry_failed);
    assert!(!options.unclaim_pending);
    assert_eq!(options.max_steps, Some(4));
    assert!(!options.require_complete);
    assert!(!options.require_no_attention);
}

// Ensure restore run options parse the native pending-operation recovery mode.
#[test]
fn parses_restore_run_unclaim_pending_options() {
    let options = RestoreRunOptions::parse([
        OsString::from("--journal"),
        OsString::from("restore-apply-journal.json"),
        OsString::from("--unclaim-pending"),
        OsString::from("--out"),
        OsString::from("restore-run.json"),
    ])
    .expect("parse restore run unclaim options");

    assert_eq!(options.journal, PathBuf::from("restore-apply-journal.json"));
    assert_eq!(options.out, Some(PathBuf::from("restore-run.json")));
    assert!(!options.dry_run);
    assert!(!options.execute);
    assert!(!options.retry_failed);
    assert!(options.unclaim_pending);
}

// Ensure restore run options parse the native failed-operation recovery mode.
#[test]
fn parses_restore_run_retry_failed_options() {
    let options = RestoreRunOptions::parse([
        OsString::from("--journal"),
        OsString::from("restore-apply-journal.json"),
        OsString::from("--retry-failed"),
        OsString::from("--out"),
        OsString::from("restore-run.json"),
    ])
    .expect("parse restore run retry options");

    assert_eq!(options.journal, PathBuf::from("restore-apply-journal.json"));
    assert_eq!(options.out, Some(PathBuf::from("restore-run.json")));
    assert!(!options.dry_run);
    assert!(!options.execute);
    assert!(options.retry_failed);
    assert!(!options.unclaim_pending);
}

// Ensure restore apply only renders no-mutation operation plans.
#[test]
fn restore_apply_requires_dry_run() {
    let err = RestoreApplyOptions::parse([
        OsString::from("--plan"),
        OsString::from("restore-plan.json"),
    ])
    .expect_err("apply without dry-run should fail");

    assert!(matches!(err, RestoreCommandError::Usage(_)));
}

// Ensure restore run requires an explicit execution mode.
#[test]
fn restore_run_requires_mode() {
    let err = RestoreRunOptions::parse([
        OsString::from("--journal"),
        OsString::from("restore-apply-journal.json"),
    ])
    .expect_err("restore run without dry-run should fail");

    assert!(matches!(err, RestoreCommandError::Usage(_)));
}

// Ensure restore run rejects ambiguous execution modes.
#[test]
fn restore_run_rejects_conflicting_modes() {
    let err = RestoreRunOptions::parse([
        OsString::from("--journal"),
        OsString::from("restore-apply-journal.json"),
        OsString::from("--dry-run"),
        OsString::from("--execute"),
        OsString::from("--retry-failed"),
        OsString::from("--unclaim-pending"),
    ])
    .expect_err("restore run should reject conflicting modes");

    assert!(matches!(err, RestoreCommandError::Usage(_)));
}

// Ensure restore run rejects zero-length execute batches.
#[test]
fn restore_run_rejects_zero_max_steps() {
    let err = RestoreRunOptions::parse([
        OsString::from("--journal"),
        OsString::from("restore-apply-journal.json"),
        OsString::from("--execute"),
        OsString::from("--max-steps"),
        OsString::from("0"),
    ])
    .expect_err("restore run should reject zero max steps");

    assert!(matches!(err, RestoreCommandError::Usage(_)));
}
