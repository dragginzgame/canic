#![allow(clippy::cast_possible_truncation)]

use crate::{
    Error,
    cdk::structures::{
        DefaultMemoryImpl,
        log::{Log as StableLogImpl, WriteError},
        memory::VirtualMemory,
    },
    config::schema::LogConfig,
    eager_static, ic_memory,
    log::Level,
    memory::impl_storable_unbounded,
    storage::memory::{
        MemoryError,
        id::log::{LOG_DATA_ID, LOG_INDEX_ID},
    },
    utils::case::{Case, Casing},
};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::VecDeque};

///
/// StableLog
///

type StableLog =
    StableLogImpl<LogEntry, VirtualMemory<DefaultMemoryImpl>, VirtualMemory<DefaultMemoryImpl>>;

struct LogIndexMemory;
struct LogDataMemory;

#[derive(Clone)]
struct LogMemory {
    index: VirtualMemory<DefaultMemoryImpl>,
    data: VirtualMemory<DefaultMemoryImpl>,
}

impl LogMemory {
    fn new() -> Self {
        Self {
            index: ic_memory!(LogIndexMemory, LOG_INDEX_ID),
            data: ic_memory!(LogDataMemory, LOG_DATA_ID),
        }
    }
}

eager_static! {
    static LOG_MEMORY: LogMemory = LogMemory::new();
}

fn create_log() -> StableLog {
    LOG_MEMORY.with(|mem| StableLogImpl::new(mem.index.clone(), mem.data.clone()))
}

eager_static! {
    static LOG: RefCell<StableLog> = RefCell::new(create_log());
}

fn with_log<R>(f: impl FnOnce(&StableLog) -> R) -> R {
    LOG.with_borrow(f)
}

fn with_log_mut<R>(f: impl FnOnce(&mut StableLog) -> R) -> R {
    LOG.with_borrow_mut(f)
}

///
/// LogEntry
///

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LogEntry {
    pub crate_name: String,
    pub created_at: u64,
    pub level: Level,
    pub topic: Option<String>,
    pub message: String,
}

impl_storable_unbounded!(LogEntry);

///
/// Log
///

pub struct Log;

impl Log {
    pub(crate) fn append<T, M>(
        cfg: &LogConfig,
        now_secs: u64,
        crate_name: &str,
        topic: Option<T>,
        level: Level,
        message: M,
    ) -> Result<u64, Error>
    where
        T: ToString,
        M: AsRef<str>,
    {
        if cfg.max_entries == 0 {
            return Ok(0);
        }

        let entry = LogEntry {
            crate_name: crate_name.to_string(),
            created_at: now_secs,
            level,
            topic: normalize_topic(topic),
            message: message.as_ref().to_string(),
        };

        let entry = truncate_entry(entry, cfg.max_entry_bytes);
        let id = append_raw(&entry)?;

        Ok(id)
    }

    #[must_use]
    pub(crate) fn snapshot() -> Vec<LogEntry> {
        let mut out = Vec::new();
        with_log(|log| {
            for entry in log.iter() {
                out.push(entry);
            }
        });

        out
    }
}

///
/// RetentionSummary
///

#[derive(Clone, Debug, Default)]
pub struct RetentionSummary {
    pub before: u64,
    pub retained: u64,
    pub dropped_by_age: u64,
    pub dropped_by_limit: u64,
}

impl RetentionSummary {
    #[must_use]
    pub const fn dropped_total(&self) -> u64 {
        self.dropped_by_age + self.dropped_by_limit
    }
}

pub fn apply_retention_with_cfg(cfg: &LogConfig, now_secs: u64) -> Result<RetentionSummary, Error> {
    let before = with_log(StableLog::len);

    if cfg.max_entries == 0 {
        with_log_mut(|log| *log = create_log());
        return Ok(RetentionSummary {
            before,
            retained: 0,
            dropped_by_age: 0,
            dropped_by_limit: before,
        });
    }

    if before == 0 {
        return Ok(RetentionSummary::default());
    }

    let max_entries = usize::try_from(cfg.max_entries).unwrap_or(usize::MAX);
    let mut retained = VecDeque::new();
    let mut eligible = 0u64;

    with_log(|log| {
        for entry in log.iter() {
            if let Some(max_age) = cfg.max_age_secs
                && now_secs.saturating_sub(entry.created_at) > max_age
            {
                continue;
            }

            eligible += 1;
            retained.push_back(entry);
            if retained.len() > max_entries {
                retained.pop_front();
            }
        }
    });

    let retained_len = retained.len() as u64;
    let dropped_by_age = if cfg.max_age_secs.is_some() {
        before.saturating_sub(eligible)
    } else {
        0
    };
    let dropped_by_limit = eligible.saturating_sub(retained_len);

    if dropped_by_age == 0 && dropped_by_limit == 0 {
        return Ok(RetentionSummary {
            before,
            retained: retained_len,
            dropped_by_age: 0,
            dropped_by_limit: 0,
        });
    }

    with_log_mut(|log| *log = create_log());
    for entry in retained {
        let entry = truncate_entry(entry, cfg.max_entry_bytes);
        append_raw(&entry)?;
    }

    Ok(RetentionSummary {
        before,
        retained: retained_len,
        dropped_by_age,
        dropped_by_limit,
    })
}

//
// ─────────────────────────────────────────────────────────────
// Internal helpers (mechanical)
// ─────────────────────────────────────────────────────────────
//

const TRUNCATION_SUFFIX: &str = "...[truncated]";

fn append_raw(entry: &LogEntry) -> Result<u64, MemoryError> {
    with_log(|log| log.append(entry)).map_err(map_write_error)
}

const fn map_write_error(err: WriteError) -> MemoryError {
    match err {
        WriteError::GrowFailed {
            current_size,
            delta,
        } => MemoryError::LogWriteFailed {
            current_size,
            delta,
        },
    }
}

// Centralizes topic normalization to enforce invariants.
#[allow(clippy::single_option_map)]
fn normalize_topic<T: ToString>(topic: Option<T>) -> Option<String> {
    topic.map(|t| t.to_string().to_case(Case::Snake))
}

fn truncate_entry(mut entry: LogEntry, max_bytes: u32) -> LogEntry {
    if let Some(msg) = truncate_message(&entry.message, max_bytes) {
        entry.message = msg;
    }
    entry
}

fn truncate_message(message: &str, max_entry_bytes: u32) -> Option<String> {
    let max_entry_bytes = usize::try_from(max_entry_bytes).unwrap_or(usize::MAX);
    if message.len() <= max_entry_bytes {
        return None;
    }

    if max_entry_bytes == 0 {
        return Some(String::new());
    }

    if max_entry_bytes <= TRUNCATION_SUFFIX.len() {
        return Some(truncate_to_boundary(message, max_entry_bytes).to_string());
    }

    let keep_len = max_entry_bytes - TRUNCATION_SUFFIX.len();
    let mut truncated = truncate_to_boundary(message, keep_len).to_string();
    truncated.push_str(TRUNCATION_SUFFIX);

    Some(truncated)
}

fn truncate_to_boundary(message: &str, max_bytes: usize) -> &str {
    if message.len() <= max_bytes {
        return message;
    }

    let mut end = max_bytes;
    while end > 0 && !message.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }

    &message[..end]
}
