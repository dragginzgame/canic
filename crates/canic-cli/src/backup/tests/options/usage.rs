//! Module: backup::tests::options::usage
//!
//! Responsibility: backup CLI usage text tests.
//! Does not own: concrete option parser field assertions.
//! Boundary: human-facing backup command help text.

use super::super::super::*;

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
