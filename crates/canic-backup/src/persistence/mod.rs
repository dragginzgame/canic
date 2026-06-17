//! Module: persistence
//!
//! Responsibility: persist backup manifests, journals, plans, and integrity reports.
//! Does not own: manifest construction, journal state transitions, or backup execution.
//! Boundary: validates data before filesystem writes and before resume integrity checks.

mod error;
mod integrity;
mod json;
mod layout;

pub use error::PersistenceError;
pub use integrity::{
    ArtifactIntegrityReport, BackupExecutionIntegrityReport, BackupIntegrityReport,
    resolve_backup_artifact_path,
};
pub use layout::BackupLayout;

#[cfg(test)]
mod tests;
