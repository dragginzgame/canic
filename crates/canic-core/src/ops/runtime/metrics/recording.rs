//! Module: ops::runtime::metrics::recording
//!
//! Responsibility: provide typed recording adapters for runtime metric counters.
//! Does not own: workflow decisions, persisted records, or endpoint DTOs.
//! Boundary: ops-layer metrics consumed by workflow metrics projection.

use crate::{
    InternalError,
    ops::runtime::metrics::{
        placement_index::{
            PlacementIndexMetricOperation, PlacementIndexMetricOutcome, PlacementIndexMetricReason,
            PlacementIndexMetrics,
        },
        scaling::{
            ScalingMetricOperation, ScalingMetricOutcome, ScalingMetricReason, ScalingMetrics,
        },
    },
};

#[cfg(feature = "sharding")]
use crate::ops::runtime::metrics::sharding::{
    ShardingMetricOperation, ShardingMetricOutcome, ShardingMetricReason, ShardingMetrics,
};

///
/// PlacementIndexMetricEvent
///
/// Typed recording adapter for Placement Index metric events.
///

pub struct PlacementIndexMetricEvent;

impl PlacementIndexMetricEvent {
    /// Record one Placement Index metric row with an explicit outcome and reason.
    pub fn record(
        operation: PlacementIndexMetricOperation,
        outcome: PlacementIndexMetricOutcome,
        reason: PlacementIndexMetricReason,
    ) {
        PlacementIndexMetrics::record(operation, outcome, reason);
    }

    /// Record a started Placement Index metric row.
    pub fn started(operation: PlacementIndexMetricOperation) {
        Self::record(
            operation,
            PlacementIndexMetricOutcome::Started,
            PlacementIndexMetricReason::Ok,
        );
    }

    /// Record a completed Placement Index metric row.
    pub fn completed(operation: PlacementIndexMetricOperation, reason: PlacementIndexMetricReason) {
        Self::record(operation, PlacementIndexMetricOutcome::Completed, reason);
    }

    /// Record a skipped Placement Index metric row.
    pub fn skipped(operation: PlacementIndexMetricOperation, reason: PlacementIndexMetricReason) {
        Self::record(operation, PlacementIndexMetricOutcome::Skipped, reason);
    }

    /// Record a failed Placement Index metric row classified from an internal error.
    pub fn failed(operation: PlacementIndexMetricOperation, err: &InternalError) {
        Self::record(
            operation,
            PlacementIndexMetricOutcome::Failed,
            PlacementIndexMetricReason::from_error(err),
        );
    }

    /// Record a failed Placement Index metric row with an explicit bounded reason.
    pub fn failed_reason(
        operation: PlacementIndexMetricOperation,
        reason: PlacementIndexMetricReason,
    ) {
        Self::record(operation, PlacementIndexMetricOutcome::Failed, reason);
    }
}

///
/// ScalingMetricEvent
///
/// Typed recording adapter for scaling metric events.
///

pub struct ScalingMetricEvent;

impl ScalingMetricEvent {
    /// Record one scaling metric row with an explicit outcome and reason.
    pub fn record(
        operation: ScalingMetricOperation,
        outcome: ScalingMetricOutcome,
        reason: ScalingMetricReason,
    ) {
        ScalingMetrics::record(operation, outcome, reason);
    }

    /// Record a started scaling metric row.
    pub fn started(operation: ScalingMetricOperation) {
        Self::record(
            operation,
            ScalingMetricOutcome::Started,
            ScalingMetricReason::Ok,
        );
    }

    /// Record a completed scaling metric row.
    pub fn completed(operation: ScalingMetricOperation, reason: ScalingMetricReason) {
        Self::record(operation, ScalingMetricOutcome::Completed, reason);
    }

    /// Record a skipped scaling metric row.
    pub fn skipped(operation: ScalingMetricOperation, reason: ScalingMetricReason) {
        Self::record(operation, ScalingMetricOutcome::Skipped, reason);
    }

    /// Record a failed scaling metric row classified from an internal error.
    pub fn failed(operation: ScalingMetricOperation, err: &InternalError) {
        Self::record(
            operation,
            ScalingMetricOutcome::Failed,
            ScalingMetricReason::from_error(err),
        );
    }

    /// Record a failed scaling metric row with an explicit bounded reason.
    pub fn failed_reason(operation: ScalingMetricOperation, reason: ScalingMetricReason) {
        Self::record(operation, ScalingMetricOutcome::Failed, reason);
    }
}

///
/// ShardingMetricEvent
///
/// Typed recording adapter for sharding metric events.
///

#[cfg(feature = "sharding")]
pub struct ShardingMetricEvent;

#[cfg(feature = "sharding")]
impl ShardingMetricEvent {
    /// Record one sharding metric row with an explicit outcome and reason.
    pub fn record(
        operation: ShardingMetricOperation,
        outcome: ShardingMetricOutcome,
        reason: ShardingMetricReason,
    ) {
        ShardingMetrics::record(operation, outcome, reason);
    }

    /// Record a started sharding metric row.
    pub fn started(operation: ShardingMetricOperation) {
        Self::record(
            operation,
            ShardingMetricOutcome::Started,
            ShardingMetricReason::Ok,
        );
    }

    /// Record a completed sharding metric row.
    pub fn completed(operation: ShardingMetricOperation, reason: ShardingMetricReason) {
        Self::record(operation, ShardingMetricOutcome::Completed, reason);
    }

    /// Record a skipped sharding metric row.
    pub fn skipped(operation: ShardingMetricOperation, reason: ShardingMetricReason) {
        Self::record(operation, ShardingMetricOutcome::Skipped, reason);
    }

    /// Record a failed sharding metric row classified from an internal error.
    pub fn failed(operation: ShardingMetricOperation, err: &InternalError) {
        Self::record(
            operation,
            ShardingMetricOutcome::Failed,
            ShardingMetricReason::from_error(err),
        );
    }

    /// Record a failed sharding metric row with an explicit bounded reason.
    pub fn failed_reason(operation: ShardingMetricOperation, reason: ShardingMetricReason) {
        Self::record(operation, ShardingMetricOutcome::Failed, reason);
    }
}
