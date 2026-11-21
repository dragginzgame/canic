//! Bounded canister diagnostics, independent of recording engines and durable authority.

#[cfg(all(test, feature = "method"))]
mod tests;

#[cfg(feature = "metrics")]
mod endpoint;
#[cfg(feature = "metrics")]
pub use endpoint::set_guard;
#[cfg(all(feature = "metrics", feature = "timers"))]
pub(crate) use endpoint::start_timer_window;

#[cfg(feature = "method")]
pub mod performance;
#[cfg(feature = "method")]
mod protocol;
#[cfg(feature = "method")]
pub use protocol::{BlobCommand, BlobSizes, ChildMetrics, ReadMetrics, Report, Request};

pub mod store;

use serde::{Deserialize, Serialize};

/// Supplier total with its original overflow quality. No arithmetic or recording lives here.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Counter {
    /// Exact until overflow; decimal text in JSON to avoid browser integer precision loss.
    #[serde(with = "total")]
    pub total: u128,
    /// Sticky supplier overflow flag; a clipped total must not be presented as exact.
    pub overflowed: bool,
}

mod total {
    use serde::{Deserialize, Deserializer, Serializer, de::Error as _};

    pub fn serialize<S: Serializer>(value: &u128, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            serializer.serialize_str(&value.to_string())
        } else {
            serializer.serialize_u128(*value)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u128, D::Error> {
        if deserializer.is_human_readable() {
            String::deserialize(deserializer)?
                .parse()
                .map_err(D::Error::custom)
        } else {
            u128::deserialize(deserializer)
        }
    }
}

/// Volatile aggregate counters for one timer runtime incarnation, reset on upgrade.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Timers {
    /// Canister version captured at lifecycle initialization, not at query time.
    pub started_at_version: u64,
    /// IC Unix nanoseconds at that same initialization.
    pub started_at_ns: u64,
    /// Logical acceptances, not proof of invocation or business effects.
    pub accepted: Counter,
    /// Callbacks that returned normally, including no-work and expected-failure returns.
    pub completed: Counter,
    /// Confirmed abandoned callbacks.
    pub abandoned: Counter,
    /// Normal returns classified as expected domain failures; no retry implied.
    pub expected_failures: Counter,
    /// Self-call submissions refused before acceptance.
    pub deferred_submissions: Counter,
    /// Terminal provider faults in this incarnation.
    pub provider_faults: Counter,
}

/// Uncertified, same-query observations. Absence means unavailable, never zero or healthy.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Snapshot {
    /// IC Unix nanoseconds at observation, supplied by the endpoint owner.
    pub observed_at_ns: u64,
    /// Selected supplier's aggregate report; no operation IDs, callers or private topology.
    pub timers: Option<Timers>,
}
