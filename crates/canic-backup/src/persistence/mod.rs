//! Module: persistence
//!
//! Responsibility: persist backup manifests, journals, plans, and integrity reports.
//! Does not own: manifest construction, journal state transitions, or backup execution.
//! Boundary: validates data before filesystem writes and before resume integrity checks.

mod artifact_commit;
mod command_lifetime_lock;
mod error;
mod file_lock;
mod integrity;
mod journal_lock;
mod json;
mod layout;

pub(crate) use artifact_commit::commit_artifact_directory;
pub use command_lifetime_lock::CommandLifetimeHandle;
pub(crate) use command_lifetime_lock::{CommandLifetimeLock, CommandLifetimeLockError};
pub use error::PersistenceError;
pub use integrity::{
    ArtifactIntegrityReport, BackupExecutionIntegrityReport, BackupIntegrityReport,
    resolve_backup_artifact_path,
};
pub(crate) use journal_lock::{JournalLock, JournalLockError};
pub(crate) use json::write_json_durable;
pub use layout::BackupLayout;

#[cfg(test)]
mod tests;
