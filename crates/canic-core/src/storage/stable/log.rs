//! Module: storage::stable::log
//!
//! Responsibility: persist the bounded, ordered runtime log.
//! Does not own: retention configuration, timer scheduling, or DTO projection.
//! Boundary: one stable map owns append, count eviction, age deletion, and snapshots.

use crate::cdk::structures::btreemap::BTreeMap as StableBtreeMap;
use crate::{
    cdk::structures::{DefaultMemoryImpl, Memory, memory::VirtualMemory},
    eager_static, impl_storable_unbounded,
    log::{Level, Topic},
    role_contract::allocation::memory::observability::LOG_ENTRIES_ID,
    storage::StorageError,
};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

eager_static! {
    static LOG: RefCell<LogStore<VirtualMemory<DefaultMemoryImpl>>> = RefCell::new(
        LogStore::new(StableBtreeMap::init(crate::ic_memory_key!(
            authority = CANIC_CORE_MEMORY_AUTHORITY,
            key = "canic.core.log_entries.v1",
            ty = LogEntryRecord,
            id = LOG_ENTRIES_ID
        )))
    );
}

/// One persisted runtime log entry.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LogEntryRecord {
    pub crate_name: String,
    pub created_at: u64,
    pub level: Level,
    pub topic: Option<Topic>,
    pub message: String,
}

impl LogEntryRecord {
    pub const STATE_CONTRACT_NAME: &'static str = "LogEntryRecord";
}

impl_storable_unbounded!(LogEntryRecord);

/// One logical runtime-log snapshot row preserving its ordered sequence key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LogEntrySnapshotRecord {
    pub sequence: u64,
    pub record: LogEntryRecord,
}

/// Canonical runtime-log allocation snapshot.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LogEntriesData {
    pub entries: Vec<LogEntrySnapshotRecord>,
}

impl LogEntriesData {
    pub const STATE_CONTRACT_NAME: &'static str = "LogEntriesData";
}

/// Result of one bounded age-retention batch.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LogRetentionBatch {
    pub before: u64,
    pub retained: u64,
    pub dropped: u64,
    pub more_due: bool,
}

/// Stable ordered runtime-log authority.
pub struct LogStore<M: Memory> {
    entries: StableBtreeMap<u64, LogEntryRecord, M>,
}

impl<M: Memory> LogStore<M> {
    pub const fn new(entries: StableBtreeMap<u64, LogEntryRecord, M>) -> Self {
        Self { entries }
    }

    fn append(
        &mut self,
        max_entries: u64,
        max_entry_bytes: u32,
        entry: LogEntryRecord,
    ) -> Result<Option<u64>, StorageError> {
        if max_entries == 0 {
            self.entries.clear_new();
            return Ok(None);
        }

        let sequence = match self.entries.last_key_value() {
            Some((sequence, previous)) => {
                if entry.created_at < previous.created_at {
                    return Err(StorageError::LogTimestampRegressed {
                        previous: previous.created_at,
                        current: entry.created_at,
                    });
                }
                sequence
                    .checked_add(1)
                    .ok_or(StorageError::LogSequenceExhausted)?
            }
            None => 0,
        };

        if self.entries.len() > max_entries {
            // Configuration is compiled per deployment. A lower limit is a
            // hard cut of non-authoritative runtime history, not an unbounded
            // deletion loop during the first post-upgrade append.
            self.entries.clear_new();
        } else if self.entries.len() == max_entries {
            let Some((oldest, _)) = self.entries.first_key_value() else {
                return Err(StorageError::LogCountInvariant);
            };
            self.entries.remove(&oldest);
        }

        let previous = self
            .entries
            .insert(sequence, truncate_entry(entry, max_entry_bytes));
        if previous.is_some() {
            return Err(StorageError::LogSequenceConflict(sequence));
        }

        Ok(Some(sequence))
    }

    fn retain_created_before(&mut self, cutoff: u64, limit: usize) -> LogRetentionBatch {
        let before = self.entries.len();
        let mut dropped = 0u64;

        let limit = u64::try_from(limit).unwrap_or(u64::MAX);
        while dropped < limit {
            let Some((sequence, entry)) = self.entries.first_key_value() else {
                break;
            };
            if entry.created_at >= cutoff {
                break;
            }
            self.entries.remove(&sequence);
            dropped = dropped.saturating_add(1);
        }

        let more_due = self
            .entries
            .first_key_value()
            .is_some_and(|(_, entry)| entry.created_at < cutoff);
        LogRetentionBatch {
            before,
            retained: self.entries.len(),
            dropped,
            more_due,
        }
    }

    fn snapshot(&self) -> Vec<LogEntryRecord> {
        self.entries.iter().map(|entry| entry.value()).collect()
    }

    fn oldest_created_at(&self) -> Option<u64> {
        self.entries
            .first_key_value()
            .map(|(_, entry)| entry.created_at)
    }

