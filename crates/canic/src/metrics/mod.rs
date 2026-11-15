//! Bounded public diagnostics, independent of recording engines and durable authority.
use serde::{Deserialize, Serialize};

#[cfg(all(feature = "metrics", target_arch = "wasm32"))]
pub mod endpoint;
#[cfg(test)]
mod tests;

/// Shared read-only metrics query. This version reports aggregate timer counters only.
pub const METHOD: u64 = 2;
/// Empty request, bounded before allocation.
pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 32,
    allocation: 256,
    nodes: 4,
    depth: 4,
};
/// Six fixed counters and reset metadata; no arbitrary labels or declaration inventory.
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 1024,
    // Wire charges each visited native record (including nested records) conservatively.
    allocation: 32_768,
    nodes: 128,
    depth: 8,
};

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

/// Mount a public, bounded query using an explicitly supplied local snapshot function.
/// Mounting initializes no provider, storage, timer, sampler or persistent history.
#[cfg(all(feature = "metrics", target_arch = "wasm32"))]
#[macro_export]
macro_rules! mount_metrics_endpoint {
    ($snapshot:path) => {
        mod canic_metrics {
            use endpoint::cdk;
            use $crate::metrics::{Snapshot, endpoint};
            #[cdk::query(
                crate = "cdk",
                name = "metrics",
                guard = "endpoint::admit",
                decode_with = "endpoint::decode",
                encode_with = "endpoint::encode"
            )]
            fn metrics() -> Snapshot {
                $snapshot()
            }
        }
    };
}
