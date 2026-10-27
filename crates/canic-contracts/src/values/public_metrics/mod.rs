//! Module: domain::public_metrics
//!
//! Responsibility: identify explicitly publishable aggregate metric families.
//! Does not own: caller authorization, collection, or snapshot storage.
//! Boundary: shared selectors for configuration and read contracts.

use serde::{Deserialize, Serialize};

#[cfg(all(feature = "wire-schema", not(target_arch = "wasm32")))]
mod schema;

/// Aggregate families an App may explicitly publish to every caller.

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum PublicMetricFamily {
    #[serde(rename = "application")]
    Application,
    #[serde(rename = "cycles")]
    Cycles,
    #[serde(rename = "operations")]
    Operations,
    #[serde(rename = "performance")]
    Performance,
    #[serde(rename = "shard_occupancy")]
    ShardOccupancy,
}

/// Interpretation of a sampled value. Counter windows must change on every reset.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub enum PublicMetricKind {
    Gauge,
    Counter {
        window_id: u64,
        saturated: bool,
    },
    TimerCounter {
        registration: TimerMetricRegistration,
        saturated: bool,
    },
}

/// Source-owned counter lifetime, scoped to one canister's observed runtime history.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(
    all(feature = "wire-schema", not(target_arch = "wasm32")),
    derive(ic_wire::Wire)
)]
pub struct TimerMetricRegistration {
    pub canister_version: u64,
    pub started_at_ns: u64,
    pub sequence: u64,
}
