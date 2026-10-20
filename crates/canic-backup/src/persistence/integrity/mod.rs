//! Module: persistence::integrity
//!
//! Responsibility: verify persisted backup manifests, journals, and artifacts agree.
//! Does not own: JSON persistence, manifest construction, or restore execution.
//! Boundary: exposes integrity reports and backup artifact path resolution.

mod artifacts;
mod execution;
mod path;
mod reports;
mod topology;

pub(super) use artifacts::verify_layout_integrity;
pub(super) use execution::verify_execution_integrity;
pub use path::resolve_backup_artifact_path;
pub use reports::{ArtifactIntegrityReport, BackupExecutionIntegrityReport, BackupIntegrityReport};
