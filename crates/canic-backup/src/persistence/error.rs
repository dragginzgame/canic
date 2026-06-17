//! Module: persistence::error
//!
//! Responsibility: define typed persistence and integrity failures.
//! Does not own: filesystem operations, JSON encoding, or checksum calculation.
//! Boundary: aggregates lower-layer errors returned by persistence APIs.

use crate::{
    artifacts::ArtifactChecksumError, execution::BackupExecutionJournalError,
    journal::JournalValidationError, manifest::ManifestValidationError, plan::BackupPlanError,
};

use std::io;

use thiserror::Error as ThisError;

///
/// PersistenceError
///
/// Typed persistence and integrity failure for backup layout operations.
/// Owned by persistence and returned by backup file IO and verification APIs.
///

#[derive(Debug, ThisError)]
pub enum PersistenceError {
    #[error("artifact path escapes backup root: {artifact_path}")]
    ArtifactPathEscapesBackup { artifact_path: String },

    #[error("manifest backup id {manifest} does not match journal backup id {journal}")]
    BackupIdMismatch { manifest: String, journal: String },

    #[error(transparent)]
    Checksum(#[from] ArtifactChecksumError),

    #[error("backup execution operation {sequence} is {state} but has no matching receipt")]
    ExecutionOperationMissingReceipt { sequence: usize, state: String },

    #[error("backup execution operation {sequence} timestamp does not match latest receipt")]
    ExecutionOperationReceiptTimestampMismatch { sequence: usize },

    #[error(transparent)]
    InvalidBackupPlan(#[from] BackupPlanError),

    #[error(transparent)]
    InvalidExecutionJournal(#[from] BackupExecutionJournalError),

    #[error(transparent)]
    InvalidJournal(#[from] JournalValidationError),

    #[error(transparent)]
    InvalidManifest(#[from] ManifestValidationError),

    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error("manifest topology receipt {field} does not match journal topology receipt")]
    ManifestJournalTopologyReceiptMismatch {
        field: String,
        manifest: String,
        journal: Option<String>,
    },

    #[error(
        "manifest artifact path for {canister_id} snapshot {snapshot_id} does not match journal artifact path"
    )]
    ManifestJournalArtifactPathMismatch {
        canister_id: String,
        snapshot_id: String,
        manifest: String,
        journal: String,
    },

    #[error(
        "manifest checksum for {canister_id} snapshot {snapshot_id} does not match journal checksum"
    )]
    ManifestJournalChecksumMismatch {
        canister_id: String,
        snapshot_id: String,
        manifest: String,
        journal: String,
    },

    #[error("artifact path does not exist: {0}")]
    MissingArtifact(String),

    #[error("manifest member {canister_id} snapshot {snapshot_id} has no journal artifact")]
    MissingJournalArtifact {
        canister_id: String,
        snapshot_id: String,
    },

    #[error("journal artifact {canister_id} snapshot {snapshot_id} has no checksum")]
    MissingJournalArtifactChecksum {
        canister_id: String,
        snapshot_id: String,
    },

    #[error("journal artifact {canister_id} snapshot {snapshot_id} is not durable")]
    NonDurableArtifact {
        canister_id: String,
        snapshot_id: String,
    },

    #[error("backup plan {field} does not match execution journal")]
    PlanJournalMismatch {
        field: &'static str,
        plan: String,
        journal: String,
    },

    #[error("backup plan operation {sequence} {field} does not match execution journal")]
    PlanJournalOperationMismatch {
        sequence: usize,
        field: &'static str,
        plan: String,
        journal: String,
    },

    #[error("journal artifact {canister_id} snapshot {snapshot_id} is not declared in manifest")]
    UnexpectedJournalArtifact {
        canister_id: String,
        snapshot_id: String,
    },
}