    #[cfg(test)]
    fn export(&self) -> LogEntriesData {
        LogEntriesData {
            entries: self
                .entries
                .iter()
                .map(|entry| LogEntrySnapshotRecord {
                    sequence: *entry.key(),
                    record: entry.value(),
                })
                .collect(),
        }
    }

    #[cfg(test)]
    fn import(&mut self, data: LogEntriesData) {
        self.entries.clear_new();
        for entry in data.entries {
            self.entries.insert(entry.sequence, entry.record);
        }
    }
}

/// Static facade over the runtime log store.
pub struct Log;

impl Log {
    pub(crate) fn append(
        max_entries: u64,
        max_entry_bytes: u32,
        entry: LogEntryRecord,
    ) -> Result<Option<u64>, StorageError> {
        LOG.with_borrow_mut(|log| log.append(max_entries, max_entry_bytes, entry))
    }

    #[must_use]
    pub(crate) fn snapshot() -> Vec<LogEntryRecord> {
        LOG.with_borrow(LogStore::snapshot)
    }

    #[must_use]
    pub(crate) fn oldest_created_at() -> Option<u64> {
        LOG.with_borrow(LogStore::oldest_created_at)
    }

    #[must_use]
    pub(crate) fn retain_created_before(cutoff: u64, limit: usize) -> LogRetentionBatch {
        LOG.with_borrow_mut(|log| log.retain_created_before(cutoff, limit))
    }

    #[cfg(test)]
    pub(crate) fn export() -> LogEntriesData {
        LOG.with_borrow(LogStore::export)
    }

    #[cfg(test)]
    pub(crate) fn import(data: LogEntriesData) {
        LOG.with_borrow_mut(|log| log.import(data));
    }

    #[cfg(test)]
    pub(crate) fn reset_for_tests() {
        LOG.with_borrow_mut(|log| log.entries.clear_new());
    }
}

const TRUNCATION_SUFFIX: &str = "...[truncated]";

fn truncate_entry(mut entry: LogEntryRecord, max_bytes: u32) -> LogEntryRecord {
    if let Some(message) = truncate_message(&entry.message, max_bytes) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(created_at: u64, message: &str) -> LogEntryRecord {
        LogEntryRecord {
            crate_name: "canic-test".to_string(),
            created_at,
            level: Level::Info,
            topic: None,
            message: message.to_string(),
        }
    }

    #[test]
    fn append_enforces_the_exact_count_limit_and_entry_bound() {
        Log::reset_for_tests();
        Log::append(2, 3, entry(1, "first")).expect("append first");
        Log::append(2, 3, entry(2, "second")).expect("append second");
        Log::append(2, 3, entry(3, "third")).expect("append third");

        let entries = Log::snapshot();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0], entry(2, "sec"));
        assert_eq!(entries[1], entry(3, "thi"));
    }

    #[test]
    fn zero_count_limit_clears_existing_entries_without_appending() {
        Log::reset_for_tests();
        Log::append(2, 100, entry(1, "first")).expect("append first");
        assert_eq!(
            Log::append(0, 100, entry(2, "second")).expect("disable log"),
            None
        );
        assert!(Log::snapshot().is_empty());
    }

    #[test]
    fn append_rejects_timestamp_regression_before_mutation() {
        Log::reset_for_tests();
        Log::append(2, 100, entry(2, "first")).expect("append first");
        let err = Log::append(2, 100, entry(1, "second")).expect_err("reject regression");

        assert!(matches!(
            err,
            StorageError::LogTimestampRegressed {
                previous: 2,
                current: 1
            }
        ));
        assert_eq!(Log::snapshot(), vec![entry(2, "first")]);
    }

    #[test]
    fn reduced_count_limit_clears_history_in_one_hard_cut() {
        Log::reset_for_tests();
        for created_at in 1..=3 {
            Log::append(3, 100, entry(created_at, "entry")).expect("append entry");
        }

        Log::append(1, 100, entry(4, "current")).expect("apply reduced limit");

        assert_eq!(Log::snapshot(), vec![entry(4, "current")]);
    }

    #[test]
    fn age_retention_is_bounded_and_preserves_the_strict_cutoff() {
        Log::reset_for_tests();
        for created_at in [10, 20, 30] {
            Log::append(10, 100, entry(created_at, "entry")).expect("append entry");
        }

        let first = Log::retain_created_before(30, 1);
        assert_eq!(first.dropped, 1);
        assert!(first.more_due);
        assert_eq!(Log::oldest_created_at(), Some(20));

        let second = Log::retain_created_before(30, 1);
        assert_eq!(second.dropped, 1);
        assert!(!second.more_due);
        assert_eq!(Log::oldest_created_at(), Some(30));
    }

    #[test]
    fn snapshot_round_trip_preserves_sequence_and_records() {
        Log::reset_for_tests();
        Log::append(10, 100, entry(10, "first")).expect("append first");
        Log::append(10, 100, entry(20, "second")).expect("append second");
        let data = Log::export();

        Log::reset_for_tests();
        Log::import(data.clone());

        assert_eq!(Log::export(), data);
    }
}
