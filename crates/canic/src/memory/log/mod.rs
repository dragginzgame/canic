use crate::{
    Error, ThisError,
    cdk::structures::{
        DefaultMemoryImpl,
        log::{Log as StableLogImpl, WriteError},
        memory::VirtualMemory,
    },
    config::{Config, model::LogConfig},
    eager_static, ic_memory, impl_storable_unbounded,
    log::Level,
    memory::{
        MemoryError,
        id::log::{LOG_DATA_ID, LOG_INDEX_ID},
    },
    utils::time,
};
use candid::CandidType;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

// -----------------------------------------------------------------------------
// Stable log storage
// -----------------------------------------------------------------------------

type StableLogStorage =
    StableLogImpl<LogEntry, VirtualMemory<DefaultMemoryImpl>, VirtualMemory<DefaultMemoryImpl>>;

struct LogIndexMemory;
struct LogDataMemory;

eager_static! {
    static LOG: RefCell<StableLogStorage> = RefCell::new(init_log());
}

fn init_log() -> StableLogStorage {
    StableLogImpl::init(
        ic_memory!(LogIndexMemory, LOG_INDEX_ID),
        ic_memory!(LogDataMemory, LOG_DATA_ID),
    )
}

fn reset_log() -> StableLogStorage {
    StableLogImpl::new(
        ic_memory!(LogIndexMemory, LOG_INDEX_ID),
        ic_memory!(LogDataMemory, LOG_DATA_ID),
    )
}

fn log_config() -> LogConfig {
    Config::try_get()
        .map(|cfg| cfg.log.clone())
        .unwrap_or_default()
}

/// Removes old entries by age or count.
/// Everything uses *seconds only* (`created_at` and `now_secs()`).
#[allow(clippy::cast_possible_truncation)]
fn apply_retention(cfg: &LogConfig) -> Result<(), Error> {
    if cfg.max_entries == 0 {
        LOG.with_borrow_mut(|log| *log = reset_log());
        return Ok(());
    }

    let now_secs = time::now_secs();
    let max_entries = cfg.max_entries.try_into().unwrap_or(usize::MAX);

    // Collect all entries
    let mut retained: Vec<LogEntry> = LOG.with_borrow(|log| log.iter().collect());

    // Apply max age (seconds)
    if let Some(age_limit) = cfg.max_age_secs {
        retained.retain(|entry| now_secs.saturating_sub(entry.created_at) <= age_limit);
    }

    // Apply max number of entries
    if retained.len() > max_entries {
        let drop = retained.len() - max_entries;
        retained.drain(0..drop);
    }

    // If nothing changed, skip rewrite
    if retained.len() == LOG.with_borrow(|l| l.len() as usize) {
        return Ok(());
    }

    // Rewrite log
    LOG.with_borrow_mut(|log| *log = reset_log());

    for entry in retained {
        LOG.with_borrow(|log| log.append(&entry))
            .map_err(LogError::from)
            .map_err(Error::from)?;
    }

    Ok(())
}

// -----------------------------------------------------------------------------
// LogError
// -----------------------------------------------------------------------------

#[derive(Debug, ThisError)]
pub enum LogError {
    #[error("log entry not found at index {0}")]
    EntryNotFound(u64),

    #[error("log write failed: current_size={current_size}, delta={delta}")]
    WriteFailed { current_size: u64, delta: u64 },
}

impl From<WriteError> for LogError {
    fn from(err: WriteError) -> Self {
        let WriteError::GrowFailed {
            current_size,
            delta,
        } = err;
        Self::WriteFailed {
            current_size,
            delta,
        }
    }
}

impl From<LogError> for Error {
    fn from(err: LogError) -> Self {
        MemoryError::from(err).into()
    }
}

// -----------------------------------------------------------------------------
// LogEntry
// -----------------------------------------------------------------------------

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct LogEntry {
    pub created_at: u64,
    pub level: Level,
    pub topic: Option<String>,
    pub message: String,
}

impl LogEntry {
    #[must_use]
    pub fn new(level: Level, topic: Option<&str>, message: &str) -> Self {
        Self {
            created_at: time::now_secs(),
            level,
            topic: topic.map(ToString::to_string),
            message: message.to_string(),
        }
    }
}

impl_storable_unbounded!(LogEntry);

// -----------------------------------------------------------------------------
// LogEntryView
// -----------------------------------------------------------------------------

#[derive(CandidType, Debug, Clone, Serialize)]
pub struct LogEntryView {
    pub index: u64,
    pub created_at: u64,
    pub level: Level,
    pub topic: Option<String>,
    pub message: String,
}

impl LogEntryView {
    fn from_pair(index: u64, entry: LogEntry) -> Self {
        Self {
            index,
            created_at: entry.created_at,
            level: entry.level,
            topic: entry.topic,
            message: entry.message,
        }
    }
}

pub type LogView = Vec<LogEntryView>;

