//! Bounded canister diagnostics, independent of recording engines and durable authority.

#[cfg(all(test, feature = "method"))]
mod tests;

#[cfg(feature = "method")]
mod endpoint;
#[cfg(feature = "method")]
pub use endpoint::set_guard;
#[cfg(feature = "timers")]
pub(crate) use endpoint::start_timer_window;

#[cfg(feature = "method")]
pub mod performance;
#[cfg(feature = "method")]
mod protocol;
#[cfg(feature = "method")]
pub use protocol::{BlobCommand, BlobSizes, ObserveChildMetrics, ReadMetrics, Report, Request};

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

// Wire encodes the binary u128. Only human-readable serializers use decimal text.
#[cfg(feature = "inspection")]
impl ic_wire::typescript::Wire for Counter {
    fn schema(
        context: &mut ic_wire::typescript::SchemaContext,
    ) -> Result<serde_json::Value, ic_wire::typescript::GenerationError> {
        Ok(serde_json::json!({
            "k": "record",
            "fields": [
                {"name": "total", "value": context.describe::<u128>()?},
                {"name": "overflowed", "value": context.describe::<bool>()?}
            ]
        }))
    }
}

#[cfg(feature = "method")]
impl From<ic_metrics::MeasurementCounter> for Counter {
    fn from(counter: ic_metrics::MeasurementCounter) -> Self {
        Self {
            total: counter.total(),
            overflowed: counter.total_overflowed(),
        }
    }
}

// Wire encodes the binary u128. Only human-readable serializers use decimal text.
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
#[cfg_attr(feature = "inspection", derive(ic_wire::typescript::Wire))]
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
#[cfg_attr(feature = "inspection", derive(ic_wire::typescript::Wire))]
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Snapshot {
    /// IC Unix nanoseconds at observation, supplied by the endpoint owner.
    pub observed_at_ns: u64,
    /// Selected supplier's aggregate report; no operation IDs, callers or private topology.
    pub timers: Option<Timers>,
}
