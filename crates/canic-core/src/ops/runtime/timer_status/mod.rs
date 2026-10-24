//! Module: ops::runtime::timer_status
//!
//! Responsibility: project timer state into Canic operator-health labels.
//! Does not own: scheduling, callback authority or timer state mutation.
//! Boundary: runtime status and public metrics consume the same projection.

use canic_contracts::values::runtime::TimerProcessCondition;
use ic_timers::{
    InactiveReason, TimerCompletionOutcome, TimerLastOutcome, TimerRuntimeStateSnapshot,
    TimerSchedulingMode, TimerSnapshot,
};

/// Derive operator health from one coherent upstream snapshot.
pub const fn condition(snapshot: &TimerSnapshot) -> TimerProcessCondition {
    from_values(
        snapshot.state(),
        snapshot.scheduling_mode(),
        snapshot.observability().outcomes().last_outcome(),
    )
}

/// Project only exact counters representable by Canic's u64 reporting contracts.
pub fn counter_total(counters: &[ic_timers::MeasurementCounter]) -> Option<u64> {
    counters.iter().try_fold(0_u64, |total, counter| {
        total.checked_add(u64::try_from(counter.exact_total().ok()?).ok()?)
    })
}

const fn from_values(
    state: TimerRuntimeStateSnapshot,
    mode: TimerSchedulingMode,
    outcome: Option<TimerLastOutcome>,
) -> TimerProcessCondition {
    use TimerProcessCondition as Condition;
    match state {
        TimerRuntimeStateSnapshot::Inactive {
            reason: InactiveReason::Cancelled,
        } => Condition::Disabled,
        TimerRuntimeStateSnapshot::Inactive {
            reason:
                InactiveReason::Abandoned
                | InactiveReason::InvariantFailure
                | InactiveReason::ControlFailure(_),
        } => Condition::Failed,
        TimerRuntimeStateSnapshot::Inactive {
            reason: InactiveReason::Stopped,
        } if matches!(
            outcome,
            Some(TimerLastOutcome::Completed(
                TimerCompletionOutcome::RetryableFailure
            ))
        ) =>
        {
            Condition::Failed
        }
        TimerRuntimeStateSnapshot::Inactive { .. } => Condition::Idle,
        TimerRuntimeStateSnapshot::Ordinary(_) | TimerRuntimeStateSnapshot::Watchdog(_)
            if matches!(mode, TimerSchedulingMode::Retry) =>
        {
            Condition::Retrying
        }
        TimerRuntimeStateSnapshot::Ordinary(_) | TimerRuntimeStateSnapshot::Watchdog(_) => {
            Condition::Active
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ic_timers::OrdinaryRuntimeStateSnapshot;

    #[test]
    fn counter_projection_refuses_overflow_and_narrowing() {
        use ic_timers::MeasurementCounter;
        let exact = |total| MeasurementCounter::from_parts(total, false).unwrap();
        assert_eq!(counter_total(&[exact(4), exact(5)]), Some(9));
        assert_eq!(
            counter_total(&[exact(u128::from(u64::MAX))]),
            Some(u64::MAX)
        );
        assert_eq!(counter_total(&[exact(u128::from(u64::MAX) + 1)]), None);
        assert_eq!(
            counter_total(&[exact(u128::from(u64::MAX)), exact(1)]),
            None
        );
        assert_eq!(
            counter_total(&[MeasurementCounter::from_parts(u128::MAX, true).unwrap()]),
            None
        );
    }

    #[test]
    fn inactive_reasons_and_retry_authority_determine_health() {
        let failure = Some(TimerLastOutcome::Completed(
            TimerCompletionOutcome::RetryableFailure,
        ));
        for (reason, outcome, expected) in [
            (
                InactiveReason::Cancelled,
                failure,
                TimerProcessCondition::Disabled,
            ),
            (
                InactiveReason::NeverScheduled,
                None,
                TimerProcessCondition::Idle,
            ),
            (InactiveReason::Stopped, None, TimerProcessCondition::Idle),
            (
                InactiveReason::Stopped,
                failure,
                TimerProcessCondition::Failed,
            ),
            (
                InactiveReason::Abandoned,
                None,
                TimerProcessCondition::Failed,
            ),
            (
                InactiveReason::InvariantFailure,
                None,
                TimerProcessCondition::Failed,
            ),
        ] {
            assert_eq!(
                from_values(
                    TimerRuntimeStateSnapshot::Inactive { reason },
                    TimerSchedulingMode::Retry,
                    outcome
                ),
                expected
            );
        }
        let state = TimerRuntimeStateSnapshot::Ordinary(OrdinaryRuntimeStateSnapshot::Running {
            generation: 1,
        });
        assert_eq!(
            from_values(state, TimerSchedulingMode::Retry, failure),
            TimerProcessCondition::Retrying
        );
        assert_eq!(
            from_values(state, TimerSchedulingMode::AfterCompletion, failure),
            TimerProcessCondition::Active
        );
    }
}
