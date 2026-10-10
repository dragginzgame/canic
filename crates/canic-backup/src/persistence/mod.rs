//! Module: persistence
//!
//! Responsibility: persist backup manifests, journals, plans, and integrity reports.
//! Does not own: manifest construction, journal state transitions, or backup execution.
//! Boundary: validates data before filesystem writes and before resume integrity checks.

mod command_lifetime_lock;
mod error;
mod file_lock;
mod integrity;
mod journal_lock;
mod json;
mod layout;
mod layout_lifetime;

pub use command_lifetime_lock::CommandLifetimeHandle;
pub(crate) use command_lifetime_lock::{CommandLifetimeLock, CommandLifetimeLockError};
pub use error::PersistenceError;
pub(crate) use ic_backup::ops::persistence::commit_artifact_directory;
pub(crate) use integrity::verify_durable_artifact;
pub use integrity::{
    ArtifactIntegrityReport, BackupExecutionIntegrityReport, BackupIntegrityReport,
    resolve_backup_artifact_path,
};
pub use journal_lock::{JournalLock, JournalLockError};
#[cfg(test)]
pub(crate) use json::{
    DurableWriteBarrier, create_json_durable_at_barriers, write_json_durable_at_barriers,
};
pub(crate) use json::{create_json_durable, read_json, write_json_durable};
pub use layout::BackupLayout;
pub use layout_lifetime::{BackupExecutionGuard, BackupLayoutGuard};

#[cfg(test)]
mod tests;
