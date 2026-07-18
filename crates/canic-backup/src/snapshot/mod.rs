mod capture;
mod manifest;
mod support;
mod targets;
mod topology;
mod types;

pub use manifest::build_snapshot_manifest;
pub use targets::resolve_snapshot_targets;
pub use topology::{ensure_topology_stable, topology_hash_for_targets};
pub use types::*;

use crate::{
    journal::DownloadJournal, journal::DownloadOperationMetrics, persistence::BackupLayout,
};
use capture::{SnapshotArtifactPaths, capture_snapshot_artifact, dry_run_artifact};
use topology::accepted_pre_snapshot_topology_hash;

/// Create and download snapshots for the selected canister set.
pub fn download_snapshots(
    config: &SnapshotDownloadConfig,
    driver: &mut impl SnapshotDriver,
) -> Result<SnapshotDownloadResult, SnapshotDownloadError> {
    validate_snapshot_lifecycle(config.lifecycle)?;
    let targets = resolve_snapshot_targets(config, driver)?;
    let discovery_topology_hash = topology_hash_for_targets(&config.canister, &targets)?;
    let pre_snapshot_topology_hash =
        accepted_pre_snapshot_topology_hash(config, driver, &discovery_topology_hash)?;
    let layout = BackupLayout::new(config.out.clone());
    let mut artifacts = Vec::with_capacity(targets.len());
    let mut planned_commands = Vec::new();
    let mut journal = DownloadJournal {
        journal_version: 1,
        backup_id: config.backup_id.clone(),
        discovery_topology_hash: Some(discovery_topology_hash.hash.clone()),
        pre_snapshot_topology_hash: Some(pre_snapshot_topology_hash.hash.clone()),
        operation_metrics: DownloadOperationMetrics {
            target_count: targets.len(),
            ..DownloadOperationMetrics::default()
        },
        artifacts: Vec::new(),
    };

    for target in &targets {
        let paths = SnapshotArtifactPaths::new(&config.out, &target.canister_id);

        if config.dry_run {
            let (artifact, commands) =
                dry_run_artifact(config, driver, target, paths.artifact_path);
            artifacts.push(artifact);
            planned_commands.extend(commands);
            continue;
        }

        artifacts.push(capture_snapshot_artifact(
            config,
            driver,
            &layout,
            &mut journal,
            target,
            paths,
        )?);
    }

    if !config.dry_run {
        let manifest = build_snapshot_manifest(SnapshotManifestInput {
            backup_id: config.backup_id.clone(),
            created_at: config.created_at.clone(),
            tool_name: config.tool_name.clone(),
            tool_version: config.tool_version.clone(),
            environment: config.environment.clone(),
            root_canister: config
                .root
                .clone()
                .unwrap_or_else(|| config.canister.clone()),
            selected_canister: config.canister.clone(),
            include_children: config.include_children,
            targets: &targets,
            artifacts: &artifacts,
            discovery_topology_hash,
            pre_snapshot_topology_hash,
        })?;
        layout.write_manifest(&manifest)?;
    }

    Ok(SnapshotDownloadResult {
        artifacts,
        planned_commands,
    })
}

const fn validate_snapshot_lifecycle(
    lifecycle: SnapshotLifecycleMode,
) -> Result<(), SnapshotDownloadError> {
    if lifecycle.stop_before_snapshot() {
        return Ok(());
    }

    Err(SnapshotDownloadError::SnapshotRequiresStoppedCanister)
}

#[cfg(test)]
mod tests;
