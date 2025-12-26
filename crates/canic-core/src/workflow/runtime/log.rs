use crate::{
    Error,
    dto::page::{Page, PageRequest},
    log,
    log::{Level, Topic},
    model::memory::log::{LogEntry, StableLog, apply_retention},
    ops::{OPS_INIT_DELAY, OPS_LOG_RETENTION_INTERVAL},
    workflow::ic::timer::{TimerId, TimerOps},
};
use candid::CandidType;
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, time::Duration};

thread_local! {
    static RETENTION_TIMER: RefCell<Option<TimerId>> = const { RefCell::new(None) };
}

/// How often to enforce retention after the first sweep.
const RETENTION_INTERVAL: Duration = OPS_LOG_RETENTION_INTERVAL;

///
/// LogEntryDto
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct LogEntryDto {
    pub index: u64,
    pub created_at: u64,
    pub crate_name: String,
    pub level: Level,
    pub topic: Option<String>,
    pub message: String,
}

impl LogEntryDto {
    fn from_indexed_entry(index: usize, entry: LogEntry) -> Self {
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
/// LogOps
///

pub struct LogOps;

impl LogOps {
    /// Start periodic log retention sweeps. Safe to call multiple times.
    pub fn start_retention() {
        let _ = TimerOps::set_guarded_interval(
            &RETENTION_TIMER,
            OPS_INIT_DELAY,
            "log_retention:init",
            || async {
                let _ = Self::retain();
            },
            RETENTION_INTERVAL,
            "log_retention:interval",
            || async {
                let _ = Self::retain();
            },
        );
    }

    /// Stop periodic retention sweeps.
    pub fn stop_retention() {
        let _ = TimerOps::clear_guarded(&RETENTION_TIMER);
    }

    /// Run a retention sweep immediately.
    /// This enforces configured retention limits on stable logs.
    #[must_use]
    pub fn retain() -> bool {
        match apply_retention() {
            Ok(summary) => {
                let dropped = summary.dropped_total();
                if dropped > 0 {
                    let before = summary.before;
                    let retained = summary.retained;
                    let dropped_by_age = summary.dropped_by_age;
                    let dropped_by_limit = summary.dropped_by_limit;
                    log!(
                        Topic::Memory,
                        Info,
                        "log retention: dropped={dropped} (age={dropped_by_age}, limit={dropped_by_limit}), before={before}, retained={retained}"
                    );
                }
                true
            }
            Err(err) => {
                log!(Topic::Memory, Warn, "log retention failed: {err}");
                false
            }
        }
    }

    /// Append a log entry to stable storage.
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
        request: PageRequest,
    ) -> Page<LogEntryDto> {
        let request = request.clamped();

        let (raw_entries, total) = StableLog::entries_page_filtered(
            crate_name.as_deref(),
            topic.as_deref(),
            min_level,
            request,
        );

        let entries = raw_entries
            .into_iter()
            .map(|(i, entry)| LogEntryDto::from_indexed_entry(i, entry))
            .collect();

        Page { entries, total }
    }
}
