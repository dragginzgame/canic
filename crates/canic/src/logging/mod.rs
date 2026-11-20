//! Bounded, incarnation-local operational logs. Lifecycle owns initialization.

#[cfg(test)]
mod tests;

use crate::method::{Contract, wire};
pub use ic_logging::Level;
use ic_logging::{EntryInput, Filter, Limits, Log, PageRequest};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, num::NonZeroUsize};

///
/// Entry
/// Operator-visible row from the current incarnation.
///

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Entry {
    /// Sequence within this incarnation.
    pub sequence: u64,
    /// IC time in seconds.
    pub created_at_secs: u64,
    /// Supplier severity.
    pub level: String,
    /// Static producer label.
    pub source: String,
    /// Bounded message with no request or reply payload.
    pub message: String,
    /// Whether the supplier truncated the message.
    pub truncated: bool,
}

///
/// Page
/// Newest-first slice of the current heap log.
///

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct Page {
    /// Canister version at initialization. Restart pagination if this changes.
    pub started_at_version: u64,
    /// At most eight entries.
    pub entries: Vec<Entry>,
    /// Exclusive cursor for the next older page.
    pub next_before: Option<u64>,
}

///
/// Logs
/// Root controller's operational log reader.
///

pub struct Logs;

impl Contract for Logs {
    const NAME: &'static str = "logs";
    const ID: u64 = wire::method_id(Self::NAME);
    const REPLY: wire::Limits = wire::Limits {
        bytes: 4096,
        nodes: 4096,
        depth: 8,
        allocation: 65536,
    };
    type Input = Option<u64>;
    type Output = Page;
}

const LOG_LIMITS: Limits = Limits {
    max_entries: NonZeroUsize::new(128).unwrap(),
    max_label_bytes: 32,
    max_message_bytes: 160,
    max_text_bytes: 8192,
};

thread_local! {
    static LOG: RefCell<Option<(u64, Log)>> = const { RefCell::new(None) };
}

/// Start the log during install or reopen. Repeated initialization preserves current rows.
pub(crate) fn initialize() {
    LOG.with_borrow_mut(|slot| {
        slot.get_or_insert_with(|| {
            (
                ic_platform_runtime::system::canister_version(),
                Log::new(LOG_LIMITS),
            )
        });
    });
}

/// Append a sanitized diagnostic. Supplier refusal or an unopened log leaves the operation alone.
pub fn record(level: Level, source: &'static str, message: &str) {
    LOG.with_borrow_mut(|slot| {
        if let Some((_, log)) = slot {
            let _ = log.append(EntryInput {
                created_at_secs: ic_platform_runtime::system::time() / 1_000_000_000,
                level,
                source,
                topic: None,
                message,
            });
        }
    });
}

/// Read without altering the supplier or scheduling work.
/// # Panics
/// Requires the owning actor to have initialized logging.
#[must_use]
pub fn page(before: Option<u64>) -> Page {
    LOG.with_borrow(|slot| {
        let limit = NonZeroUsize::new(8).unwrap();
        let (started_at_version, log) = slot.as_ref().expect("initialized log");
        let page = log.page(
            Filter::default(),
            PageRequest {
                before,
                max_results: limit,
                max_scanned: limit,
            },
        );

        Page {
            started_at_version: *started_at_version,
            next_before: page.next_before,
            entries: page
                .entries
                .into_iter()
                .map(|row| Entry {
                    sequence: row.sequence,
                    created_at_secs: row.created_at_secs,
                    level: format!("{:?}", row.level),
                    source: row.source.clone(),
                    message: row.message.clone(),
                    truncated: row.truncated,
                })
                .collect(),
        }
    })
}