// -----------------------------------------------------------------------------
// StableLog API
// -----------------------------------------------------------------------------

pub struct StableLog;

impl StableLog {
    // -------- Append / write --------

    pub fn append(entry: LogEntry) -> Result<u64, Error> {
        let cfg = log_config();

        if cfg.max_entries == 0 {
            return Ok(0);
        }

        apply_retention(&cfg)?;

        LOG.with_borrow(|log| log.append(&entry))
            .map_err(LogError::from)
            .map_err(Error::from)
    }

    pub fn append_line(level: Level, message: &str) -> Result<u64, Error> {
        Self::append(LogEntry::new(level, None, message))
    }

    pub fn append_text(message: impl AsRef<str>) -> Result<u64, Error> {
        Self::append(LogEntry::new(Level::Info, None, message.as_ref()))
    }

    pub fn append_text_with_topic(
        topic: impl AsRef<str>,
        message: impl AsRef<str>,
    ) -> Result<u64, Error> {
        Self::append(LogEntry::new(
            Level::Info,
            Some(topic.as_ref()),
            message.as_ref(),
        ))
    }

    // -------- Single-entry read --------

    #[must_use]
    pub fn get(index: u64) -> Option<LogEntry> {
        LOG.with_borrow(|log| log.get(index))
    }

    pub fn try_get(index: u64) -> Result<LogEntry, Error> {
        Self::get(index).ok_or_else(|| LogError::EntryNotFound(index).into())
    }

    pub fn first() -> Option<LogEntry> {
        LOG.with_borrow(StableLogImpl::first)
    }

    pub fn last() -> Option<LogEntry> {
        LOG.with_borrow(StableLogImpl::last)
    }

    // -------- Pagination / views --------

    fn page_iter(
        log: &StableLogStorage,
        offset: usize,
        limit: usize,
    ) -> impl Iterator<Item = (usize, LogEntry)> + '_ {
        log.iter().enumerate().skip(offset).take(limit)
    }

    #[must_use]
    pub fn entries_page(offset: u64, limit: u64) -> LogView {
        let offset = usize::try_from(offset).unwrap_or(usize::MAX);
        let limit = usize::try_from(limit).unwrap_or(usize::MAX);

        LOG.with_borrow(|log| {
            Self::page_iter(log, offset, limit)
                .map(|(i, entry)| LogEntryView::from_pair(i as u64, entry))
                .collect()
        })
    }

    #[must_use]
    pub fn entries_page_level(offset: u64, limit: u64, min_level: Level) -> LogView {
        let offset = usize::try_from(offset).unwrap_or(usize::MAX);
        let limit = usize::try_from(limit).unwrap_or(usize::MAX);

        LOG.with_borrow(|log| {
            Self::page_iter(log, offset, limit)
                .filter(|(_, e)| e.level >= min_level)
                .map(|(i, e)| LogEntryView::from_pair(i as u64, e))
                .collect()
        })
    }

    #[must_use]
    pub fn tail(limit: u64) -> LogView {
        let len = Self::len();
        let offset = len.saturating_sub(limit);
        Self::entries_page(offset, limit)
    }

    #[must_use]
    pub fn entries() -> LogView {
        LOG.with_borrow(|log| {
            log.iter()
                .enumerate()
                .map(|(i, e)| LogEntryView::from_pair(i as u64, e))
                .collect()
        })
    }

    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn entries_from(start: u64) -> LogView {
        LOG.with_borrow(|log| {
            log.iter()
                .enumerate()
                .skip(start as usize)
                .map(|(i, e)| LogEntryView::from_pair(i as u64, e))
                .collect()
        })
    }

    // -------- Bulk transforms --------

    pub fn clear_below(min_level: Level) {
        let retained: Vec<LogEntry> =
            LOG.with_borrow(|log| log.iter().filter(|e| e.level >= min_level).collect());

        LOG.with_borrow_mut(|log| *log = reset_log());

        for entry in retained {
            let _ = Self::append(entry);
        }
    }

    pub fn clear() {
        LOG.with_borrow_mut(|log| *log = reset_log());
    }

    // -------- Introspection --------

    pub fn len() -> u64 {
        LOG.with_borrow(StableLogImpl::len)
    }

    /// Count entries at or above `min_level`.
    #[must_use]
    pub fn len_level(min_level: Level) -> u64 {
        LOG.with_borrow(|log| log.iter().filter(|entry| entry.level >= min_level).count() as u64)
    }

    #[must_use]
    pub fn is_empty() -> bool {
        Self::len() == 0
    }

    #[must_use]
    pub fn size_bytes() -> (u64, u64) {
        LOG.with_borrow(|log| (log.index_size_bytes(), log.data_size_bytes()))
    }

    pub fn iter() -> impl Iterator<Item = LogEntry> {
        LOG.with_borrow(|log| log.iter().collect::<Vec<_>>())
            .into_iter()
    }
}
