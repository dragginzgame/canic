use crate::{
    Error,
    cdk::timers::{TimerId, clear_timer, set_timer, set_timer_interval},
    log,
    log::{Level, Topic},
    model::memory::log::{LogEntry, StableLog, apply_retention},
    ops::model::{OPS_INIT_DELAY, OPS_LOG_RETENTION_INTERVAL},
};
use candid::CandidType;
use serde::Serialize;
use std::{cell::RefCell, time::Duration};

thread_local! {
    static RETENTION_TIMER: RefCell<Option<TimerId>> = const { RefCell::new(None) };
}

/// How often to enforce retention after the first sweep.
const RETENTION_INTERVAL: Duration = OPS_LOG_RETENTION_INTERVAL;

///
/// LogEntryDto
///

#[derive(CandidType, Clone, Debug, Serialize)]
pub struct LogEntryDto {
    pub index: u64,
    pub created_at: u64,
    pub crate_name: String,
    pub level: Level,
    pub topic: Option<String>,
    pub message: String,
}

impl LogEntryDto {
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

///
/// LogPageDto
///

#[derive(CandidType, Serialize)]
pub struct LogPageDto {
    pub entries: Vec<LogEntryDto>,
    pub total: u64,
}

///
/// LogOps
///

pub struct LogOps;

impl LogOps {
    /// Start periodic log retention sweeps. Safe to call multiple times.
    pub fn start_retention() {
        RETENTION_TIMER.with_borrow_mut(|slot| {
            if slot.is_some() {
                return;
            }

            let init = set_timer(OPS_INIT_DELAY, async {
                let _ = Self::retain();

                let interval = set_timer_interval(RETENTION_INTERVAL, || async {
                    let _ = Self::retain();
                });

                RETENTION_TIMER.with_borrow_mut(|slot| *slot = Some(interval));
            });

            *slot = Some(init);
        });
    }

    /// Stop periodic retention sweeps.
    pub fn stop_retention() {
        RETENTION_TIMER.with_borrow_mut(|slot| {
            if let Some(id) = slot.take() {
                clear_timer(id);
            }
        });
    }

    /// Run a retention sweep immediately.
    #[must_use]
    pub fn retain() -> bool {
        match apply_retention() {
            Ok(()) => true,
            Err(err) => {
                log!(Topic::Memory, Warn, "log retention failed: {err}");
                false
            }
        }
    }

    /// Append a log entry to stable storage.
    pub fn append<T: ToString, M: AsRef<str>>(
        crate_name: &str,
        topic: Option<T>,
        level: Level,
        message: M,
    ) -> Result<u64, Error> {
        StableLog::append(crate_name, topic, level, message)
    }

    ///
    /// Export a page of log entries and the total count.
    ///
    #[must_use]
    pub fn page(
        crate_name: Option<String>,
        topic: Option<String>,
        min_level: Option<Level>,
        offset: u64,
        limit: u64,
    ) -> LogPageDto {
        let (raw_entries, total) = StableLog::entries_page_filtered(
            crate_name.as_deref(),
            topic.as_deref(),
            min_level,
            offset,
            limit,
        );

        let entries = raw_entries
            .into_iter()
            .map(|(i, entry)| LogEntryDto::from_pair(i, entry))
            .collect();

        LogPageDto { entries, total }
    }
}
