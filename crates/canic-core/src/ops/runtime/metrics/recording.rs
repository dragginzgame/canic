//! Module: ops::runtime::metrics::recording
//!
//! Responsibility: provide typed recording adapters for runtime metric counters.
//! Does not own: workflow decisions, persisted records, or endpoint DTOs.
//! Boundary: ops-layer metrics consumed by workflow metrics projection.

use crate::{
    InternalError,
    ops::runtime::metrics::{
        placement_binding::{
            PlacementBindingMetricOperation, PlacementBindingMetricOutcome,
            PlacementBindingMetricReason, PlacementBindingMetrics,
        },
        pool::{PoolMetricOperation, PoolMetricOutcome, PoolMetricReason, PoolMetrics},
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
/// PlacementBindingMetricEvent
///
/// Typed recording adapter for binding metric events.
///

pub struct PlacementBindingMetricEvent;

impl PlacementBindingMetricEvent {
    /// Record one binding metric row with an explicit outcome and reason.
    pub fn record(
        operation: PlacementBindingMetricOperation,
        outcome: PlacementBindingMetricOutcome,
        reason: PlacementBindingMetricReason,
    ) {
        PlacementBindingMetrics::record(operation, outcome, reason);
    }

    /// Record a started binding metric row.
    pub fn started(operation: PlacementBindingMetricOperation) {
        Self::record(
            operation,
            PlacementBindingMetricOutcome::Started,
            PlacementBindingMetricReason::Ok,
        );
    }

    /// Record a completed binding metric row.
    pub fn completed(
        operation: PlacementBindingMetricOperation,
        reason: PlacementBindingMetricReason,
    ) {
        Self::record(operation, PlacementBindingMetricOutcome::Completed, reason);
    }

    /// Record a skipped binding metric row.
    pub fn skipped(
        operation: PlacementBindingMetricOperation,
        reason: PlacementBindingMetricReason,
    ) {
        Self::record(operation, PlacementBindingMetricOutcome::Skipped, reason);
    }

    /// Record a failed binding metric row classified from an internal error.
    pub fn failed(operation: PlacementBindingMetricOperation, err: &InternalError) {
        Self::record(
            operation,
            PlacementBindingMetricOutcome::Failed,
            PlacementBindingMetricReason::from_error(err),
        );
    }

    /// Record a failed binding metric row with an explicit bounded reason.
    pub fn failed_reason(
        operation: PlacementBindingMetricOperation,
        reason: PlacementBindingMetricReason,
    ) {
        Self::record(operation, PlacementBindingMetricOutcome::Failed, reason);
    }
}

///
/// PoolMetricEvent
///
/// Typed recording adapter for pool metric events.
///

pub struct PoolMetricEvent;

impl PoolMetricEvent {
    /// Record one pool metric row with an explicit outcome and reason.
    pub fn record(
        operation: PoolMetricOperation,
        outcome: PoolMetricOutcome,
        reason: PoolMetricReason,
    ) {
        PoolMetrics::record(operation, outcome, reason);
    }

    /// Record a started pool metric row.
    pub fn started(operation: PoolMetricOperation) {
        Self::record(operation, PoolMetricOutcome::Started, PoolMetricReason::Ok);
    }

    /// Record a completed pool metric row.
    pub fn completed(operation: PoolMetricOperation, reason: PoolMetricReason) {
        Self::record(operation, PoolMetricOutcome::Completed, reason);
    }

    /// Record a skipped pool metric row.
    pub fn skipped(operation: PoolMetricOperation, reason: PoolMetricReason) {
        Self::record(operation, PoolMetricOutcome::Skipped, reason);
    }

    /// Record a failed pool metric row classified from an internal error.
    pub fn failed(operation: PoolMetricOperation, err: &InternalError) {
        Self::record(
            operation,
            PoolMetricOutcome::Failed,
            PoolMetricReason::from_error(err),
        );
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
