//! Module: dto::public_status
//!
//! Responsibility: carry public responsiveness and bounded published metric snapshots.
//! Does not own: collection, publication policy, or authority.
//! Boundary: every field is intended for anonymous callers.

use crate::dto::{
    page::{Page, PageRequest},
    prelude::*,
};

pub use crate::domain::public_metrics::{PublicMetricFamily, PublicMetricKind};

/// Public query responsiveness; no runtime diagnostics or Fleet readiness assessment.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum PublicHealthStatus {
    #[serde(rename = "responding")]
    Responding,
}

/// Local identity and responsiveness without diagnostic checks or Fleet readiness.
#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct PublicHealth {
    pub canister_id: Principal,
    pub role: Option<String>,
    pub health: PublicHealthStatus,
    pub observed_at_ns: u64,
}

/// Select a published aggregate family and one bounded page.
#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct PublicMetricsRequest {
    pub family: PublicMetricFamily,
    pub page: PageRequest,
}

/// An explicitly named aggregate value with an optional canister dimension.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct PublicMetric {
    pub name: String,
    pub canister_id: Option<Principal>,
    pub value: u128,
    pub unit: String,
    pub observed_at_ns: u64,
    pub kind: PublicMetricKind,
}

/// Availability is data, independent of caller identity.
#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum PublicSnapshotState {
    Disabled,
    Unavailable,
    Fresh,
    Stale,
}

/// One cached aggregate snapshot. Reading it never refreshes it.
#[derive(CandidType, Debug, Deserialize)]
pub struct PublicMetricsSnapshot {
    pub family: PublicMetricFamily,
    pub state: PublicSnapshotState,
    pub sampled_at_ns: Option<u64>,
    pub stale_after_ns: u64,
    pub truncated: bool,
    pub metrics: Page<PublicMetric>,
}

/// Select one exact cached series; points are ordered by five-minute slot.
#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct PublicHistoryRequest {
    pub family: PublicMetricFamily,
    pub name: String,
    pub canister_id: Option<Principal>,
    pub page: PageRequest,
}

/// One actual observation, never an interpolated or replayed sample.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct PublicHistoryPoint {
    pub delta: Option<PublicCounterDelta>,
    pub slot_start_ns: u64,
    pub observed_at_ns: u64,
    pub value: u128,
    pub kind: PublicMetricKind,
}

/// Heap-only chart coverage. Missing slots are gaps, including across suspension.
#[derive(CandidType, Debug, Deserialize)]
pub struct PublicHistorySnapshot {
    pub state: PublicSnapshotState,
    pub unit: Option<String>,
    pub heap_started_at_ns: Option<u64>,
    pub canister_version: u64,
    pub coverage_start_ns: Option<u64>,
    pub cadence_ns: u64,
    pub retention_ns: u64,
    pub stale_after_ns: u64,
    pub truncated: bool,
    pub series_limit: u64,
    pub byte_limit: u64,
    pub reserved_bytes: u64,
    pub points: Page<PublicHistoryPoint>,
}

/// Comparable cumulative-counter movement across adjacent slots in the same window.
#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct PublicCounterDelta {
    pub amount: u128,
    pub elapsed_ns: u64,
}
