//! Cached application and cost measurements, without inferred totals or billing.

use serde::{Deserialize, Serialize};

///
/// MetricSampleState
///
/// Cached source state in host reports, independent of reply receipt time.
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricSampleState {
    Disabled,
    Unavailable,
    Fresh,
    Stale,
}

///
/// MetricKind
///
/// Host interpretation; counter windows require the same source heap epoch.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MetricKind {
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
/// MetricView
///
/// Exact source value in host JSON, with decimal strings preserving integer precision.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MetricView {
    pub name: String,
    #[serde(deserialize_with = "Option::deserialize")]
    pub canister_id: Option<String>,
    pub value: String,
    pub unit: String,
    pub observed_at_ns: u64,
    pub measurement: MetricKind,
}

///
/// MetricSamplesView
///
/// Bounded source page in the host report; missing or truncated rows never mean zero activity.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MetricSamplesView {
    pub state: MetricSampleState,
    #[serde(deserialize_with = "Option::deserialize")]
    pub sampled_at_ns: Option<u64>,
    pub stale_after_ns: u64,
    pub truncated: bool,
    pub rows: Vec<MetricView>,
}
