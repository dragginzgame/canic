//! Module: persistence
//!
//! Responsibility: persist backup manifests, journals, plans, and integrity reports.
//! Does not own: manifest construction, journal state transitions, or backup execution.
//! Boundary: validates data before filesystem writes and before resume integrity checks.

mod command_custody;
mod error;
mod integrity;
mod json;
mod layout;

pub(crate) use command_custody::acquire_recorded_command;
pub use error::PersistenceError;
pub use ic_backup::ops::persistence::BackupLayoutGuard;
pub use ic_backup::ops::persistence::{
    CommandLifetimeLock, CommandLifetimeLockError, CommandQuiescenceGuard,
};
pub use ic_backup::ops::persistence::{JournalLock, JournalLockError};
pub use integrity::{
    ArtifactIntegrityReport, BackupExecutionIntegrityReport, BackupIntegrityReport,
    resolve_backup_artifact_path,
};
#[cfg(test)]
pub(crate) use json::{
    DurableWriteBarrier, create_json_durable_at_barriers, write_json_durable_at_barriers,
};
pub use json::{JSON_RECORD_MAX_BYTES, read_json};
pub(crate) use json::{create_json_durable, write_json_durable};
pub use layout::BackupExecutionGuard;
pub use layout::BackupLayout;

#[cfg(test)]
mod tests;
