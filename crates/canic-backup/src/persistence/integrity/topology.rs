//! Module: persistence::integrity::topology
//!
//! Responsibility: verify manifest and journal topology receipts agree.
//! Does not own: artifact checksums, execution integrity, or path resolution.
//! Boundary: returns typed persistence errors for topology receipt drift.

use crate::{
    journal::DownloadJournal, manifest::DeploymentBackupManifest, persistence::PersistenceError,
};

use serde::{Deserialize, Serialize};

///
/// TopologyReceiptMismatch
///
/// Internal mismatch projection for one manifest/journal topology receipt.
/// Owned by persistence integrity and converted into typed persistence errors.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct TopologyReceiptMismatch {
    field: String,
    manifest: String,
    journal: Option<String>,
}

pub(super) fn verify_manifest_journal_binding(
    manifest: &DeploymentBackupManifest,
    journal: &DownloadJournal,
) -> Result<(), PersistenceError> {
    if manifest.backup_id != journal.backup_id {
        return Err(PersistenceError::BackupIdMismatch {
            manifest: manifest.backup_id.clone(),
            journal: journal.backup_id.clone(),
        });
    }

    if let Some(mismatch) = topology_receipt_mismatches(manifest, journal)
        .into_iter()
        .next()
    {
        return Err(PersistenceError::ManifestJournalTopologyReceiptMismatch {
            field: mismatch.field,
            manifest: mismatch.manifest,
            journal: mismatch.journal,
        });
    }

    Ok(())
}

fn topology_receipt_mismatches(
    manifest: &DeploymentBackupManifest,
    journal: &DownloadJournal,
) -> Vec<TopologyReceiptMismatch> {
    let mut mismatches = Vec::new();
    record_topology_receipt_mismatch(
        &mut mismatches,
        "discovery_topology_hash",
        &manifest.deployment.discovery_topology_hash,
        journal.discovery_topology_hash.as_deref(),
    );
    record_topology_receipt_mismatch(
        &mut mismatches,
        "pre_snapshot_topology_hash",
        &manifest.deployment.pre_snapshot_topology_hash,
        journal.pre_snapshot_topology_hash.as_deref(),
    );
    mismatches
}

fn record_topology_receipt_mismatch(
    mismatches: &mut Vec<TopologyReceiptMismatch>,
    field: &str,
    manifest: &str,
    journal: Option<&str>,
) {
    if journal == Some(manifest) {
        return;
    }

    mismatches.push(TopologyReceiptMismatch {
        field: field.to_string(),
        manifest: manifest.to_string(),
        journal: journal.map(ToString::to_string),
    });
}
