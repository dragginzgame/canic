use crate::{
    Error,
    cdk::utils::time::now_secs,
    log::Level,
    ops::config::ConfigOps,
    storage::memory::log::{Log, LogEntry as ModelLogEntry, RetentionSummary, apply_retention},
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
    ) -> Result<u64, Error> {
        if !crate::log::is_ready() {
            return Ok(0);
        }

        let cfg = ConfigOps::log_config()?;
        let now = now_secs();

        Log::append(&cfg, now, crate_name, topic, level, message)
    }

    /// Apply the configured log retention policy.
    ///
    /// Returns a summary describing how many entries were removed.
    pub fn apply_retention() -> Result<RetentionSummary, Error> {
        let cfg = ConfigOps::log_config()?;
        let now = now_secs();

        let max_entries = usize::try_from(cfg.max_entries).unwrap_or(usize::MAX);
        let cutoff = cfg.max_age_secs.map(|max_age| now.saturating_sub(max_age));

        apply_retention(cutoff, max_entries, cfg.max_entry_bytes)
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
