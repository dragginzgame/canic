//! Module: observatory::view::comparison
//!
//! Responsibility: passive private interval reports and typed missing evidence.
//! Boundary: historical local evidence only, without billing or mutation authority.

use crate::observatory::view::{CostEvidenceLimitation, Observation, ObservatoryAuthorityView};
use serde::{Deserialize, Serialize};

///
/// CostComparisonFailure
///
/// Closed evidence failures exposed by host interval reports and comparison errors.
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CostComparisonFailure {
    AuthorityUnavailable,

    BindingChanged,

    BindingUnavailable,

    CounterDecreased,

    CounterWindowChanged,

    InvalidMetric,

    InvalidSnapshot,

    MissingCosts,

    MissingMetric,

    MissingParent,

    NonAdvancingWindow,

    Overflow,

    SaturatedCounter,

    SnapshotUnavailable,

    SourceWindowChanged,

    SourceWindowUnavailable,

    StaleSample,

    TimeWindowsDiffer,

    TruncatedSample,

    UnsupportedSchema,
}

///
/// CostComparisonResult
///
/// An independent interval result in the private host report; absence never means zero.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum CostComparisonResult<T> {
    Available { value: T },

    Unavailable { reason: CostComparisonFailure },
}

///
/// CycleMovementView
///
/// Host interval movement in exact signed decimal cycles, with actual source-clock bounds.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CycleMovementView {
    pub start_ns: u64,
    pub end_ns: u64,
    pub elapsed_ns: u64,
    pub cycles: String,
}

///
/// RoleCostComparisonView
///
/// Private role comparison; adjusted decrease covers known grants and does not establish burn.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RoleCostComparisonView {
    pub role: String,
    pub canister_id: String,
    #[serde(deserialize_with = "Option::deserialize")]
    pub parent_canister_id: Option<String>,
    pub balance_change: CostComparisonResult<CycleMovementView>,
    pub incoming_grants: CostComparisonResult<CycleMovementView>,
    pub outgoing_grants: CostComparisonResult<CycleMovementView>,
    pub known_grant_adjusted_decrease: CostComparisonResult<CycleMovementView>,
    pub limitations: Vec<CostEvidenceLimitation>,
}

///
/// ObservatoryComparisonView
///
/// Bounded private host comparison of two saved snapshots of the same retained Fleet binding.
///

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservatoryComparisonView {
    pub schema_version: u16,
    pub environment: String,
    pub fleet: String,
    pub recorded_authority: Observation<ObservatoryAuthorityView>,
    pub before_collected_at_unix_ms: u64,
    pub after_collected_at_unix_ms: u64,
    pub roles: Vec<RoleCostComparisonView>,
}
