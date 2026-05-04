use crate::InternalError;

use super::{
    directory::{
        DirectoryMetricOperation, DirectoryMetricOutcome, DirectoryMetricReason, DirectoryMetrics,
    },
    pool::{PoolMetricOperation, PoolMetricOutcome, PoolMetricReason, PoolMetrics},
    scaling::{ScalingMetricOperation, ScalingMetricOutcome, ScalingMetricReason, ScalingMetrics},
};

#[cfg(feature = "sharding")]
use super::sharding::{
    ShardingMetricOperation, ShardingMetricOutcome, ShardingMetricReason, ShardingMetrics,
};

///
/// DirectoryMetricEvent
///

pub struct DirectoryMetricEvent;

impl DirectoryMetricEvent {
    /// Record one directory metric row with an explicit outcome and reason.
    pub fn record(
        operation: DirectoryMetricOperation,
        outcome: DirectoryMetricOutcome,
        reason: DirectoryMetricReason,
    ) {
        DirectoryMetrics::record(operation, outcome, reason);
    }

    /// Record a started directory metric row.
    pub fn started(operation: DirectoryMetricOperation) {
        Self::record(
            operation,
            DirectoryMetricOutcome::Started,
            DirectoryMetricReason::Ok,
        );
    }

    /// Record a completed directory metric row.
    pub fn completed(operation: DirectoryMetricOperation, reason: DirectoryMetricReason) {
        Self::record(operation, DirectoryMetricOutcome::Completed, reason);
    }

    /// Record a skipped directory metric row.
    pub fn skipped(operation: DirectoryMetricOperation, reason: DirectoryMetricReason) {
        Self::record(operation, DirectoryMetricOutcome::Skipped, reason);
    }

    /// Record a failed directory metric row classified from an internal error.
    pub fn failed(operation: DirectoryMetricOperation, err: &InternalError) {
        Self::record(
            operation,
            DirectoryMetricOutcome::Failed,
            DirectoryMetricReason::from_error(err),
        );
    }

    /// Record a failed directory metric row with an explicit bounded reason.
    pub fn failed_reason(operation: DirectoryMetricOperation, reason: DirectoryMetricReason) {
        Self::record(operation, DirectoryMetricOutcome::Failed, reason);
    }
}

///
/// PoolMetricEvent
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
