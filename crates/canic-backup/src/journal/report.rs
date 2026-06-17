//! Module: journal::report
//!
//! Responsibility: project durable journal state into resumability reports.
//! Does not own: journal validation, artifact state mutation, or persistence.
//! Boundary: reads journal records and returns read-only reporting projections.

use crate::journal::{ArtifactState, DownloadJournal, DownloadOperationMetrics, ResumeAction};

use serde::{Deserialize, Serialize};

impl DownloadJournal {
    /// Build a resumability report from the current journal state.
    #[must_use]
    pub fn resume_report(&self) -> JournalResumeReport {
        let mut counts = JournalStateCounts::default();
        let mut artifacts = Vec::with_capacity(self.artifacts.len());

        for artifact in &self.artifacts {
            counts.record(artifact.state, artifact.resume_action());
            artifacts.push(ArtifactResumeReport {
                canister_id: artifact.canister_id.clone(),
                snapshot_id: artifact.snapshot_id.clone(),
                state: artifact.state,
                resume_action: artifact.resume_action(),
                artifact_path: artifact.artifact_path.clone(),
                temp_path: artifact.temp_path.clone(),
                updated_at: artifact.updated_at.clone(),
            });
        }

        JournalResumeReport {
            backup_id: self.backup_id.clone(),
            discovery_topology_hash: self.discovery_topology_hash.clone(),
            pre_snapshot_topology_hash: self.pre_snapshot_topology_hash.clone(),
            total_artifacts: self.artifacts.len(),
            is_complete: counts.skip == self.artifacts.len(),
            pending_artifacts: self.artifacts.len() - counts.skip,
            counts,
            operation_metrics: self.operation_metrics.clone(),
            artifacts,
        }
    }
}

///
/// JournalResumeReport
///
/// Read-only resume projection of one download journal.
/// Owned by backup journaling and consumed by status/reporting surfaces.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct JournalResumeReport {
    pub backup_id: String,
    pub discovery_topology_hash: Option<String>,
    pub pre_snapshot_topology_hash: Option<String>,
    pub total_artifacts: usize,
    pub is_complete: bool,
    pub pending_artifacts: usize,
    pub counts: JournalStateCounts,
    pub operation_metrics: DownloadOperationMetrics,
    pub artifacts: Vec<ArtifactResumeReport>,
}

///
/// JournalStateCounts
///
/// Aggregated artifact state and resume-action counters.
/// Owned by backup journaling and embedded in resume reports.
///

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct JournalStateCounts {
    pub created: usize,
    pub downloaded: usize,
    pub checksum_verified: usize,
    pub durable: usize,
    pub download: usize,
    pub verify_checksum: usize,
    pub finalize: usize,
    pub skip: usize,
}

impl JournalStateCounts {
    const fn record(&mut self, state: ArtifactState, action: ResumeAction) {
        match state {
            ArtifactState::Created => self.created += 1,
            ArtifactState::Downloaded => self.downloaded += 1,
            ArtifactState::ChecksumVerified => self.checksum_verified += 1,
            ArtifactState::Durable => self.durable += 1,
        }

        match action {
            ResumeAction::Download => self.download += 1,
            ResumeAction::VerifyChecksum => self.verify_checksum += 1,
            ResumeAction::Finalize => self.finalize += 1,
            ResumeAction::Skip => self.skip += 1,
        }
    }
}

///
/// ArtifactResumeReport
///
/// Read-only resume projection for one artifact journal entry.
/// Owned by backup journaling and embedded in resume reports.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ArtifactResumeReport {
    pub canister_id: String,
    pub snapshot_id: String,
    pub state: ArtifactState,
    pub resume_action: ResumeAction,
    pub artifact_path: String,
    pub temp_path: Option<String>,
    pub updated_at: String,
}
