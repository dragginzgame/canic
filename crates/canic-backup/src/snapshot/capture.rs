use super::{
    SnapshotArtifact, SnapshotDownloadConfig, SnapshotDownloadError, SnapshotDriver,
    support::safe_path_segment,
};
use crate::{
    artifacts::ArtifactChecksum,
    discovery::SnapshotTarget,
    journal::{ArtifactJournalEntry, ArtifactState, DownloadJournal},
    persistence::BackupLayout,
    timestamp::current_timestamp_marker,
};
use std::{
    fs,
    path::{Path, PathBuf},
};

///
/// SnapshotArtifactPaths
///

pub(super) struct SnapshotArtifactPaths {
    pub(super) artifact_path: PathBuf,
    relative_path: PathBuf,
    temp_path: PathBuf,
}

impl SnapshotArtifactPaths {
    pub(super) fn new(root: &Path, canister_id: &str) -> Self {
        let relative_path = PathBuf::from(safe_path_segment(canister_id));
        let artifact_path = root.join(&relative_path);
        let temp_path = root.join(format!("{}.tmp", safe_path_segment(canister_id)));

        Self {
            artifact_path,
            relative_path,
            temp_path,
        }
    }
}

pub(super) fn dry_run_artifact(
    config: &SnapshotDownloadConfig,
    driver: &impl SnapshotDriver,
    target: &SnapshotTarget,
    artifact_path: PathBuf,
) -> (SnapshotArtifact, Vec<String>) {
    let mut commands = Vec::new();
    if config.lifecycle.stop_before_snapshot() {
        commands.push(driver.stop_canister_command(&target.canister_id));
    }
    commands.push(driver.create_snapshot_command(&target.canister_id));
    commands.push(driver.download_snapshot_command(
        &target.canister_id,
        "<snapshot-id>",
        &artifact_path,
    ));
    if config.lifecycle.resume_after_snapshot() {
        commands.push(driver.start_canister_command(&target.canister_id));
    }

    (
        SnapshotArtifact {
            canister_id: target.canister_id.clone(),
            snapshot_id: "<snapshot-id>".to_string(),
            path: artifact_path,
            checksum: "<sha256>".to_string(),
        },
        commands,
    )
}

pub(super) fn capture_snapshot_artifact(
    config: &SnapshotDownloadConfig,
    driver: &mut impl SnapshotDriver,
    layout: &BackupLayout,
    journal: &mut DownloadJournal,
    target: &SnapshotTarget,
    paths: SnapshotArtifactPaths,
) -> Result<SnapshotArtifact, SnapshotDownloadError> {
    if config.lifecycle.stop_before_snapshot() {
        driver
            .stop_canister(&target.canister_id)
            .map_err(SnapshotDownloadError::Driver)?;
    }

    let result = capture_snapshot_artifact_body(
        driver,
        layout,
        journal,
        target,
        &paths.relative_path,
        paths.artifact_path,
        paths.temp_path,
    );

    if config.lifecycle.resume_after_snapshot() {
        match result {
            Ok(artifact) => {
                driver
                    .start_canister(&target.canister_id)
                    .map_err(SnapshotDownloadError::Driver)?;
                Ok(artifact)
            }
            Err(error) => {
                let _ = driver.start_canister(&target.canister_id);
                Err(error)
            }
        }
    } else {
        result
    }
}

fn capture_snapshot_artifact_body(
    driver: &mut impl SnapshotDriver,
    layout: &BackupLayout,
    journal: &mut DownloadJournal,
    target: &SnapshotTarget,
    artifact_relative_path: &Path,
    artifact_path: PathBuf,
    temp_path: PathBuf,
) -> Result<SnapshotArtifact, SnapshotDownloadError> {
    journal.operation_metrics.snapshot_create_started += 1;
    let snapshot_id = driver
        .create_snapshot(&target.canister_id)
        .map_err(SnapshotDownloadError::Driver)?;
    journal.operation_metrics.snapshot_create_completed += 1;
    let mut entry = ArtifactJournalEntry {
        canister_id: target.canister_id.clone(),
        snapshot_id: snapshot_id.clone(),
        state: ArtifactState::Created,
        temp_path: None,
        artifact_path: artifact_relative_path.display().to_string(),
        checksum_algorithm: "sha256".to_string(),
        checksum: None,
        updated_at: current_timestamp_marker(),
    };
    journal.artifacts.push(entry.clone());
    layout.write_journal(journal)?;

    if temp_path.exists() {
        fs::remove_dir_all(&temp_path)?;
    }
    fs::create_dir_all(&temp_path)?;
    journal.operation_metrics.snapshot_download_started += 1;
    layout.write_journal(journal)?;
    driver
        .download_snapshot(&target.canister_id, &snapshot_id, &temp_path)
        .map_err(SnapshotDownloadError::Driver)?;
    journal.operation_metrics.snapshot_download_completed += 1;
    entry.advance_to(ArtifactState::Downloaded, current_timestamp_marker())?;
    entry.temp_path = Some(temp_path.display().to_string());
    update_journal_entry(journal, &entry);
    layout.write_journal(journal)?;

    journal.operation_metrics.checksum_verify_started += 1;
    layout.write_journal(journal)?;
    let checksum = ArtifactChecksum::from_path(&temp_path)?;
    journal.operation_metrics.checksum_verify_completed += 1;
    entry.checksum = Some(checksum.hash.clone());
    entry.advance_to(ArtifactState::ChecksumVerified, current_timestamp_marker())?;
    update_journal_entry(journal, &entry);
    layout.write_journal(journal)?;

    journal.operation_metrics.artifact_finalize_started += 1;
    layout.write_journal(journal)?;
    if artifact_path.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!("artifact path already exists: {}", artifact_path.display()),
        )
        .into());
    }
    fs::rename(&temp_path, &artifact_path)?;
    journal.operation_metrics.artifact_finalize_completed += 1;
    entry.temp_path = None;
    entry.advance_to(ArtifactState::Durable, current_timestamp_marker())?;
    update_journal_entry(journal, &entry);
    layout.write_journal(journal)?;

    Ok(SnapshotArtifact {
        canister_id: target.canister_id.clone(),
        snapshot_id,
        path: artifact_path,
        checksum: checksum.hash,
    })
}

fn update_journal_entry(journal: &mut DownloadJournal, entry: &ArtifactJournalEntry) {
    if let Some(existing) = journal.artifacts.iter_mut().find(|existing| {
        existing.canister_id == entry.canister_id && existing.snapshot_id == entry.snapshot_id
    }) {
        *existing = entry.clone();
    }
}
