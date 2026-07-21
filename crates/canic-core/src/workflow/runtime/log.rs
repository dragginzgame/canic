//! Module: workflow::runtime::log
//!
//! Responsibility: append bounded runtime logs and schedule exact age retention.
//! Does not own: log configuration, stable schemas, or public log queries.
//! Boundary: every runtime-log mutation reaches storage through this workflow.

use crate::{
    InternalError, InternalErrorOrigin,
    config::schema::LogConfig,
    domain::policy::pure as policy,
    log::{Level, Topic},
    ops::{config::ConfigOps, ic::IcOps, runtime::log::LogOps},
    workflow::runtime::timer::{TimerDirective, TimerKey, TimerRunResult, TimerWorkflow},
};

const RETENTION_BATCH_SIZE: usize = 256;
const NANOS_PER_SECOND: u64 = 1_000_000_000;

/// Canonical append and age-retention owner for runtime logs.
pub struct LogRetentionWorkflow;

impl LogRetentionWorkflow {
    /// Reconstruct the sole live age deadline from retained log state.
    pub fn start() -> Result<(), InternalError> {
        let config = ConfigOps::log_config()?;
        Self::reconcile(&config)
    }

    /// Append one count-bounded entry and reconcile the exact oldest age deadline.
    pub(crate) fn append_runtime_log(
        crate_name: &str,
        topic: Option<Topic>,
        level: Level,
        message: &str,
        created_at: u64,
    ) -> Result<(), InternalError> {
        let config = ConfigOps::log_config()?;
        LogOps::append_runtime_log(
            crate_name,
            topic,
            level,
            message,
            created_at,
            config.max_entries,
            config.max_entry_bytes,
        )?;
        Self::reconcile(&config)
    }

    fn reconcile(config: &LogConfig) -> Result<(), InternalError> {
        let deadline = match config.max_age_secs {
            Some(max_age_secs) => Self::next_deadline_ns(max_age_secs)?,
            None => None,
        };
        TimerWorkflow::reconcile_at(TimerKey::LogRetention, deadline, || async {
            Self::run_due_batch()
        });
        Ok(())
    }

    fn run_due_batch() -> TimerRunResult {
        let config = match ConfigOps::log_config() {
            Ok(config) => config,
            Err(err) => {
                IcOps::println(&format!("log retention stopped: {err}"));
                return TimerRunResult::invariant_failure();
            }
        };
        let Some(max_age_secs) = config.max_age_secs else {
            return TimerRunResult::no_work(TimerDirective::Stop);
        };

        let now_secs = IcOps::now_secs();
        let cutoff = policy::log::age_cutoff(now_secs, max_age_secs);
        let batch = LogOps::retain_created_before(cutoff, RETENTION_BATCH_SIZE);
        if batch.dropped > 0 {
            IcOps::println(&format!(
                "log retention: dropped={}, before={}, retained={}",
                batch.dropped, batch.before, batch.retained
            ));
        }

        let directive = if batch.more_due {
            TimerDirective::ContinueImmediately
        } else {
            match Self::next_directive(max_age_secs, now_secs) {
                Ok(directive) => directive,
                Err(err) => {
                    IcOps::println(&format!("log retention stopped: {err}"));
                    return TimerRunResult::invariant_failure();
                }
            }
        };
        if batch.dropped == 0 {
            TimerRunResult::no_work(directive)
        } else {
            TimerRunResult::success(batch.dropped, directive)
        }
    }

    fn next_directive(max_age_secs: u64, now_secs: u64) -> Result<TimerDirective, InternalError> {
        let Some(deadline_ns) = Self::next_deadline_ns(max_age_secs)? else {
            return Ok(TimerDirective::Stop);
        };
        let now_ns = seconds_to_nanos(now_secs)?;
        if deadline_ns <= now_ns {
            Ok(TimerDirective::ContinueImmediately)
        } else {
            Ok(TimerDirective::ScheduleAt(deadline_ns))
        }
    }

    fn next_deadline_ns(max_age_secs: u64) -> Result<Option<u64>, InternalError> {
        let Some(created_at) = LogOps::oldest_created_at() else {
            return Ok(None);
        };
        let deadline_secs =
            policy::log::age_expiry_at(created_at, max_age_secs).ok_or_else(|| {
                InternalError::invariant(
                    InternalErrorOrigin::Workflow,
                    "runtime-log age deadline overflowed seconds",
                )
            })?;
        seconds_to_nanos(deadline_secs).map(Some)
    }
}

fn seconds_to_nanos(seconds: u64) -> Result<u64, InternalError> {
    seconds.checked_mul(NANOS_PER_SECOND).ok_or_else(|| {
        InternalError::invariant(
            InternalErrorOrigin::Workflow,
            "runtime-log age deadline overflowed nanoseconds",
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::runtime::log::LogOps;

    fn append(created_at: u64) {
        LogOps::append_test_entry(created_at);
    }

    #[test]
    fn age_deadline_is_exact_and_empty_state_stops() {
        LogOps::reset_for_tests();
        assert_eq!(
            LogRetentionWorkflow::next_directive(10, 100).expect("empty state must be valid"),
            TimerDirective::Stop
        );

        append(100);
        assert_eq!(
            LogRetentionWorkflow::next_directive(10, 110).expect("deadline must fit"),
            TimerDirective::ScheduleAt(111 * NANOS_PER_SECOND)
        );
        assert_eq!(
            LogRetentionWorkflow::next_directive(10, 111).expect("deadline must fit"),
            TimerDirective::ContinueImmediately
        );
    }

    #[test]
    fn age_deadline_overflow_fails_closed() {
        LogOps::reset_for_tests();
        append(u64::MAX / NANOS_PER_SECOND);

        let err = LogRetentionWorkflow::next_deadline_ns(0)
            .expect_err("unrepresentable nanosecond deadline must reject");

        assert_eq!(err.class(), crate::InternalErrorClass::Invariant);
        assert_eq!(err.origin(), InternalErrorOrigin::Workflow);
    }

    #[test]
    fn due_batch_is_bounded_and_continues_only_for_due_work() {
        LogOps::reset_for_tests();
        for created_at in 0..=RETENTION_BATCH_SIZE as u64 {
            append(created_at);
        }

        let cutoff = u64::try_from(RETENTION_BATCH_SIZE).expect("batch size") + 1;
        let first = LogOps::retain_created_before(cutoff, RETENTION_BATCH_SIZE);
        assert_eq!(first.dropped, RETENTION_BATCH_SIZE as u64);
        assert!(first.more_due);

        let second = LogOps::retain_created_before(cutoff, RETENTION_BATCH_SIZE);
        assert_eq!(second.dropped, 1);
        assert!(!second.more_due);
    }
}
