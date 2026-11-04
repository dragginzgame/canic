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
    dto::log::{LogEntry, LogPage, LogPageRequest},
    log::{Level, Topic},
};
use std::{cell::RefCell, num::NonZeroUsize};

const LOG_LABEL_MAX_BYTES: usize = 256;
const LOG_PAGE_MAX_LIMIT: usize = 1_000;
const LOG_SCAN_MAX: u32 = 1024;
const LOG_PAGE_TEXT_MAX_BYTES: usize = 1024 * 1024;

thread_local! {
    static LOG: RefCell<Option<ic_logging::Log>> = const { RefCell::new(None) };
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
                |selected| selected.prune_expired(now_secs, max_age_secs, max_remove),
            )
        })
    }

    /// Project the upstream expiry deadline without reconstructing age arithmetic.
    #[must_use]
    pub fn next_expiry_secs(max_age_secs: u64) -> Option<u64> {
        LOG.with_borrow(|selected| selected.as_ref()?.next_expiry_secs(max_age_secs))
    }

    /// Project one upstream bounded scan without counting or copying complete history.
    pub fn page_filtered(
        crate_name: Option<&str>,
        topic: Option<&str>,
        min_level: Option<Level>,
        page: LogPageRequest,
    ) -> Result<LogPage, InternalError> {
        if page.limit == 0 {
            return Err(InternalError::invalid_input());
        }
        let filter = ic_logging::Filter {
            source: crate_name,
            topic: topic.map_or(ic_logging::TopicMatch::Any, ic_logging::TopicMatch::Exact),
            min_level: min_level.map(upstream_level),
        };
        LOG.with_borrow(|selected| match selected {
            Some(selected) => project_page(selected, filter, page),
            None => Ok(LogPage {
                entries: Vec::new(),
                scanned: 0,
                next_before: None,
            }),
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
    selected: &mut Option<ic_logging::Log>,
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
        // Preserve the admitted count and per-field limits within the address space.
        max_text_bytes: capacity
            .saturating_mul((max_entry_bytes as usize).saturating_add(2 * LOG_LABEL_MAX_BYTES)),
    };
    if selected.as_ref().is_some_and(|log| log.limits() != limits) {
        *selected = None;
    }
    let log = selected.get_or_insert_with(|| ic_logging::Log::new(limits));
    log.append(input)
        .map(|receipt| Some(receipt.sequence))
        .map_err(append_error)
}

fn project_page(
    log: &ic_logging::Log,
    filter: ic_logging::Filter<'_>,
    request: LogPageRequest,
) -> Result<LogPage, InternalError> {
    let limit = NonZeroUsize::new(usize::from(request.limit).min(LOG_PAGE_MAX_LIMIT))
        .ok_or_else(InternalError::invalid_input)?;
    let scan = log.page(
        filter,
        ic_logging::PageRequest {
            before: request.before,
            max_results: limit,
            max_scanned: NonZeroUsize::new(LOG_SCAN_MAX as usize).expect("nonzero scan bound"),
        },
    );
    let mut entries = Vec::new();
    let mut text_bytes = 0usize;
    let mut last_returned = None;
    let mut next_before = scan.next_before;
    for entry in scan.entries {
        let row_bytes = entry
            .source
            .len()
            .saturating_add(entry.topic.as_ref().map_or(0, String::len))
            .saturating_add(entry.message.len());
        if row_bytes > LOG_PAGE_TEXT_MAX_BYTES.saturating_sub(text_bytes) {
            if entries.is_empty() {
                return Err(InternalError::public(codes::CAPACITY_LIMIT));
            }
            // Resume before the last returned row so this unreturned row is retried.
            next_before = last_returned;
            break;
        }
        text_bytes += row_bytes;
        last_returned = Some(entry.sequence);
        entries.push(project_entry(entry));
    }
    let page = LogPage {
        entries,
        scanned: u32::try_from(scan.scanned).expect("bounded upstream scan"),
        next_before,
    };
    // Check the actual maintained result frame once, after bounded scan/copy work.
    ic_wire::native::encoded_frame_len(
        &Ok::<_, canic_contracts::dto::error::Error>(&page),
        canic_contracts::protocol::protected_logs::RESPONSE_LIMITS,
    )
    .map_err(|_| InternalError::public(codes::CAPACITY_LIMIT))?;
    Ok(page)
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
        ic_logging::AppendError::LabelTooLong { .. }
        | ic_logging::AppendError::TextBudgetExceeded { .. } => codes::CAPACITY_LIMIT,
        ic_logging::AppendError::TimestampRegressed { .. } => codes::TIME_ORDERING,
        ic_logging::AppendError::SequenceExhausted => codes::VERSION_CAPACITY,
    };
    InternalError::public(code)
}
