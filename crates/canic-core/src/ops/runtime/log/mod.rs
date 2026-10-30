//! Module: ops::runtime::log
//!
//! Responsibility: bind the upstream heap logger to Canic configuration and DTOs.
//! Does not own: log storage algorithms, retention policy, filtering, or timers.
//! Boundary: one caller-owned IC Logging instance; no stable log allocation.

#[cfg(test)]
mod tests;

use crate::{
    InternalError,
    diagnostics::codes,
    dto::{
        log::LogEntry,
        page::{Page, PageRequest},
    },
    log::{Level, Topic},
};
use std::{cell::RefCell, num::NonZeroUsize};

const LOG_LABEL_MAX_BYTES: usize = 256;
const LOG_PAGE_MAX_LIMIT: usize = 1_000;
const LOG_SCAN_BATCH: NonZeroUsize = NonZeroUsize::new(256).unwrap();

thread_local! {
    static LOG: RefCell<Option<RuntimeLog>> = const { RefCell::new(None) };
}

///
/// RuntimeLog
///
/// Selected immutable host limits alongside the upstream recorder they configure.
/// Replacing the host selection starts a new diagnostic window.
///

struct RuntimeLog {
    limits: ic_logging::Limits,
    log: ic_logging::Log,
}

///
/// LogOps
///
/// Canic context and boundary conversion for the independent logging engine.
///

pub struct LogOps;

impl LogOps {
    /// Append through the upstream owner after Canic's startup admission.
    pub fn append_runtime_log(
        crate_name: &str,
        topic: Option<Topic>,
        level: Level,
        message: &str,
        created_at: u64,
        max_entries: u64,
        max_entry_bytes: u32,
    ) -> Result<Option<u64>, InternalError> {
        if !crate::log::is_ready() {
            return Ok(None);
        }
        LOG.with_borrow_mut(|selected| {
            append(
                selected,
                ic_logging::EntryInput {
                    created_at_secs: created_at,
                    level: upstream_level(level),
                    source: crate_name,
                    topic: topic.map(Topic::log_label),
                    message,
                },
                max_entries,
                max_entry_bytes,
            )
        })
    }

    /// Run one upstream retention batch using the host's explicit time and budget.
    #[must_use]
    pub fn prune_expired(
        now_secs: u64,
        max_age_secs: u64,
        max_remove: usize,
    ) -> ic_logging::RetentionBatch {
        LOG.with_borrow_mut(|selected| {
            selected.as_mut().map_or(
                ic_logging::RetentionBatch {
                    removed: 0,
                    more_due: false,
                },
                |selected| {
                    selected
                        .log
                        .prune_expired(now_secs, max_age_secs, max_remove)
                },
            )
        })
    }

    /// Project the upstream expiry deadline without reconstructing age arithmetic.
    #[must_use]
    pub fn next_expiry_secs(max_age_secs: u64) -> Option<u64> {
        LOG.with_borrow(|selected| selected.as_ref()?.log.next_expiry_secs(max_age_secs))
    }

    /// Convert upstream cursor scans into Canic's offset/total query envelope.
    /// Exact totals scan retained history; only selected reply rows copy text.
    #[must_use]
    pub fn page_filtered(
        crate_name: Option<&str>,
        topic: Option<&str>,
        min_level: Option<Level>,
        page: PageRequest,
    ) -> Page<LogEntry> {
        let filter = ic_logging::Filter {
            source: crate_name,
            topic,
            min_level: min_level.map(upstream_level),
        };
        LOG.with_borrow(|selected| match selected {
            Some(selected) => project_page(&selected.log, filter, page),
            None => Page {
                entries: Vec::new(),
                total: 0,
            },
        })
    }

    #[cfg(test)]
    pub(crate) fn append_test_entry(created_at: u64) {
        LOG.with_borrow_mut(|selected| {
            append(
                selected,
                ic_logging::EntryInput {
                    created_at_secs: created_at,
                    level: ic_logging::Level::Info,
                    source: "canic-test",
                    topic: None,
                    message: "entry",
                },
                1_000,
                1_000,
            )
            .expect("append test log")
        });
    }

    #[cfg(test)]
    pub(crate) fn reset_for_tests() {
        LOG.with_borrow_mut(|selected| *selected = None);
    }
}

fn append(
    selected: &mut Option<RuntimeLog>,
    input: ic_logging::EntryInput<'_>,
    max_entries: u64,
    max_entry_bytes: u32,
) -> Result<Option<u64>, InternalError> {
    let capacity = usize::try_from(max_entries).map_err(|_| InternalError::invalid_input())?;
    let Some(max_entries) = NonZeroUsize::new(capacity) else {
        *selected = None;
        return Ok(None);
    };
    let limits = ic_logging::Limits {
        max_entries,
        max_message_bytes: max_entry_bytes as usize,
        max_label_bytes: LOG_LABEL_MAX_BYTES,
    };
    if selected
        .as_ref()
        .is_some_and(|runtime| runtime.limits != limits)
    {
        *selected = None;
    }
    let runtime = selected.get_or_insert_with(|| RuntimeLog {
        limits,
        log: ic_logging::Log::new(limits),
    });
    runtime.log.append(input).map(Some).map_err(append_error)
}

fn project_page(
    log: &ic_logging::Log,
    filter: ic_logging::Filter<'_>,
    request: PageRequest,
) -> Page<LogEntry> {
    let limit = usize::try_from(request.limit)
        .unwrap_or(usize::MAX)
        .min(LOG_PAGE_MAX_LIMIT);
    let mut entries = Vec::new();
    let mut total = 0u64;
    let mut before = None;
    loop {
        let page = log.page(
            filter,
            ic_logging::PageRequest {
                before,
                max_results: LOG_SCAN_BATCH,
                max_scanned: LOG_SCAN_BATCH,
            },
        );
        for entry in page.entries {
            if total >= request.offset && entries.len() < limit {
                entries.push(project_entry(entry));
            }
            total = total.saturating_add(1);
        }
        let Some(cursor) = page.next_before else {
            break;
        };
        before = Some(cursor);
    }
    Page { entries, total }
}

fn project_entry(entry: &ic_logging::Entry) -> LogEntry {
    LogEntry {
        crate_name: entry.source.clone(),
        created_at: entry.created_at_secs,
        level: canic_level(entry.level),
        topic: entry.topic.clone(),
        message: entry.message.clone(),
    }
}

const fn upstream_level(level: Level) -> ic_logging::Level {
    match level {
        Level::Debug => ic_logging::Level::Debug,
        Level::Info => ic_logging::Level::Info,
        Level::Ok => ic_logging::Level::Ok,
        Level::Warn => ic_logging::Level::Warn,
        Level::Error => ic_logging::Level::Error,
    }
}

const fn canic_level(level: ic_logging::Level) -> Level {
    match level {
        ic_logging::Level::Debug => Level::Debug,
        ic_logging::Level::Info => Level::Info,
        ic_logging::Level::Ok => Level::Ok,
        ic_logging::Level::Warn => Level::Warn,
        ic_logging::Level::Error => Level::Error,
    }
}

const fn append_error(error: ic_logging::AppendError) -> InternalError {
    let code = match error {
        ic_logging::AppendError::LabelTooLong { .. } => codes::CAPACITY_LIMIT,
        ic_logging::AppendError::TimestampRegressed { .. } => codes::TIME_ORDERING,
        ic_logging::AppendError::SequenceExhausted => codes::VERSION_CAPACITY,
    };
    InternalError::public(code)
}
