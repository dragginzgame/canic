//! Module: observatory::view::cost
//!
//! Responsibility: passive private cost-evidence views for host snapshots.
//! Boundary: source measurements and limitations, without cost conversion or collection.

use crate::observatory::view::Observation;
use serde::{Deserialize, Serialize};

///
/// CostSampleState
///
/// Cached source state in private host reports, independent of reply receipt time.
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CostSampleState {
    Disabled,
    Unavailable,
    Fresh,
    Stale,
}

///
/// CostMetricKind
///
/// Private host interpretation; counter windows require the same source heap epoch.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CostMetricKind {
    Gauge,
    Counter {
        window_id: u64,
        saturated: bool,
    },
    TimerCounter {
        registration: TimerRegistrationView,
        saturated: bool,
    },
}

///
/// TimerRegistrationView
///
/// Source-owned lifetime for timer measurements in one observed canister history.
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TimerRegistrationView {
    pub canister_version: u64,
    pub started_at_ns: u64,
    pub sequence: u64,
}

///
/// CostMetricView
///
/// Exact source value in private host JSON, with decimal strings preserving cycle precision.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CostMetricView {
    pub name: String,
    #[serde(deserialize_with = "Option::deserialize")]
    pub canister_id: Option<String>,
    pub value: String,
    pub unit: String,
    pub observed_at_ns: u64,
    pub measurement: CostMetricKind,
}

///
/// CostSamplesView
///
/// Bounded source page in the host report; missing or truncated rows never mean zero activity.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CostSamplesView {
    pub state: CostSampleState,
    #[serde(deserialize_with = "Option::deserialize")]
    pub sampled_at_ns: Option<u64>,
    pub stale_after_ns: u64,
    pub truncated: bool,
    pub rows: Vec<CostMetricView>,
}

///
/// CostWindowView
///
/// Host report's runtime restart anchor, read after the families; no timer registration identity.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CostWindowView {
    pub canister_version: u64,
    #[serde(deserialize_with = "Option::deserialize")]
    pub heap_started_at_ns: Option<u64>,
}

///
/// CostEvidenceLimitation
///
/// Host report's reasons the evidence cannot establish billed cost or per-timer savings.
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CostEvidenceLimitation {
    SingleSnapshot,
    SourceWindowChanged,
    SourceWindowUnavailable,
    AggregateTimerCallbacksUnqualified,
    TransferCoverageIncomplete,
    UnattributedExecutionMessageStorage,
}

///
/// RoleCostEvidenceView
///
/// Optional host snapshot evidence from existing caches, with independent family failures.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RoleCostEvidenceView {
    pub balance: Observation<CostSamplesView>,
    pub funding_and_callbacks: Observation<CostSamplesView>,
    pub timer_instructions: Observation<CostSamplesView>,
    pub window: Observation<CostWindowView>,
    pub limitations: Vec<CostEvidenceLimitation>,
}
