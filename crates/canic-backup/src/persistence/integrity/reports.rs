//! Module: persistence::integrity::reports
//!
//! Responsibility: define read-only backup integrity report projections.
//! Does not own: integrity verification, persistence, or restore execution.
//! Boundary: exposes serializable report DTOs for persistence callers.

use serde::{Deserialize, Serialize};

///
/// BackupIntegrityReport
///
/// Read-only integrity projection for manifest, journal, and artifact checks.
/// Owned by persistence integrity and returned by backup layout verification.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BackupIntegrityReport {
    pub backup_id: String,
    pub verified: bool,
    pub manifest_members: usize,
    pub journal_artifacts: usize,
    pub durable_artifacts: usize,
    pub artifacts: Vec<ArtifactIntegrityReport>,
}

///
/// BackupExecutionIntegrityReport
///
/// Read-only integrity projection for backup plan and execution journal checks.
/// Owned by persistence integrity and returned by execution resume verification.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BackupExecutionIntegrityReport {
    pub plan_id: String,
    pub run_id: String,
    pub verified: bool,
    pub plan_operations: usize,
    pub journal_operations: usize,
}

///
/// ArtifactIntegrityReport
///
/// Read-only integrity projection for one durable artifact.
/// Owned by persistence integrity and embedded in backup integrity reports.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactIntegrityReport {
    pub canister_id: String,
    pub snapshot_id: String,
    pub artifact_path: String,
    pub checksum: String,
}
