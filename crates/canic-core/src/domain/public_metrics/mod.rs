//! Module: domain::public_metrics
//!
//! Responsibility: identify explicitly publishable aggregate metric families.
//! Does not own: caller authorization, collection, or snapshot storage.
//! Boundary: shared selectors for configuration and read contracts.

use candid::CandidType;
use serde::{Deserialize, Serialize};

/// Aggregate families an App may explicitly publish to every caller.
#[derive(
    CandidType, Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize,
)]
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
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PublicMetricKind {
    Gauge,
    Counter { window_id: u64, saturated: bool },
}
