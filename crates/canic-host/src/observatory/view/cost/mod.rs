//! Module: observatory::view::cost
//!
//! Responsibility: passive private cost-evidence views for host snapshots.
//! Boundary: source measurements and limitations, without cost conversion or collection.

use crate::observatory::view::{MetricSamplesView, Observation};
use serde::{Deserialize, Serialize};

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
    pub balance: Observation<MetricSamplesView>,
    pub funding_and_callbacks: Observation<MetricSamplesView>,
    pub timer_instructions: Observation<MetricSamplesView>,
    pub window: Observation<CostWindowView>,
    pub limitations: Vec<CostEvidenceLimitation>,
}
