//! Module: persistence
//!
//! Responsibility: persist backup manifests, journals, plans, and integrity reports.
//! Does not own: manifest construction, journal state transitions, or backup execution.
//! Boundary: validates data before filesystem writes and before resume integrity checks.

mod artifact_commit;
mod error;
mod integrity;
mod json;
mod layout;

pub(crate) use artifact_commit::commit_artifact_directory;
pub use error::PersistenceError;
pub use integrity::{
    ArtifactIntegrityReport, BackupExecutionIntegrityReport, BackupIntegrityReport,
    resolve_backup_artifact_path,
};
pub(crate) use json::write_json_durable;
pub use layout::BackupLayout;

#[cfg(test)]
mod tests;
