#![allow(clippy::cast_possible_truncation)]
use crate::{
    Error, ThisError,
    cdk::{
        structures::{
            DefaultMemoryImpl,
            log::{Log as StableLogImpl, WriteError},
            memory::VirtualMemory,
        },
        utils::time,
    },
    config::{Config, schema::LogConfig},
    dto::page::PageRequest,
    eager_static, ic_memory,
    log::Level,
    memory::impl_storable_unbounded,
    model::memory::{
        MemoryError,
        id::log::{LOG_DATA_ID, LOG_INDEX_ID},
    },
    utils::case::{Case, Casing},
};
use candid::CandidType;
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::VecDeque};

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

pub fn log_config() -> LogConfig {
    Config::get().log.clone()
}

const TRUNCATION_SUFFIX: &str = "...[truncated]";

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
    pub(crate) fn new(crate_name: &str, level: Level, topic: Option<&str>, msg: &str) -> Self {
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

pub struct StableLog;

impl StableLog {
    // -------- Append --------

    pub(crate) fn append<T, M>(
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

    /// Append an entry, returning its log index.
    /// When logging is disabled (`max_entries == 0`), returns 0 and does not write.
    pub(crate) fn append_entry(entry: LogEntry) -> Result<u64, Error> {
        let cfg = log_config();

        if cfg.max_entries == 0 {
            return Ok(0);
        }

        let entry = maybe_truncate_entry(entry, cfg.max_entry_bytes);

        with_log(|log| log.append(&entry))
            .map_err(LogError::from)
            .map_err(Error::from)
    }

    // -------- Helper -----------

    fn normalize_topic<T: ToString>(topic: Option<T>) -> Option<String> {
        topic.as_ref().map(|t| t.to_string().to_case(Case::Snake))
    }

    #[must_use]
    pub(crate) fn entries_page_filtered(
        crate_name: Option<&str>,
        topic: Option<&str>,
        min_level: Option<Level>,
        request: PageRequest,
    ) -> (Vec<(usize, LogEntry)>, u64) {
        let request = request.clamped();
        let limit = usize::try_from(request.limit).unwrap_or(usize::MAX);
        let topic_norm: Option<String> = Self::normalize_topic(topic);
        let topic_norm = topic_norm.as_deref();
        let offset = request.offset;

        with_log(|log| {
            let mut total = 0u64;
            let mut entries: Vec<(usize, LogEntry)> = Vec::new();

            for (idx, entry) in iter_filtered(log, crate_name, topic_norm, min_level) {
                if total < offset {
                    total = total.saturating_add(1);
                    continue;
                }

                if entries.len() < limit {
                    entries.push((idx, entry));
                }

                total = total.saturating_add(1);
            }

            (entries, total)
        })
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

// apply_retention
// currently using the local config
pub fn apply_retention() -> Result<RetentionSummary, Error> {
    let cfg = log_config();
    let original_len = with_log(StableLogImpl::len);

    if cfg.max_entries == 0 {
        with_log_mut(|log| *log = create_log());
        return Ok(RetentionSummary {
            before: original_len,
            retained: 0,
            dropped_by_age: 0,
            dropped_by_limit: original_len,
        });
    }

    let now = time::now_secs();
    let max_entries = cfg.max_entries;
    let max_entries_usize = usize::try_from(max_entries).unwrap_or(usize::MAX);

    if original_len == 0 {
        return Ok(RetentionSummary::default());
    }

    // Fast path: no age filter and we are already within limits.
    if cfg.max_age_secs.is_none() && original_len <= max_entries {
        return Ok(RetentionSummary {
            before: original_len,
            retained: original_len,
            dropped_by_age: 0,
            dropped_by_limit: 0,
        });
    }

    let mut retained = VecDeque::with_capacity(
        max_entries_usize.min(usize::try_from(original_len).unwrap_or(usize::MAX)),
    );
    let mut eligible = 0u64;

    with_log(|log| {
        for entry in log.iter() {
            if let Some(age) = cfg.max_age_secs
                && now.saturating_sub(entry.created_at) > age
            {
                continue;
            }

            eligible += 1;
            retained.push_back(entry);
            if retained.len() > max_entries_usize {
                retained.pop_front();
            }
        }
    });

    let retained_len = retained.len() as u64;
    let dropped_by_age = if cfg.max_age_secs.is_some() {
        original_len.saturating_sub(eligible)
    } else {
        0
    };
    let dropped_by_limit = eligible.saturating_sub(retained_len);
    let changed = dropped_by_age > 0 || dropped_by_limit > 0;

    let summary = RetentionSummary {
        before: original_len,
        retained: retained_len,
        dropped_by_age,
        dropped_by_limit,
    };

    if !changed {
        return Ok(summary);
    }

    // Rewrite
    with_log_mut(|log| *log = create_log());
    for entry in retained {
        let entry = maybe_truncate_entry(entry, cfg.max_entry_bytes);
        with_log(|log| log.append(&entry))
            .map_err(LogError::from)
            .map_err(Error::from)?;
    }

    Ok(summary)
}

fn maybe_truncate_entry(mut entry: LogEntry, max_entry_bytes: u32) -> LogEntry {
    if let Some(message) = truncate_message(&entry.message, max_entry_bytes) {
        entry.message = message;
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

//
// TESTS
//

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, schema::ConfigModel};

    #[test]
    fn retention_trims_old_and_excess_entries() {
        Config::reset_for_tests();
        let mut cfg = ConfigModel::test_default();
        cfg.log.max_entries = 2;
        cfg.log.max_age_secs = Some(5);
        Config::init_from_toml(&toml::to_string(&cfg).unwrap()).unwrap();

        let now = time::now_secs();
        // fresh entry
        StableLog::append("test", Option::<&str>::None, Level::Info, "fresh1").unwrap();

        // old entry (backdated)
        let mut old = LogEntry::new("test", Level::Info, None, "old");
        old.created_at = now.saturating_sub(10);
        StableLog::append_entry(old).unwrap();

        // another fresh entry
        StableLog::append("test", Option::<&str>::None, Level::Info, "fresh2").unwrap();

        let summary = apply_retention().unwrap();
        assert_eq!(summary.dropped_by_age, 1);
        assert_eq!(summary.dropped_by_limit, 0);

        let (entries, total) =
            StableLog::entries_page_filtered(None, None, None, PageRequest::new(10, 0));
        assert_eq!(total, 2);
        let msgs: Vec<_> = entries.into_iter().map(|(_, e)| e.message).collect();
        assert!(msgs.contains(&"fresh1".to_string()));
        assert!(msgs.contains(&"fresh2".to_string()));
        assert!(!msgs.contains(&"old".to_string()));
    }

    #[test]
    fn log_truncates_oversized_messages() {
        Config::reset_for_tests();
        let mut cfg = ConfigModel::test_default();
        cfg.log.max_entries = 10;
        cfg.log.max_entry_bytes = 20;
        Config::init_from_toml(&toml::to_string(&cfg).unwrap()).unwrap();

        let message = "abcdefghijklmnopqrstuvwxyz";
        StableLog::append(
            "log_truncate_test",
            Option::<&str>::None,
            Level::Info,
            message,
        )
        .unwrap();

        let (entries, total) = StableLog::entries_page_filtered(
            Some("log_truncate_test"),
            None,
            None,
            PageRequest::new(10, 0),
        );
        assert_eq!(total, 1);
        let entry = entries.first().expect("expected a logged entry").1.clone();

        let keep_len = cfg
            .log
            .max_entry_bytes
            .saturating_sub(TRUNCATION_SUFFIX.len() as u32) as usize;
        let expected = format!("{}{}", &message[..keep_len], TRUNCATION_SUFFIX);
        assert_eq!(entry.message, expected);
    }
}
