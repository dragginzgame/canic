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
#[cfg(all(
    test,
    any(target_os = "linux", target_os = "android", target_vendor = "apple")
))]
pub(crate) use artifact_commit::{ArtifactCommitBarrier, commit_artifact_directory_at_barriers};
pub use command_lifetime_lock::CommandLifetimeHandle;
pub(crate) use command_lifetime_lock::{CommandLifetimeLock, CommandLifetimeLockError};
pub use error::PersistenceError;
pub(crate) use integrity::verify_durable_artifact;
pub use integrity::{
    ArtifactIntegrityReport, BackupExecutionIntegrityReport, BackupIntegrityReport,
    resolve_backup_artifact_path,
};
pub(crate) use journal_lock::{JournalLock, JournalLockError};
#[cfg(test)]
pub(crate) use json::{
    DurableWriteBarrier, create_json_durable_at_barriers, write_json_durable_at_barriers,
};
pub(crate) use json::{create_json_durable, read_json, write_json_durable};
pub use layout::BackupLayout;

#[cfg(test)]
mod tests;
