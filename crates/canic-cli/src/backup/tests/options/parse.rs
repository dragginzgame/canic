//! Module: backup::tests::options::parse
//!
//! Responsibility: backup option parser success-path tests.
//! Does not own: selector conflict validation.
//! Boundary: parsed option field values for backup subcommands.

use super::super::super::*;
use std::{ffi::OsString, path::PathBuf};

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
        OsString::from(crate::cli::globals::INTERNAL_ENVIRONMENT_OPTION),
        OsString::from("local"),
        OsString::from(crate::cli::globals::INTERNAL_ICP_OPTION),
        OsString::from("/bin/icp"),
    ])
    .expect("parse options");

    assert_eq!(options.deployment, "demo");
    assert_eq!(options.subtree, Some("app".to_string()));
    assert_eq!(options.out, Some(PathBuf::from("backups/plan")));
    assert!(options.dry_run);
    assert_eq!(options.environment, "local");
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
