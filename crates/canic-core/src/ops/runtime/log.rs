use crate::{
    InternalError,
    dto::{
        log::LogEntry,
        page::{Page, PageRequest},
    },
    log::{Level, Topic},
    ops::{config::ConfigOps, runtime::RuntimeOpsError},
    storage::{
        StorageError,
        stable::log::{Log, LogEntryRecord, RetentionSummary, apply_retention},
    },
};
use thiserror::Error as ThisError;

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
        topic: Option<Topic>,
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
            level,
            topic,
            message: message.to_string(),
        };

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

    /// Build a filtered point-in-time log view before DTO projection.
    ///
    /// This avoids allocating topic strings and DTO rows for entries that the
    /// query layer will immediately discard.
    #[must_use]
    pub fn page_filtered(
        crate_name: Option<&str>,
        topic: Option<&str>,
        min_level: Option<Level>,
        page: PageRequest,
    ) -> Page<LogEntry> {
        let mut entries = Vec::new();
        let mut total = 0u64;
        let offset = page.offset;
        let limit = page.limit.min(1_000);

        for entry in Log::snapshot().into_iter().rev() {
            if !record_matches(&entry, crate_name, topic, min_level) {
                continue;
            }

            if total >= offset && (entries.len() as u64) < limit {
                entries.push(record_to_entry(entry));
            }

            total = total.saturating_add(1);
        }

        Page { entries, total }
    }
}

// Convert a stored log record into the public query DTO.
fn record_to_entry(entry: LogEntryRecord) -> LogEntry {
    LogEntry {
        crate_name: entry.crate_name,
        created_at: entry.created_at,
        level: entry.level,
        topic: entry.topic.map(|topic| topic.log_label().to_string()),
        message: entry.message,
    }
}

// Apply the public log filters to a stored log record.
fn record_matches(
    entry: &LogEntryRecord,
    crate_name: Option<&str>,
    topic: Option<&str>,
    min_level: Option<Level>,
) -> bool {
    crate_name.is_none_or(|name| entry.crate_name == name)
        && topic.is_none_or(|needle| topic_matches(entry.topic, needle))
        && min_level.is_none_or(|min| entry.level >= min)
}

// Compare an optional stored topic against the query filter label.
fn topic_matches(topic: Option<Topic>, needle: &str) -> bool {
    topic.is_some_and(|topic| topic.log_label() == needle)
}
