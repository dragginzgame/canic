//! Module: dto::public_status
//!
//! Responsibility: carry public summary health and bounded published metric snapshots.
//! Does not own: collection, publication policy, or authority.
//! Boundary: every field is intended for anonymous callers.

use crate::{
    domain::runtime::HealthStatus,
    dto::{
        page::{Page, PageRequest},
        prelude::*,
    },
};

pub use crate::domain::public_metrics::PublicMetricFamily;

/// Minimal local identity and health without diagnostic checks or failure details.
#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct PublicHealth {
    pub canister_id: Principal,
    pub role: Option<String>,
    pub health: HealthStatus,
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
