//! Module: backup::create::plan
//!
//! Responsibility: build backup create planning inputs and topology hashes.
//! Does not own: CLI option parsing, persistence, or runner execution.
//! Boundary: deterministic conversion from installed deployment rows to backup plan inputs.

use super::super::BackupCommandError;
use crate::support::path_stamp::{current_backup_directory_stamp, file_safe_component};
use candid::Principal;
use canic_backup::{
    plan::{AuthorityEvidence, ControlAuthority, SnapshotReadAuthority},
    registry::RegistryEntry as BackupRegistryEntry,
    topology::{TopologyHasher, TopologyRecord},
};
use canic_host::registry::RegistryEntry as HostRegistryEntry;
use std::path::PathBuf;

pub(super) fn backup_registry_entries(entries: &[HostRegistryEntry]) -> Vec<BackupRegistryEntry> {
    entries
        .iter()
        .map(|entry| BackupRegistryEntry {
            pid: entry.pid.clone(),
            role: entry.role.clone(),
            kind: entry.kind.clone(),
            parent_pid: entry.parent_pid.clone(),
            module_hash: entry.module_hash.clone(),
        })
        .collect()
}

pub(super) fn registry_topology_hash(
    registry: &[BackupRegistryEntry],
) -> Result<String, BackupCommandError> {
    let records = registry
        .iter()
        .map(|entry| {
            Ok(TopologyRecord {
                pid: Principal::from_text(&entry.pid).map_err(|_| {
                    BackupCommandError::InvalidRegistryPrincipal {
                        canister_id: entry.pid.clone(),
                    }
                })?,
                parent_pid: entry
                    .parent_pid
                    .as_deref()
                    .map(Principal::from_text)
                    .transpose()
                    .map_err(|_| BackupCommandError::InvalidRegistryPrincipal {
                        canister_id: entry.parent_pid.clone().unwrap_or_default(),
                    })?,
                role: entry.role.clone().unwrap_or_default(),
                module_hash: entry.module_hash.clone(),
            })
        })
        .collect::<Result<Vec<_>, BackupCommandError>>()?;

    Ok(TopologyHasher::hash(&records).hash)
}

pub(super) fn backup_plan_id(deployment: &str) -> String {
    format!(
        "plan-{}-{}",
        file_safe_component(deployment),
        current_backup_directory_stamp()
    )
}

pub(super) fn default_backup_output_path(deployment: &str) -> PathBuf {
    PathBuf::from("backups").join(format!(
        "deployment-{}-{}",
        file_safe_component(deployment),
        current_backup_directory_stamp()
    ))
}

pub(super) const fn backup_control_authority(dry_run: bool) -> ControlAuthority {
    if dry_run {
        ControlAuthority::root_controller(AuthorityEvidence::Declared)
    } else {
        ControlAuthority::operator_controller(AuthorityEvidence::Proven)
    }
}

pub(super) const fn backup_snapshot_read_authority(dry_run: bool) -> SnapshotReadAuthority {
    if dry_run {
        SnapshotReadAuthority::root_configured_read(AuthorityEvidence::Declared)
    } else {
        SnapshotReadAuthority::operator_controller(AuthorityEvidence::Proven)
    }
}

pub(super) const fn backup_quiescence_policy(
    dry_run: bool,
) -> canic_backup::plan::QuiescencePolicy {
    if dry_run {
        canic_backup::plan::QuiescencePolicy::RootCoordinated
    } else {
        canic_backup::plan::QuiescencePolicy::CrashConsistent
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Ensure generated backup layout paths use deployment-target naming.
    #[test]
    fn default_backup_output_path_uses_deployment_prefix() {
        let path = default_backup_output_path("demo local");
        let rendered = path.display().to_string();

        assert!(rendered.starts_with("backups/deployment-demo-local-"));
        assert!(!rendered.contains("fleet-demo"));
    }

    // Ensure generated plan ids are derived from the deployment target.
    #[test]
    fn backup_plan_id_uses_deployment_target() {
        let plan_id = backup_plan_id("demo local");

        assert!(plan_id.starts_with("plan-demo-local-"));
    }
}
