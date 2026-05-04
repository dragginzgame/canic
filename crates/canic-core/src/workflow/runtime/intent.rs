use crate::{
    ops::{
        ic::IcOps,
        runtime::metrics::intent::{
            IntentMetricOperation, IntentMetricOutcome, IntentMetricReason, IntentMetricSurface,
            IntentMetrics,
        },
        storage::intent::IntentStoreOps,
    },
    workflow::{
        config::{WORKFLOW_INIT_DELAY, WORKFLOW_INTENT_CLEANUP_INTERVAL},
        prelude::*,
        runtime::timer::{TimerId, TimerWorkflow},
    },
};
use std::{cell::RefCell, time::Duration};

thread_local! {
    static INTENT_CLEANUP_TIMER: RefCell<Option<TimerId>> = const { RefCell::new(None) };
}

const CLEANUP_INTERVAL: Duration = WORKFLOW_INTENT_CLEANUP_INTERVAL;

///
/// IntentCleanupWorkflow
///

pub struct IntentCleanupWorkflow;

impl IntentCleanupWorkflow {
    /// Start periodic intent cleanup sweeps.
    pub fn ensure_started() {
        let _ = TimerWorkflow::set_guarded_interval(
            &INTENT_CLEANUP_TIMER,
            WORKFLOW_INIT_DELAY,
            "intent_cleanup:init",
            || async {
                let _ = Self::cleanup();
            },
            CLEANUP_INTERVAL,
            "intent_cleanup:interval",
            || async {
                let _ = Self::cleanup();
            },
        );
    }

    /// Run a cleanup sweep immediately.
    #[must_use]
    pub fn cleanup() -> bool {
        if Self::stop_when_idle() {
            record_cleanup_intent(
                IntentMetricOperation::Cleanup,
                IntentMetricOutcome::Completed,
                IntentMetricReason::Idle,
            );
            return true;
        }

        let now = IcOps::now_secs();
        let expired = IntentStoreOps::list_expired_pending_intents(now);

        if expired.is_empty() {
            record_cleanup_intent(
                IntentMetricOperation::Cleanup,
                IntentMetricOutcome::Completed,
                IntentMetricReason::NoExpired,
            );
            return true;
        }

        let expired_total = expired.len();
        let mut aborted = 0usize;
        let mut errors = 0usize;

        for intent_id in expired {
            match IntentStoreOps::abort_intent_if_pending(intent_id) {
                Ok(true) => {
                    record_cleanup_intent(
                        IntentMetricOperation::Abort,
                        IntentMetricOutcome::Completed,
                        IntentMetricReason::Expired,
                    );
                    aborted += 1;
                }
                Ok(false) => {}
                Err(err) => {
                    record_cleanup_intent(
                        IntentMetricOperation::Abort,
                        IntentMetricOutcome::Failed,
                        IntentMetricReason::StorageFailed,
                    );
                    errors += 1;
                    log!(
                        Topic::Memory,
                        Warn,
                        "intent cleanup abort failed id={intent_id}: {err}"
                    );
                }
            }
        }

        log!(
            Topic::Memory,
            Info,
            "intent cleanup: expired={expired_total} aborted={aborted} errors={errors}"
        );

        if errors == 0 {
            record_cleanup_intent(
                IntentMetricOperation::Cleanup,
                IntentMetricOutcome::Completed,
                IntentMetricReason::Ok,
            );
            Self::stop_when_idle();
        } else {
            record_cleanup_intent(
                IntentMetricOperation::Cleanup,
                IntentMetricOutcome::Failed,
                IntentMetricReason::StorageFailed,
            );
        }

        errors == 0
    }

    // Stop the cleanup timer once there are no pending intents left.
    fn stop_when_idle() -> bool {
        match IntentStoreOps::pending_total() {
            Ok(0) => {
                let _ = TimerWorkflow::clear_guarded(&INTENT_CLEANUP_TIMER);
                true
            }
            Ok(_) => false,
            Err(err) => {
                record_cleanup_intent(
                    IntentMetricOperation::Cleanup,
                    IntentMetricOutcome::Failed,
                    IntentMetricReason::StorageFailed,
                );
                log!(
                    Topic::Memory,
                    Warn,
                    "intent cleanup pending check failed: {err}"
                );
                false
            }
        }
    }
}

// Record a cleanup-surface intent metric with fixed labels only.
fn record_cleanup_intent(
    operation: IntentMetricOperation,
    outcome: IntentMetricOutcome,
    reason: IntentMetricReason,
) {
    IntentMetrics::record(IntentMetricSurface::Cleanup, operation, outcome, reason);
}
