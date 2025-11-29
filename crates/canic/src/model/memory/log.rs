#![allow(clippy::cast_possible_truncation)]
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
    model::{
        ModelError,
        memory::{
            MemoryError,
            id::log::{LOG_DATA_ID, LOG_INDEX_ID},
        },
    },
    utils::{
        case::{Case, Casing},
        time,
    },
};
use candid::CandidType;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

// -----------------------------------------------------------------------------
// Stable Log Storage (ic-stable-structures)
// -----------------------------------------------------------------------------

type StableLogStorage =
    StableLogImpl<LogEntry, VirtualMemory<DefaultMemoryImpl>, VirtualMemory<DefaultMemoryImpl>>;

// Marker structs for ic_memory! macro
struct LogIndexMemory;
struct LogDataMemory;

fn create_log() -> StableLogStorage {
    StableLogImpl::new(
        ic_memory!(LogIndexMemory, LOG_INDEX_ID),
        ic_memory!(LogDataMemory, LOG_DATA_ID),
    )
}

eager_static! {
    static LOG: RefCell<StableLogStorage> = RefCell::new(create_log());
}

// Small helpers for readability
fn with_log<R>(f: impl FnOnce(&StableLogStorage) -> R) -> R {
    LOG.with_borrow(|l| f(l))
}

fn with_log_mut<R>(f: impl FnOnce(&mut StableLogStorage) -> R) -> R {
    LOG.with_borrow_mut(|l| f(l))
}

fn log_config() -> LogConfig {
    Config::try_get().map(|c| c.log.clone()).unwrap_or_default()
}

// -----------------------------------------------------------------------------
// Retention
// -----------------------------------------------------------------------------

fn apply_retention(cfg: &LogConfig) -> Result<(), Error> {
    if cfg.max_entries == 0 {
        with_log_mut(|log| *log = create_log());
        return Ok(());
    }

    let now = time::now_secs();
    let max_entries = cfg.max_entries as usize;

    // Load all entries once
    let mut retained: Vec<LogEntry> = with_log(|log| log.iter().collect());

    // Age filter (seconds)
    if let Some(age) = cfg.max_age_secs {
        retained.retain(|e| now.saturating_sub(e.created_at) <= age);
    }

    // Count filter
    if retained.len() > max_entries {
        let drop = retained.len() - max_entries;
        retained.drain(0..drop);
    }

    // Detect if unchanged — skip rewrite
    let original_len = with_log(|log| log.len() as usize);
    if retained.len() == original_len {
        return Ok(());
    }

    // Rewrite
    with_log_mut(|log| *log = create_log());
    for entry in retained {
        with_log(|log| log.append(&entry))
            .map_err(LogError::from)
            .map_err(Error::from)?;
    }

    Ok(())
}

// -----------------------------------------------------------------------------
// Errors
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
        match err {
            WriteError::GrowFailed {
                current_size,
                delta,
            } => Self::WriteFailed {
                current_size,
                delta,
            },
        }
    }
}

impl From<LogError> for Error {
    fn from(err: LogError) -> Self {
        ModelError::MemoryError(MemoryError::from(err)).into()
    }
}

// -----------------------------------------------------------------------------
// LogEntry
// -----------------------------------------------------------------------------

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct LogEntry {
    pub crate_name: String,
    pub created_at: u64,
    pub level: Level,
    pub topic: Option<String>,
    pub message: String,
}

impl LogEntry {
    pub fn new(crate_name: &str, level: Level, topic: Option<&str>, msg: &str) -> Self {
        Self {
            crate_name: crate_name.to_string(),
            created_at: time::now_secs(),
            level,
            topic: topic.map(ToString::to_string),
            message: msg.to_string(),
        }
    }
}

impl_storable_unbounded!(LogEntry);

// -----------------------------------------------------------------------------
// LogEntryView
// -----------------------------------------------------------------------------

#[derive(CandidType, Clone, Debug, Serialize)]
pub struct LogEntryView {
    pub index: u64,
    pub created_at: u64,
    pub crate_name: String,
    pub level: Level,
    pub topic: Option<String>,
    pub message: String,
}

impl LogEntryView {
    fn from_pair(index: usize, entry: LogEntry) -> Self {
        Self {
            index: index as u64,
            created_at: entry.created_at,
            crate_name: entry.crate_name,
            level: entry.level,
            topic: entry.topic,
            message: entry.message,
        }
    }
}

pub type LogView = Vec<LogEntryView>;

// -----------------------------------------------------------------------------
// Core filtering iterator
// -----------------------------------------------------------------------------

