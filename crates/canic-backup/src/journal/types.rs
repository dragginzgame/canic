//! Module: journal::types
//!
//! Responsibility: define durable artifact journal state and transitions.
//! Does not own: persistence, resume reporting, or full journal validation.
//! Boundary: exposes typed journal records consumed by backup workflows.

use crate::journal::JournalValidationError;

use serde::{Deserialize, Serialize};

///
/// DownloadJournal
///
/// Durable artifact download journal for one backup run.
/// Owned by backup journaling and consumed by resume and integrity checks.
///

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DownloadJournal {
    pub journal_version: u16,
    pub backup_id: String,
    pub discovery_topology_hash: String,
    pub pre_snapshot_topology_hash: String,
    pub operation_metrics: DownloadOperationMetrics,
    pub artifacts: Vec<ArtifactJournalEntry>,
}

///
/// DownloadOperationMetrics
///
/// Counters for artifact download lifecycle progress.
/// Owned by backup journaling and reported in resume summaries.
///

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DownloadOperationMetrics {
    pub target_count: usize,
    pub snapshot_create_started: usize,
    pub snapshot_create_completed: usize,
    pub snapshot_download_started: usize,
    pub snapshot_download_completed: usize,
    pub checksum_verify_started: usize,
    pub checksum_verify_completed: usize,
    pub artifact_finalize_started: usize,
    pub artifact_finalize_completed: usize,
}

///
/// ArtifactJournalEntry
///
/// Durable journal entry for one snapshot artifact.
/// Owned by backup journaling and advanced by snapshot download workflows.
///

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactJournalEntry {
    pub canister_id: String,
    pub snapshot_id: String,
    #[serde(deserialize_with = "crate::serialization::required_option")]
    pub snapshot_taken_at_timestamp: Option<u64>,
    #[serde(deserialize_with = "crate::serialization::required_option")]
    pub snapshot_total_size_bytes: Option<u64>,
    pub state: ArtifactState,
    #[serde(deserialize_with = "crate::serialization::required_option")]
    pub temp_path: Option<String>,
    pub artifact_path: String,
    pub checksum_algorithm: String,
    #[serde(deserialize_with = "crate::serialization::required_option")]
    pub checksum: Option<String>,
    pub updated_at: String,
}

impl ArtifactJournalEntry {
    /// Return the idempotent action needed to resume this artifact.
    #[must_use]
    pub const fn resume_action(&self) -> ResumeAction {
        match self.state {
            ArtifactState::Created => ResumeAction::Download,
            ArtifactState::Downloaded => ResumeAction::VerifyChecksum,
            ArtifactState::ChecksumVerified => ResumeAction::Finalize,
            ArtifactState::Durable => ResumeAction::Skip,
        }
    }

    /// Advance this artifact through the next canonical journal state.
    pub fn advance_to(
        &mut self,
        next_state: ArtifactState,
        updated_at: String,
    ) -> Result<(), JournalValidationError> {
        if !self.state.can_advance_to(next_state) {
            return Err(JournalValidationError::InvalidStateTransition {
                from: self.state,
                to: next_state,
            });
        }

        self.state = next_state;
        self.updated_at = updated_at;
        Ok(())
    }
}

///
/// ArtifactState
///
/// Ordered durable state for one snapshot artifact.
/// Owned by backup journaling and used to derive idempotent resume actions.
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum ArtifactState {
    Created,
    Downloaded,
    ChecksumVerified,
    Durable,
}

impl ArtifactState {
    /// Return whether `next` is the canonical immediate successor.
    #[must_use]
    pub const fn can_advance_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Created, Self::Downloaded)
                | (Self::Downloaded, Self::ChecksumVerified)
                | (Self::ChecksumVerified, Self::Durable)
        )
    }
}

///
/// ResumeAction
///
/// Next idempotent action needed to resume an artifact download.
/// Owned by backup journaling and derived from artifact state.
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ResumeAction {
    Download,
    VerifyChecksum,
    Finalize,
    Skip,
}
