//! Module: workflow::runtime::intent
//!
//! Responsibility: schedule and run cleanup for expired pending intents.
//! Does not own: intent storage schemas, business policy, or endpoint authorization.
//! Boundary: runtime workflow timer coordinating intent storage cleanup and metrics.

use crate::{
    InternalError, InternalErrorOrigin,
    ids::IntentId,
    log,
    log::Topic,
    model::intent::{
        BeginLocalIntentInput, BeginReceiptBackedIntentInput, BeginReceiptBackedIntentResult,
        ReceiptBackedIntent, SettleReceiptBackedIntentInput, SettleReceiptBackedIntentResult,
        TerminalEvidenceDecision,
    },
    ops::{
        ic::IcOps,
        runtime::{
            metrics::intent::{
                IntentMetricOperation, IntentMetricOutcome, IntentMetricReason,
                IntentMetricSurface, IntentMetrics,
            },
            timer::TimerId,
        },
        storage::intent::{IntentStoreOps, ReceiptBackedIntentOps},
    },
    workflow::{
        config::{WORKFLOW_INIT_DELAY, WORKFLOW_INTENT_CLEANUP_INTERVAL},
        runtime::timer::TimerWorkflow,
    },
};
use std::{cell::RefCell, time::Duration};

thread_local! {
    static INTENT_CLEANUP_TIMER: RefCell<Option<TimerId>> = const { RefCell::new(None) };
}

const CLEANUP_INTERVAL: Duration = WORKFLOW_INTENT_CLEANUP_INTERVAL;

/// Direct workflow for locally decidable, expirable reservations.
pub struct LocalIntentWorkflow;

impl LocalIntentWorkflow {
    pub fn begin(input: BeginLocalIntentInput) -> Result<IntentId, InternalError> {
        let now = IcOps::now_secs();
        if let Some(limit) = input.reservation_limit {
            let current = IntentStoreOps::totals(&input.resource_key).reserved_qty;
            let next = current.checked_add(input.quantity).ok_or_else(|| {
                record_intent(
                    IntentMetricSurface::Local,
                    IntentMetricOperation::CapacityCheck,
                    IntentMetricOutcome::Failed,
                    IntentMetricReason::Overflow,
                );
                InternalError::invariant(
                    InternalErrorOrigin::Workflow,
                    "local intent reservation overflow",
                )
            })?;
            if next > limit {
                record_intent(
                    IntentMetricSurface::Local,
                    IntentMetricOperation::CapacityCheck,
                    IntentMetricOutcome::Failed,
                    IntentMetricReason::Capacity,
                );
                return Err(InternalError::domain(
                    InternalErrorOrigin::Domain,
                    format!(
                        "local intent capacity exceeded key={} in_flight={current} requested={} limit={limit}",
                        input.resource_key, input.quantity
                    ),
                ));
            }
            record_intent(
                IntentMetricSurface::Local,
                IntentMetricOperation::CapacityCheck,
                IntentMetricOutcome::Completed,
                IntentMetricReason::Ok,
            );
        }

        let intent_id = IntentStoreOps::allocate_intent_id().inspect_err(|_| {
            record_intent(
                IntentMetricSurface::Local,
                IntentMetricOperation::Reserve,
                IntentMetricOutcome::Failed,
                IntentMetricReason::StorageFailed,
            );
        })?;
        IntentStoreOps::try_reserve(
            intent_id,
            input.resource_key,
            input.quantity,
            now,
            input.ttl_secs,
            now,
        )
        .inspect_err(|_| {
            record_intent(
                IntentMetricSurface::Local,
                IntentMetricOperation::Reserve,
                IntentMetricOutcome::Failed,
                IntentMetricReason::StorageFailed,
            );
        })?;
        record_intent(
            IntentMetricSurface::Local,
            IntentMetricOperation::Reserve,
            IntentMetricOutcome::Completed,
            IntentMetricReason::Ok,
        );
        IntentCleanupWorkflow::ensure_started();
        Ok(intent_id)
    }

