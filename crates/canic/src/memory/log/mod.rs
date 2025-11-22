use crate::{
    Error, ThisError,
    cdk::structures::{
        DefaultMemoryImpl,
        log::{Log as StableLogImpl, WriteError},
        memory::VirtualMemory,
    },
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

///
/// LogError
///

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

///
/// LogEntry
/// (stored)
///

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

///
/// LogEntryView
/// (exported)
///

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
// StableLog
// -----------------------------------------------------------------------------

pub struct StableLog;

impl StableLog {
    // -------- Append / write --------

    /// Append a log entry and return its index.
    pub fn append(entry: LogEntry) -> Result<u64, Error> {
        LOG.with_borrow(|log| log.append(&entry))
            .map_err(LogError::from)
            .map_err(Error::from)
    }

    /// Append a level+message with no topic.
    pub fn append_line(level: Level, message: &str) -> Result<u64, Error> {
        Self::append(LogEntry::new(level, None, message))
    }

    /// Append an INFO-level message with no topic.
    pub fn append_text(message: impl AsRef<str>) -> Result<u64, Error> {
        Self::append(LogEntry::new(Level::Info, None, message.as_ref()))
    }

    /// Append an INFO-level message with a topic.
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

    /// Return the stored entry at `index`, or `None`.
    #[must_use]
    pub fn get(index: u64) -> Option<LogEntry> {
        LOG.with_borrow(|log| log.get(index))
    }

    /// Return the stored entry at `index`, or an error.
    pub fn try_get(index: u64) -> Result<LogEntry, Error> {
        Self::get(index).ok_or_else(|| LogError::EntryNotFound(index).into())
    }

    /// First stored entry (oldest).
    #[must_use]
    pub fn first() -> Option<LogEntry> {
        LOG.with_borrow(StableLogImpl::first)
    }

    /// Last stored entry (newest).
    #[must_use]
    pub fn last() -> Option<LogEntry> {
        LOG.with_borrow(StableLogImpl::last)
    }

    // -------- Pagination / views --------

    /// Page over all entries as views.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn entries_page(offset: u64, limit: u64) -> LogView {
        LOG.with_borrow(|log| {
            log.iter()
                .enumerate()
                .skip(offset as usize)
                .take(limit as usize)
                .map(|(i, entry)| LogEntryView::from_pair(i as u64, entry))
                .collect()
        })
    }

    /// Page over entries with a minimum level.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn entries_page_level(offset: u64, limit: u64, min_level: Level) -> LogView {
        LOG.with_borrow(|log| {
            log.iter()
                .enumerate()
                .filter(|(_, e)| e.level >= min_level)
                .skip(offset as usize)
                .take(limit as usize)
                .map(|(i, e)| LogEntryView::from_pair(i as u64, e))
                .collect()
        })
    }

    /// Most recent `limit` entries.
    #[must_use]
    pub fn tail(limit: u64) -> LogView {
        let len = Self::len();
        let offset = len.saturating_sub(limit);
        Self::entries_page(offset, limit)
    }

    /// Export the entire log as views.
    #[must_use]
    pub fn entries() -> LogView {
        LOG.with_borrow(|log| {
            log.iter()
                .enumerate()
                .map(|(i, e)| LogEntryView::from_pair(i as u64, e))
                .collect()
        })
    }

    /// Export entries starting from index `start` as views.
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

    // -------- Bulk transforms / maintenance --------

    /// Clear all entries below `min_level`, keep others.
    pub fn clear_below(min_level: Level) {
        let retained: Vec<LogEntry> =
            LOG.with_borrow(|log| log.iter().filter(|e| e.level >= min_level).collect());

        LOG.with_borrow_mut(|log| *log = reset_log());

        for entry in retained {
            let _ = Self::append(entry);
        }
    }

    /// Reset log entirely (wipe index + data).
    pub fn clear() {
        LOG.with_borrow_mut(|log| {
            *log = reset_log();
        });
    }

    // -------- Introspection --------

    #[must_use]
    pub fn len() -> u64 {
        LOG.with_borrow(StableLogImpl::len)
    }

    #[must_use]
    pub fn is_empty() -> bool {
        Self::len() == 0
    }

    #[must_use]
    pub fn size_bytes() -> (u64, u64) {
        LOG.with_borrow(|log| (log.index_size_bytes(), log.data_size_bytes()))
    }

    /// Convenience: collect all entries into a Vec and iterate.
    /// (This *does* allocate; it's just a helper).
    pub fn iter() -> impl Iterator<Item = LogEntry> {
        LOG.with_borrow(|log| log.iter().collect::<Vec<_>>())
            .into_iter()
    }
}
