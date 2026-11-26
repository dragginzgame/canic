//! Bounded, incarnation-local operational logs. Lifecycle owns initialization.

#[cfg(test)]
mod tests;

use crate::method::{Contract, wire};
pub use ic_logging::Level;
use ic_logging::{EntryInput, Filter, Log, PageRequest, RetainedByteLimits};
use serde::{Deserialize, Serialize};
use std::{
    cell::{OnceCell, RefCell},
    num::NonZeroUsize,
};

///
/// Entry
/// Operator-visible row from the current incarnation.
///

#[cfg_attr(feature = "inspection", derive(ic_wire::typescript::Wire))]
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

#[cfg_attr(feature = "inspection", derive(ic_wire::typescript::Wire))]
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
/// Controller's operational log reader.
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

const DEFAULT_LOG_BYTES: usize = 512 * 1024;
const LOG_LIMITS: RetainedByteLimits = RetainedByteLimits {
    max_retained_bytes: DEFAULT_LOG_BYTES,
    max_label_bytes: 32,
    max_message_bytes: 160,
};

type ActorLog = Log<RetainedByteLimits>;

thread_local! {
    static LOG: OnceCell<(u64, RefCell<ActorLog>)> = const { OnceCell::new() };
}

fn with_log<T>(read: impl FnOnce(u64, &RefCell<ActorLog>) -> T) -> T {
    LOG.with(|slot| {
        let (version, log) = slot.get_or_init(|| {
            (
                ic_platform_runtime::system::canister_version(),
                RefCell::new(Log::with_retained_byte_limit(LOG_LIMITS)),
            )
        });
        read(*version, log)
    })
}

/// Open the log at the lifecycle boundary. Custom actors can also open it on first use.
#[cfg(feature = "lifecycle")]
pub(crate) fn initialize() {
    with_log(|_, _| ());
}

/// Append a sanitized diagnostic. The first write opens the log for actors with custom lifecycle hooks.
pub fn record(level: Level, source: &'static str, message: &str) {
    with_log(|_, log| {
        let _ = log.borrow_mut().append(EntryInput {
            created_at_secs: ic_platform_runtime::system::time() / 1_000_000_000,
            level,
            source,
            topic: None,
            message,
        });
    });
}

/// Read without altering the supplier or scheduling work.
#[must_use]
pub fn page(before: Option<u64>) -> Page {
    with_log(|started_at_version, log| {
        const LIMIT: NonZeroUsize = NonZeroUsize::new(8).unwrap();
        let log = log.borrow();
        let page = log.page(
            Filter::default(),
            PageRequest {
                before,
                max_results: LIMIT,
                max_scanned: LIMIT,
            },
        );

        Page {
            started_at_version,
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

/// Accounted row metadata and retained text bytes, in constant time.
#[must_use]
pub fn retained_bytes() -> usize {
    with_log(|_, log| log.borrow().retained_bytes())
}

#[crate::update(contract = Logs, guard = crate::guard::require_controller)]
fn logs(before: Option<u64>) -> Page {
    page(before)
}
