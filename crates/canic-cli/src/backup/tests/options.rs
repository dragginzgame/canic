//! Module: backup::tests::options
//!
//! Responsibility: backup CLI usage text and option parsing tests.
//! Does not own: backup execution, persistence, or fixture construction.
//! Boundary: command-line surface validation for the backup command family.

use super::super::*;
use std::{ffi::OsString, path::PathBuf};

// Ensure backup help stays at command-family level.
#[test]
fn backup_usage_lists_commands_without_nested_flag_dump() {
    let text = usage();

    assert!(text.contains("Usage: canic backup"));
    assert!(text.contains("create"));
    assert!(text.contains("list"));
    assert!(text.contains("inspect"));
    assert!(text.contains("prune"));
    assert!(text.contains("verify"));
    assert!(text.contains("status"));
}

#[test]
fn backup_create_usage_uses_deployment_target_wording() {
    let text = create_usage();

    assert!(text.contains("Usage: canic backup create [OPTIONS] <deployment>"));
    assert!(text.contains("Create a topology-aware deployment backup"));
    assert!(text.contains("Installed deployment target name to back up"));
    assert!(text.contains("backups/deployment-<name>-YYYYMMDD-HHMMSS"));
    assert!(!text.contains("backups/fleet-<name>"));
    assert!(!text.contains("Installed fleet"));
}

#[test]
fn missing_backup_deployment_mentions_unverified_registration_acknowledgement() {
    let message = BackupCommandError::NoInstalledDeployment {
        network: "local".to_string(),
        deployment: "demo-local".to_string(),
    }
    .to_string();

    assert!(message.contains("canic deploy register demo-local"));
    assert!(message.contains("--allow-unverified"));
}

// Ensure backup create options parse planning and live-execution controls.
#[test]
fn parses_backup_create_options() {
    let options = BackupCreateOptions::parse([
        OsString::from("demo"),
        OsString::from("--subtree"),
        OsString::from("app"),
        OsString::from("--out"),
        OsString::from("backups/plan"),
        OsString::from("--dry-run"),
        OsString::from(crate::cli::globals::INTERNAL_NETWORK_OPTION),
        OsString::from("local"),
        OsString::from(crate::cli::globals::INTERNAL_ICP_OPTION),
        OsString::from("/bin/icp"),
    ])
    .expect("parse options");

    assert_eq!(options.deployment, "demo");
    assert_eq!(options.subtree, Some("app".to_string()));
    assert_eq!(options.out, Some(PathBuf::from("backups/plan")));
    assert!(options.dry_run);
    assert_eq!(options.network, "local");
    assert_eq!(options.icp, "/bin/icp");
}

// Ensure backup prune options parse cleanup selectors and preview mode.
#[test]
fn parses_backup_prune_options() {
    let options = BackupPruneOptions::parse([
        OsString::from("--dir"),
        OsString::from("archive"),
        OsString::from("--failed"),
        OsString::from("--keep"),
        OsString::from("5"),
        OsString::from("--dry-run"),
        OsString::from("--out"),
        OsString::from("prune.txt"),
    ])
    .expect("parse prune options");

    assert_eq!(options.dir, PathBuf::from("archive"));
    assert!(options.failed);
    assert_eq!(options.keep, Some(5));
    assert!(options.dry_run);
    assert_eq!(options.out, Some(PathBuf::from("prune.txt")));
}

// Ensure backup list options default to the conventional backup root.
#[test]
fn parses_backup_list_options() {
    let options = BackupListOptions::parse([
        OsString::from("--dir"),
        OsString::from("snapshots"),
        OsString::from("--out"),
        OsString::from("backups.txt"),
    ])
    .expect("parse options");

    assert_eq!(options.dir, PathBuf::from("snapshots"));
    assert_eq!(options.out, Some(PathBuf::from("backups.txt")));

    let default_options = BackupListOptions::parse([]).expect("parse default options");
    assert_eq!(default_options.dir, PathBuf::from("backups"));
}

// Ensure backup verification options parse the intended command shape.
#[test]
fn parses_backup_verify_options() {
    let options = BackupVerifyOptions::parse([
        OsString::from("--dir"),
        OsString::from("backups/run"),
        OsString::from("--out"),
        OsString::from("report.json"),
    ])
    .expect("parse options");

    assert_eq!(options.backup_ref, None);
    assert_eq!(options.dir, Some(PathBuf::from("backups/run")));
    assert_eq!(options.out, Some(PathBuf::from("report.json")));

    let referenced = BackupVerifyOptions::parse([OsString::from("1")]).expect("parse reference");
    assert_eq!(referenced.backup_ref, Some("1".to_string()));
    assert_eq!(referenced.dir, None);
}

// Ensure backup status options parse the intended command shape.
#[test]
fn parses_backup_status_options() {
    let options = BackupStatusOptions::parse([
        OsString::from("--dir"),
        OsString::from("backups/run"),
        OsString::from("--out"),
        OsString::from("status.json"),
        OsString::from("--require-complete"),
    ])
    .expect("parse options");

    assert_eq!(options.backup_ref, None);
    assert_eq!(options.dir, Some(PathBuf::from("backups/run")));
    assert_eq!(options.out, Some(PathBuf::from("status.json")));
    assert!(options.require_complete);

    let referenced = BackupStatusOptions::parse([OsString::from("plan-demo-20260511-001234")])
        .expect("parse reference");
    assert_eq!(
        referenced.backup_ref,
        Some("plan-demo-20260511-001234".to_string())
    );
    assert_eq!(referenced.dir, None);
}

// Ensure backup inspect options parse the intended command shape.
#[test]
fn parses_backup_inspect_options() {
    let options = BackupInspectOptions::parse([
        OsString::from("--dir"),
        OsString::from("backups/run"),
        OsString::from("--out"),
        OsString::from("inspect.txt"),
        OsString::from("--json"),
    ])
    .expect("parse options");

    assert_eq!(options.backup_ref, None);
    assert_eq!(options.dir, Some(PathBuf::from("backups/run")));
    assert_eq!(options.out, Some(PathBuf::from("inspect.txt")));
    assert!(options.json);

    let referenced =
        BackupInspectOptions::parse([OsString::from("backup-test"), OsString::from("--json")])
            .expect("parse reference");
    assert_eq!(referenced.backup_ref, Some("backup-test".to_string()));
    assert_eq!(referenced.dir, None);
}

// Ensure commands require one backup selector path, either by reference or explicit dir.
#[test]
fn backup_target_options_reject_missing_or_duplicate_selectors() {
    let missing = BackupInspectOptions::parse([]).expect_err("missing selector rejects");
    std::assert_matches!(missing, BackupCommandError::Usage(_));

    let duplicate = BackupInspectOptions::parse([
        OsString::from("1"),
        OsString::from("--dir"),
        OsString::from("backups/run"),
    ])
    .expect_err("duplicate selector rejects");
    std::assert_matches!(duplicate, BackupCommandError::Usage(_));

    let invalid_keep =
        BackupPruneOptions::parse([OsString::from("--keep"), OsString::from("banana")])
            .expect_err("invalid keep rejects");
    std::assert_matches!(invalid_keep, BackupCommandError::Usage(_));

    let missing_prune_selector =
        BackupPruneOptions::parse([]).expect_err("missing prune selector rejects");
    std::assert_matches!(missing_prune_selector, BackupCommandError::Usage(_));
}
