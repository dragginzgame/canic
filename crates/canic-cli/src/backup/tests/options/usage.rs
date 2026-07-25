//! Module: backup::tests::options::usage
//!
//! Responsibility: backup CLI usage text tests.
//! Does not own: concrete option parser field assertions.
//! Boundary: human-facing backup command help text.

use super::super::super::*;
use crate::backup::manifest::ManifestCommandError;
use canic_host::installed_fleet::InstalledFleetError;
use std::ffi::OsString;

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
fn backup_create_usage_uses_fleet_target_wording() {
    let text = create_usage();

    assert!(text.contains("Usage: canic backup create [OPTIONS] <fleet>"));
    assert!(text.contains("Create a topology-aware deployment backup"));
    assert!(text.contains("Installed Fleet name to back up"));
    assert!(text.contains("backups/deployment-<name>-YYYYMMDD-HHMMSS"));
    assert!(!text.contains("backups/fleet-<name>"));
}

#[test]
fn missing_backup_fleet_preserves_canonical_typed_error() {
    let error = BackupCommandError::from(InstalledFleetError::NoInstalledFleet {
        environment: "local".to_string(),
        fleet: "demo-local".to_string(),
    });

    std::assert_matches!(
        error,
        BackupCommandError::InstalledFleet(InstalledFleetError::NoInstalledFleet {
            environment,
            fleet,
        }) if environment == "local" && fleet == "demo-local"
    );
}

#[test]
fn backup_dispatch_preserves_manifest_command_error() {
    let error = run([OsString::from("manifest"), OsString::from("validate")])
        .expect_err("missing manifest argument rejects");

    std::assert_matches!(
        error,
        BackupCommandError::Manifest(ManifestCommandError::Usage(_))
    );
}
