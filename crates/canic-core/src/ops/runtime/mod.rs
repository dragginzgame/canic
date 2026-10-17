//! Module: ops::runtime
//!
//! Responsibility: group runtime operations used by workflow.
//! Does not own: domain policy, endpoint authorization, or stable schemas.
//! Boundary: exposes ops-layer runtime facades and their typed error surface.

pub mod bootstrap;
pub mod cycles_funding;
pub mod env;
pub mod fleet_activation;
pub mod funding_usage;
pub mod init_payload;
pub mod install_source;
pub mod log;
pub mod memory;
pub mod metrics;
pub mod public_metrics;
pub mod ready;
pub mod recent_failure;
pub mod release_intents;
pub mod release_receipts;
pub mod root_funding;

use crate::{InternalError, domain::runtime::TimerProcessCondition};
use ic_timers::{
    InactiveReason, TimerCompletionOutcome, TimerLastOutcome, TimerRuntimeStateSnapshot,
    TimerSchedulingMode,
};
use thiserror::Error as ThisError;

/// Project Timer execution facts into Canic's operator-health condition.
/// Status and anonymous metrics share this policy.
pub const fn timer_process_condition(
    state: TimerRuntimeStateSnapshot,
    mode: TimerSchedulingMode,
    outcome: Option<TimerLastOutcome>,
) -> TimerProcessCondition {
    match state {
        TimerRuntimeStateSnapshot::Inactive {
            reason: InactiveReason::Cancelled,
        } => TimerProcessCondition::Disabled,
        TimerRuntimeStateSnapshot::Inactive {
            reason:
                InactiveReason::Abandoned
                | InactiveReason::InvariantFailure
                | InactiveReason::ControlFailure(_),
        } => TimerProcessCondition::Failed,
        TimerRuntimeStateSnapshot::Inactive {
            reason: InactiveReason::Stopped,
        } if matches!(
            outcome,
            Some(TimerLastOutcome::Completed(
                TimerCompletionOutcome::RetryableFailure
            ))
        ) =>
        {
            TimerProcessCondition::Failed
        }
        TimerRuntimeStateSnapshot::Inactive { .. } => TimerProcessCondition::Idle,
        TimerRuntimeStateSnapshot::Ordinary(_) | TimerRuntimeStateSnapshot::Watchdog(_)
            if matches!(mode, TimerSchedulingMode::Retry) =>
        {
            TimerProcessCondition::Retrying
        }
        TimerRuntimeStateSnapshot::Ordinary(_) | TimerRuntimeStateSnapshot::Watchdog(_) => {
            TimerProcessCondition::Active
        }
    }
}

///
/// RuntimeOpsError
///
/// Typed failure surface for runtime operation facades.
///

#[derive(Debug, ThisError)]
pub enum RuntimeOpsError {
    #[error(transparent)]
    EnvOps(#[from] env::EnvOpsError),

    #[error(transparent)]
    LogStorage(#[from] crate::storage::StorageError),

    #[error(transparent)]
    MemoryRegistryOps(#[from] memory::MemoryRegistryOpsError),
}

impl From<RuntimeOpsError> for InternalError {
    fn from(err: RuntimeOpsError) -> Self {
        match err {
            RuntimeOpsError::EnvOps(err) => err.into(),
            RuntimeOpsError::LogStorage(err) => err.into(),
            RuntimeOpsError::MemoryRegistryOps(err) => err.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ic_timers::{
        OrdinaryRuntimeStateSnapshot, TimerControlFailure, WatchdogRuntimeStateSnapshot,
    };

    #[test]
    fn timer_health_distinguishes_inactivity_failure_and_retry() {
        for (reason, expected) in [
            (InactiveReason::NeverScheduled, TimerProcessCondition::Idle),
            (InactiveReason::Stopped, TimerProcessCondition::Idle),
            (InactiveReason::Cancelled, TimerProcessCondition::Disabled),
            (InactiveReason::Abandoned, TimerProcessCondition::Failed),
            (
                InactiveReason::InvariantFailure,
                TimerProcessCondition::Failed,
            ),
            (
                InactiveReason::ControlFailure(TimerControlFailure::GenerationExhausted),
                TimerProcessCondition::Failed,
            ),
        ] {
            assert_eq!(
                timer_process_condition(
                    TimerRuntimeStateSnapshot::Inactive { reason },
                    TimerSchedulingMode::Retry,
                    None
                ),
                expected
            );
        }
        assert_eq!(
            timer_process_condition(
                TimerRuntimeStateSnapshot::Inactive {
                    reason: InactiveReason::Stopped
                },
                TimerSchedulingMode::AfterCompletion,
                Some(TimerLastOutcome::Completed(
                    TimerCompletionOutcome::RetryableFailure
                ))
            ),
            TimerProcessCondition::Failed
        );
        for state in [
            TimerRuntimeStateSnapshot::Ordinary(OrdinaryRuntimeStateSnapshot::Running {
                generation: 1,
            }),
            TimerRuntimeStateSnapshot::Watchdog(WatchdogRuntimeStateSnapshot::Scheduled {
                scheduler_generation: 1,
                deadline_ns: 1,
            }),
        ] {
            assert_eq!(
                timer_process_condition(state, TimerSchedulingMode::Retry, None),
                TimerProcessCondition::Retrying
            );
            assert_eq!(
                timer_process_condition(state, TimerSchedulingMode::AfterCompletion, None),
                TimerProcessCondition::Active
            );
        }
    }
}
