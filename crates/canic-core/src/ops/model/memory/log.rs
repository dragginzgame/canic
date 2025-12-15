use crate::{
    Error,
    dto::Page,
    log,
    log::{Level, Topic},
    model::memory::log::{LogEntry, StableLog, apply_retention},
    ops::{
        ic::timer::{TimerId, TimerOps},
        model::{OPS_INIT_DELAY, OPS_LOG_RETENTION_INTERVAL},
    },
    types::PageRequest,
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

            let init = TimerOps::set(OPS_INIT_DELAY, "log_retention:init", async {
                let _ = Self::retain();

                let interval = TimerOps::set_interval(
                    RETENTION_INTERVAL,
                    "log_retention:interval",
                    || async {
                        let _ = Self::retain();
                    },
                );

                RETENTION_TIMER.with_borrow_mut(|slot| *slot = Some(interval));
            });

            *slot = Some(init);
        });
    }

    /// Stop periodic retention sweeps.
    pub fn stop_retention() {
        RETENTION_TIMER.with_borrow_mut(|slot| {
            if let Some(id) = slot.take() {
                TimerOps::clear(id);
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
            .map(|(i, entry)| LogEntryDto::from_pair(i, entry))
            .collect();

        Page { entries, total }
    }
}