fn iter_filtered<'a>(
    log: &'a StableLogStorage,
    crate_name: Option<&'a str>, // this is optional
    topic: Option<&'a str>,      // optional
    min_level: Option<Level>,    // optional
) -> impl Iterator<Item = (usize, LogEntry)> + 'a {
    log.iter().enumerate().filter(move |(_, e)| {
        crate_name.is_none_or(|name| e.crate_name == name)
            && topic.is_none_or(|t| e.topic.as_deref() == Some(t))
            && min_level.is_none_or(|lvl| e.level >= lvl)
    })
}

// -----------------------------------------------------------------------------
// StableLog API
// -----------------------------------------------------------------------------

pub struct StableLog;

impl StableLog {
    // -------- Append --------

    pub fn append<T, M>(
        crate_name: &str,
        topic: Option<T>,
        level: Level,
        message: M,
    ) -> Result<u64, Error>
    where
        T: ToString,
        M: AsRef<str>,
    {
        let topic_normalized = Self::normalize_topic(topic);
        let entry = LogEntry::new(
            crate_name,
            level,
            topic_normalized.as_deref(),
            message.as_ref(),
        );

        Self::append_entry(entry)
    }

    pub fn append_entry(entry: LogEntry) -> Result<u64, Error> {
        let cfg = log_config();

        if cfg.max_entries == 0 {
            return Ok(0);
        }

        apply_retention(&cfg)?;
        with_log(|log| log.append(&entry))
            .map_err(LogError::from)
            .map_err(Error::from)
    }

    // -------- Helper -----------

    fn normalize_topic<T: ToString>(topic: Option<T>) -> Option<String> {
        topic.as_ref().map(|t| t.to_string().to_case(Case::Snake))
    }

    // -------- Single reads --------

    #[must_use]
    pub fn get(index: u64) -> Option<LogEntry> {
        with_log(|log| log.get(index))
    }

    pub fn try_get(index: u64) -> Result<LogEntry, Error> {
        Self::get(index).ok_or_else(|| LogError::EntryNotFound(index).into())
    }

    #[must_use]
    pub fn first() -> Option<LogEntry> {
        with_log(StableLogImpl::first)
    }

    #[must_use]
    pub fn last() -> Option<LogEntry> {
        with_log(StableLogImpl::last)
    }

    // -------- Pagination --------

    #[must_use]
    pub fn entries_page(offset: u64, limit: u64) -> LogView {
        Self::entries_page_filtered(None, None, None, offset, limit).0
    }

    #[must_use]
    pub fn entries_page_filtered(
        crate_name: Option<&str>,
        topic: Option<&str>,
        min_level: Option<Level>,
        offset: u64,
        limit: u64,
    ) -> (LogView, u64) {
        let offset = offset as usize;
        let limit = limit as usize;
        let topic_norm: Option<String> = Self::normalize_topic(topic);
        let topic_norm = topic_norm.as_deref();

        with_log(|log| {
            // Collect entire filtered list IN ORDER (once)
            let items: Vec<(usize, LogEntry)> =
                iter_filtered(log, crate_name, topic_norm, min_level).collect();

            let total = items.len() as u64;

            // Slice the requested window
            let entries = items
                .into_iter()
                .skip(offset)
                .take(limit)
                .map(|(i, e)| LogEntryView::from_pair(i, e))
                .collect();

            (entries, total)
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
        with_log(|log| {
            log.iter()
                .enumerate()
                .map(|(i, e)| LogEntryView::from_pair(i, e))
                .collect()
        })
    }

    #[must_use]
    pub fn entries_from(start: u64) -> LogView {
        let start = start as usize;
        with_log(|log| {
            log.iter()
                .enumerate()
                .skip(start)
                .map(|(i, e)| LogEntryView::from_pair(i, e))
                .collect()
        })
    }

    // -------- Bulk --------

    pub fn clear_below(min_level: Level) {
        let retained: Vec<_> =
            with_log(|log| log.iter().filter(|e| e.level >= min_level).collect());

        with_log_mut(|log| *log = create_log());

        with_log(|log| {
            for entry in retained {
                let _ = log.append(&entry);
            }
        });
    }

    pub fn clear() {
        with_log_mut(|log| *log = create_log());
    }

    // -------- Introspection --------

    #[must_use]
    pub fn len() -> u64 {
        with_log(StableLogImpl::len)
    }

    #[must_use]
    pub fn is_empty() -> bool {
        Self::len() == 0
    }

    #[must_use]
    pub fn size_bytes() -> (u64, u64) {
        with_log(|log| (log.index_size_bytes(), log.data_size_bytes()))
    }

    pub fn iter() -> impl Iterator<Item = LogEntry> {
        with_log(|log| log.iter().collect::<Vec<_>>()).into_iter()
    }
}
