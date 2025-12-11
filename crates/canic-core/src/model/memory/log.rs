#![allow(clippy::cast_possible_truncation)]
use crate::{
    Error, ThisError,
    cdk::structures::{
        DefaultMemoryImpl,
        log::{Log as StableLogImpl, WriteError},
        memory::VirtualMemory,
    },
    config::{Config, schema::LogConfig},
    eager_static, ic_memory, impl_storable_unbounded,
    log::Level,
    model::memory::{
        MemoryError,
        id::log::{LOG_DATA_ID, LOG_INDEX_ID},
    },
    types::PageRequest,
    utils::{
        case::{Case, Casing},
        time,
    },
};

use candid::CandidType;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

//
// Stable Log Storage (ic-stable-structures)
//

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

pub(crate) fn log_config() -> LogConfig {
    Config::try_get().map(|c| c.log.clone()).unwrap_or_default()
}

///
/// LogError
/// it's ok to have errors in this model-layer struct as logs have more
/// error cases than B-Tree maps
///

#[derive(Debug, ThisError)]
pub enum LogError {
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
        MemoryError::from(err).into()
    }
}

///
/// LogEntry
///

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

///
/// StableLog
///

pub(crate) struct StableLog;

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

        with_log(|log| log.append(&entry))
            .map_err(LogError::from)
            .map_err(Error::from)
    }

    // -------- Helper -----------

    fn normalize_topic<T: ToString>(topic: Option<T>) -> Option<String> {
        topic.as_ref().map(|t| t.to_string().to_case(Case::Snake))
    }

    #[must_use]
    pub fn entries_page_filtered(
        crate_name: Option<&str>,
        topic: Option<&str>,
        min_level: Option<Level>,
        request: PageRequest,
    ) -> (Vec<(usize, LogEntry)>, u64) {
        let request = request.clamped();
        let offset = usize::try_from(request.offset).unwrap_or(usize::MAX);
        let limit = usize::try_from(request.limit).unwrap_or(usize::MAX);
        let topic_norm: Option<String> = Self::normalize_topic(topic);
        let topic_norm = topic_norm.as_deref();

        with_log(|log| {
            // Collect entire filtered list IN ORDER (once)
            let items: Vec<(usize, LogEntry)> =
                iter_filtered(log, crate_name, topic_norm, min_level).collect();

            let total = items.len() as u64;

            // Slice the requested window
            let entries = items.into_iter().skip(offset).take(limit).collect();

            (entries, total)
        })
    }
}

// apply_retention
// currently using the local config
pub(crate) fn apply_retention() -> Result<(), Error> {
    let cfg = log_config();

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
