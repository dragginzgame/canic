//! Module: persistence::integrity::artifacts
//!
//! Responsibility: verify manifest, journal, and artifact checksum consistency.
//! Does not own: JSON persistence, manifest validation, or journal mutation.
//! Boundary: returns typed integrity reports or persistence errors.

use crate::{
    artifacts::ArtifactChecksum,
    journal::{ArtifactJournalEntry, ArtifactState, DownloadJournal},
    manifest::{DeploymentBackupManifest, DeploymentMember},
    persistence::{
        ArtifactIntegrityReport, BackupIntegrityReport, BackupLayout, PersistenceError,
        integrity::{resolve_backup_artifact_path, topology::verify_manifest_journal_binding},
    },
};

use std::collections::BTreeSet;

pub(in crate::persistence) fn verify_layout_integrity(
    layout: &BackupLayout,
    manifest: &DeploymentBackupManifest,
    journal: &DownloadJournal,
) -> Result<BackupIntegrityReport, PersistenceError> {
    verify_manifest_journal_binding(manifest, journal)?;

    let expected_artifacts = expected_artifact_keys(manifest);
    for entry in &journal.artifacts {
        if !expected_artifacts.contains(&(entry.canister_id.as_str(), entry.snapshot_id.as_str())) {
            return Err(PersistenceError::UnexpectedJournalArtifact {
                canister_id: entry.canister_id.clone(),
                snapshot_id: entry.snapshot_id.clone(),
            });
        }
    }

    let mut artifacts = Vec::with_capacity(journal.artifacts.len());
    for member in &manifest.deployment.members {
        artifacts.push(verify_member_artifact(layout, journal, member)?);
    }

    Ok(BackupIntegrityReport {
        backup_id: manifest.backup_id.clone(),
        verified: true,
        manifest_members: manifest.deployment.members.len(),
        journal_artifacts: journal.artifacts.len(),
        durable_artifacts: artifacts.len(),
        artifacts,
    })
}

fn expected_artifact_keys(manifest: &DeploymentBackupManifest) -> BTreeSet<(&str, &str)> {
    manifest
        .deployment
        .members
        .iter()
        .map(|member| {
            (
                member.canister_id.as_str(),
                member.source_snapshot.snapshot_id.as_str(),
            )
        })
        .collect()
}

fn verify_member_artifact(
    layout: &BackupLayout,
    journal: &DownloadJournal,
    member: &DeploymentMember,
) -> Result<ArtifactIntegrityReport, PersistenceError> {
    let Some(entry) = journal.artifacts.iter().find(|entry| {
        entry.canister_id == member.canister_id
            && entry.snapshot_id == member.source_snapshot.snapshot_id
    }) else {
        return Err(PersistenceError::MissingJournalArtifact {
            canister_id: member.canister_id.clone(),
            snapshot_id: member.source_snapshot.snapshot_id.clone(),
        });
    };

    if entry.state != ArtifactState::Durable {
        return Err(PersistenceError::NonDurableArtifact {
            canister_id: entry.canister_id.clone(),
            snapshot_id: entry.snapshot_id.clone(),
        });
    }

    let expected_hash = entry.checksum.as_deref().ok_or_else(|| {
        PersistenceError::MissingJournalArtifactChecksum {
            canister_id: entry.canister_id.clone(),
            snapshot_id: entry.snapshot_id.clone(),
        }
    })?;
    validate_member_artifact_metadata(member, entry, expected_hash)?;
    let artifact_path = resolve_backup_artifact_path(layout.root(), &entry.artifact_path)
        .ok_or_else(|| PersistenceError::ArtifactPathEscapesBackup {
            artifact_path: entry.artifact_path.clone(),
        })?;
    if !artifact_path.exists() {
        return Err(PersistenceError::MissingArtifact(
            artifact_path.display().to_string(),
        ));
    }

    ArtifactChecksum::from_relative_path_no_follow(
        layout.root(),
        std::path::Path::new(&entry.artifact_path),
    )?
    .verify(expected_hash)?;
    Ok(ArtifactIntegrityReport {
        canister_id: entry.canister_id.clone(),
        snapshot_id: entry.snapshot_id.clone(),
        artifact_path: artifact_path.display().to_string(),
        checksum: expected_hash.to_string(),
    })
}

fn validate_member_artifact_metadata(
    member: &DeploymentMember,
    entry: &ArtifactJournalEntry,
    expected_hash: &str,
) -> Result<(), PersistenceError> {
    if member.source_snapshot.artifact_path != entry.artifact_path {
        return Err(PersistenceError::ManifestJournalArtifactPathMismatch {
            canister_id: entry.canister_id.clone(),
            snapshot_id: entry.snapshot_id.clone(),
            manifest: member.source_snapshot.artifact_path.clone(),
            journal: entry.artifact_path.clone(),
        });
    }
    if let Some(manifest_hash) = member.source_snapshot.checksum.as_deref()
        && manifest_hash != expected_hash
    {
        return Err(PersistenceError::ManifestJournalChecksumMismatch {
            canister_id: entry.canister_id.clone(),
            snapshot_id: entry.snapshot_id.clone(),
            manifest: manifest_hash.to_string(),
            journal: expected_hash.to_string(),
        });
    }

    Ok(())
}