    pub fn commit(intent_id: IntentId) -> Result<(), InternalError> {
        IntentStoreOps::commit_at(intent_id, IcOps::now_secs()).inspect_err(|_| {
            record_intent(
                IntentMetricSurface::Local,
                IntentMetricOperation::Commit,
                IntentMetricOutcome::Failed,
                IntentMetricReason::StorageFailed,
            );
        })?;
        record_intent(
            IntentMetricSurface::Local,
            IntentMetricOperation::Commit,
            IntentMetricOutcome::Completed,
            IntentMetricReason::Ok,
        );
        Ok(())
    }

    pub fn rollback(intent_id: IntentId) -> Result<(), InternalError> {
        IntentStoreOps::abort(intent_id).inspect_err(|_| {
            record_intent(
                IntentMetricSurface::Local,
                IntentMetricOperation::Abort,
                IntentMetricOutcome::Failed,
                IntentMetricReason::StorageFailed,
            );
        })?;
        record_intent(
            IntentMetricSurface::Local,
            IntentMetricOperation::Abort,
            IntentMetricOutcome::Completed,
            IntentMetricReason::Ok,
        );
        Ok(())
    }
}

/// Non-awaiting workflow facade for exact-key receipt-backed operations.
pub struct ReceiptBackedIntentWorkflow;

impl ReceiptBackedIntentWorkflow {
    pub fn begin_or_load(
        input: &BeginReceiptBackedIntentInput,
    ) -> Result<BeginReceiptBackedIntentResult, InternalError> {
        let result =
            ReceiptBackedIntentOps::begin_or_load(input, IcOps::now_nanos()).inspect_err(|_| {
                record_intent(
                    IntentMetricSurface::ReceiptBacked,
                    IntentMetricOperation::Reserve,
                    IntentMetricOutcome::Failed,
                    IntentMetricReason::StorageFailed,
                );
            })?;
        if matches!(result, BeginReceiptBackedIntentResult::Created { .. }) {
            record_intent(
                IntentMetricSurface::ReceiptBacked,
                IntentMetricOperation::Reserve,
                IntentMetricOutcome::Completed,
                IntentMetricReason::Ok,
            );
        }
        Ok(result)
    }

    pub fn load(
        operation_id: crate::model::replay::OperationId,
    ) -> Result<Option<ReceiptBackedIntent>, InternalError> {
        ReceiptBackedIntentOps::load(operation_id)
    }

    pub fn settle_if_pending(
        input: &SettleReceiptBackedIntentInput,
    ) -> Result<SettleReceiptBackedIntentResult, InternalError> {
        let operation = match input.evidence.decision {
            TerminalEvidenceDecision::Committed => IntentMetricOperation::Commit,
            TerminalEvidenceDecision::RolledBack => IntentMetricOperation::Abort,
        };
        let result = ReceiptBackedIntentOps::settle_if_pending(input, IcOps::now_nanos())
            .inspect_err(|_| {
                record_intent(
                    IntentMetricSurface::ReceiptBacked,
                    operation,
                    IntentMetricOutcome::Failed,
                    IntentMetricReason::StorageFailed,
                );
            })?;
        if matches!(result, SettleReceiptBackedIntentResult::Settled { .. }) {
            record_intent(
                IntentMetricSurface::ReceiptBacked,
                operation,
                IntentMetricOutcome::Completed,
                IntentMetricReason::Ok,
            );
        }
        Ok(result)
    }
}

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
        match IntentStoreOps::expirable_pending_total() {
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

fn record_intent(
    surface: IntentMetricSurface,
    operation: IntentMetricOperation,
    outcome: IntentMetricOutcome,
    reason: IntentMetricReason,
) {
    IntentMetrics::record(surface, operation, outcome, reason);
}
