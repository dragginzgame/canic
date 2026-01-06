use crate::{
    Error,
    log::Level,
    ops::config::ConfigOps,
    storage::stable::log::{Log, LogEntry as ModelLogEntry, RetentionSummary, apply_retention},
};

///
/// LogOps
///
/// Logging control operations.
///
/// Responsibilities:
/// - Append runtime log entries
/// - Apply retention policies
/// - Expose a point-in-time read-only view of log entries
///
/// Notes:
/// - Logs are **not authoritative domain state**
/// - Logs are **never imported or cascaded**
/// - Therefore, no `Snapshot` DTO exists for logs
///
pub struct LogOps;

#[derive(Clone, Debug)]
pub struct LogEntrySnapshot {
    pub crate_name: String,
    pub created_at: u64,
    pub level: Level,
    pub topic: Option<String>,
    pub message: String,
}

impl LogOps {
    // ---------------------------------------------------------------------
    // Mutation
    // ---------------------------------------------------------------------

    /// Append a runtime log entry.
    ///
    /// This is a side-effecting operation and must only be called
    /// from update / workflow contexts.
    pub fn append_runtime_log(
        crate_name: &str,
        topic: Option<&str>,
        level: Level,
        message: &str,
        created_at: u64,
    ) -> Result<u64, Error> {
        if !crate::log::is_ready() {
            return Ok(0);
        }

        let cfg = ConfigOps::log_config()?;

        // This is a defensive size guard to protect runtime memory; workflow may also validate, but ops enforces the hard limit.
        Log::append(&cfg, created_at, crate_name, topic, level, message)
    }

    /// Apply log retention using explicit parameters.
    ///
    /// Returns a summary describing how many entries were removed.
    pub fn apply_retention(
        cutoff: Option<u64>,
        max_entries: usize,
        max_entry_bytes: u32,
    ) -> Result<RetentionSummary, Error> {
        apply_retention(cutoff, max_entries, max_entry_bytes)
    }

    // ---------------------------------------------------------------------
    // Read-only access
    // ---------------------------------------------------------------------

    /// Return a point-in-time copy of all log entries.
    ///
    /// This is **not** a snapshot in the architectural sense:
    /// - it is never imported
    /// - it is never validated
    /// - it is never persisted elsewhere
    ///
    /// Intended for read-only querying and view adaptation.
    #[must_use]
    pub fn snapshot() -> Vec<LogEntrySnapshot> {
        Log::snapshot().into_iter().map(Into::into).collect()
    }
}

impl From<ModelLogEntry> for LogEntrySnapshot {
    fn from(entry: ModelLogEntry) -> Self {
        Self {
            crate_name: entry.crate_name,
            created_at: entry.created_at,
            level: entry.level,
            topic: entry.topic,
            message: entry.message,
        }
    }
}
