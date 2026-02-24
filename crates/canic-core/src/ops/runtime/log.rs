use crate::{
    InternalError,
    dto::log::LogEntry,
    log::Level,
    ops::{config::ConfigOps, runtime::RuntimeOpsError},
    storage::{
        StorageError,
        stable::log::{Log, LogEntryRecord, LogLevelRecord, RetentionSummary, apply_retention},
    },
    utils::case::{Case, Casing},
};
use thiserror::Error as ThisError;

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

///
/// LogOpsError
///

#[derive(Debug, ThisError)]
pub enum LogOpsError {
    #[error(transparent)]
    Storage(#[from] StorageError),
}

impl From<LogOpsError> for InternalError {
    fn from(err: LogOpsError) -> Self {
        RuntimeOpsError::LogOps(err).into()
    }
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
    ) -> Result<u64, InternalError> {
        if !crate::log::is_ready() {
            return Ok(0);
        }

        let cfg = ConfigOps::log_config()?;
        let max_entries = usize::try_from(cfg.max_entries).unwrap_or(usize::MAX);

        let entry = LogEntryRecord {
            crate_name: crate_name.to_string(),
            created_at,
            level: level_to_record(level),
            topic: normalize_topic(topic),
            message: message.to_string(),
        };

        // This is a defensive size guard to protect runtime memory; workflow may also validate, but ops enforces the hard limit.
        let id = Log::append(max_entries, cfg.max_entry_bytes, entry).map_err(LogOpsError::from)?;

        Ok(id)
    }

    /// Apply log retention using explicit parameters.
    ///
    /// Returns a summary describing how many entries were removed.
    pub fn apply_retention(
        cutoff: Option<u64>,
        max_entries: usize,
        max_entry_bytes: u32,
    ) -> Result<RetentionSummary, InternalError> {
        let summary =
            apply_retention(cutoff, max_entries, max_entry_bytes).map_err(LogOpsError::from)?;

        Ok(summary)
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
    pub fn snapshot() -> Vec<LogEntry> {
        Log::snapshot()
            .into_iter()
            .map(|entry| LogEntry {
                crate_name: entry.crate_name,
                created_at: entry.created_at,
                level: record_to_level(entry.level),
                topic: entry.topic,
                message: entry.message,
            })
            .collect()
    }
}

const fn level_to_record(level: Level) -> LogLevelRecord {
    match level {
        Level::Debug => LogLevelRecord::Debug,
        Level::Info => LogLevelRecord::Info,
        Level::Ok => LogLevelRecord::Ok,
        Level::Warn => LogLevelRecord::Warn,
        Level::Error => LogLevelRecord::Error,
    }
}

const fn record_to_level(level: LogLevelRecord) -> Level {
    match level {
        LogLevelRecord::Debug => Level::Debug,
        LogLevelRecord::Info => Level::Info,
        LogLevelRecord::Ok => Level::Ok,
        LogLevelRecord::Warn => Level::Warn,
        LogLevelRecord::Error => Level::Error,
    }
}

#[expect(clippy::single_option_map)]
fn normalize_topic(topic: Option<&str>) -> Option<String> {
    topic.map(|t| t.to_string().to_case(Case::Snake))
}
