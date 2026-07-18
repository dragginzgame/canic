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
        TerminalEvidenceDecision, is_canic_owned_intent_resource_key,
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
        ensure_consumer_resource_key(&input.resource_key)?;
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
        ensure_consumer_local_intent(intent_id)?;
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
        ensure_consumer_local_intent(intent_id)?;
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
        ensure_consumer_resource_key(&input.resource_key)?;
        Self::begin_or_load_authorized(input)
    }

    pub(crate) fn begin_canic_owned_or_load(
        input: &BeginReceiptBackedIntentInput,
    ) -> Result<BeginReceiptBackedIntentResult, InternalError> {
        ensure_canic_owned_resource_key(&input.resource_key)?;
        Self::begin_or_load_authorized(input)
    }

    fn begin_or_load_authorized(
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
        let intent = ReceiptBackedIntentOps::load(operation_id)?;
        if let Some(intent) = &intent {
            ensure_consumer_resource_key(&intent.resource_key)?;
        }
        Ok(intent)
    }

    pub fn settle_if_pending(
        input: &SettleReceiptBackedIntentInput,
    ) -> Result<SettleReceiptBackedIntentResult, InternalError> {
        if let Some(intent) = ReceiptBackedIntentOps::load(input.operation_id)? {
            ensure_consumer_resource_key(&intent.resource_key)?;
        }
        Self::settle_if_pending_authorized(input)
    }

    pub(crate) fn settle_canic_owned_if_pending(
        input: &SettleReceiptBackedIntentInput,
    ) -> Result<SettleReceiptBackedIntentResult, InternalError> {
        if let Some(intent) = ReceiptBackedIntentOps::load(input.operation_id)? {
            ensure_canic_owned_resource_key(&intent.resource_key)?;
        }
        Self::settle_if_pending_authorized(input)
    }

    fn settle_if_pending_authorized(
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

fn ensure_consumer_resource_key(
    resource_key: &crate::ids::IntentResourceKey,
) -> Result<(), InternalError> {
    if is_canic_owned_intent_resource_key(resource_key) {
        return Err(InternalError::invalid_input(
            "intent resource keys beginning with 'canic:' are reserved for Canic runtime authority",
        ));
    }
    Ok(())
}

fn ensure_consumer_local_intent(intent_id: IntentId) -> Result<(), InternalError> {
    if let Some(intent) = IntentStoreOps::load(intent_id)? {
        ensure_consumer_resource_key(&intent.resource_key)?;
    }
    Ok(())
}

fn ensure_canic_owned_resource_key(
    resource_key: &crate::ids::IntentResourceKey,
) -> Result<(), InternalError> {
    if !is_canic_owned_intent_resource_key(resource_key) {
        return Err(InternalError::invariant(
            InternalErrorOrigin::Workflow,
            "Canic-owned intent must use the reserved 'canic:' resource namespace",
        ));
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cdk::types::Principal,
        dto::error::ErrorCode,
        ids::IntentResourceKey,
        model::{
            intent::{PayloadBinding, TerminalEvidence},
            replay::OperationId,
        },
        storage::stable::intent::{IntentStore, ReceiptBackedIntentStore},
        test::seams,
    };

    fn canic_receipt_input() -> BeginReceiptBackedIntentInput {
        BeginReceiptBackedIntentInput {
            operation_id: OperationId::from_bytes([1; 32]),
            payload_binding: PayloadBinding::new([2; 32]),
            resource_key: IntentResourceKey::new(format!("canic:placement:{}", "a".repeat(64))),
            quantity: 1,
            reservation_limit: 1,
        }
    }

    fn assert_reserved_namespace_error(err: &InternalError) {
        assert_eq!(
            err.public_error().map(|error| error.code),
            Some(ErrorCode::InvalidInput)
        );
    }

    #[test]
    fn consumer_begin_rejects_canic_owned_resource_namespace() {
        let _guard = seams::lock();
        IntentStore::reset_for_tests();

        let local_error = LocalIntentWorkflow::begin(BeginLocalIntentInput {
            resource_key: IntentResourceKey::new("canic:consumer"),
            quantity: 1,
            ttl_secs: None,
            reservation_limit: Some(1),
        })
        .expect_err("consumer local intent must not enter Canic namespace");
        assert_reserved_namespace_error(&local_error);

        let receipt_error = ReceiptBackedIntentWorkflow::begin_or_load(&canic_receipt_input())
            .expect_err("consumer receipt intent must not enter Canic namespace");
        assert_reserved_namespace_error(&receipt_error);
        assert_eq!(ReceiptBackedIntentStore::len(), 0);
    }

    #[test]
    fn consumer_cannot_commit_or_rollback_canic_owned_local_intent() {
        let _guard = seams::lock();
        IntentStore::reset_for_tests();
        let intent_id = IntentStoreOps::allocate_intent_id().expect("allocate internal intent");
        IntentStoreOps::try_reserve(
            intent_id,
            IntentResourceKey::new("canic:test"),
            1,
            10,
            None,
            10,
        )
        .expect("reserve internal intent");

        let commit_error = LocalIntentWorkflow::commit(intent_id)
            .expect_err("consumer commit must reject Canic-owned intent");
        assert_reserved_namespace_error(&commit_error);
        let rollback_error = LocalIntentWorkflow::rollback(intent_id)
            .expect_err("consumer rollback must reject Canic-owned intent");
        assert_reserved_namespace_error(&rollback_error);
        assert_eq!(
            IntentStoreOps::load(intent_id)
                .expect("load internal intent")
                .expect("internal intent remains pending")
                .state,
            crate::storage::stable::intent::IntentState::Pending
        );
    }

    #[test]
    fn consumer_receipt_operations_cannot_observe_or_settle_canic_owned_intent() {
        let _guard = seams::lock();
        IntentStore::reset_for_tests();
        let input = canic_receipt_input();

        assert!(matches!(
            ReceiptBackedIntentWorkflow::begin_canic_owned_or_load(&input)
                .expect("Canic-owned begin succeeds"),
            BeginReceiptBackedIntentResult::Created { revision: 1 }
        ));

        let load_error = ReceiptBackedIntentWorkflow::load(input.operation_id)
            .expect_err("consumer load must reject Canic-owned intent");
        assert_reserved_namespace_error(&load_error);

        let settle = SettleReceiptBackedIntentInput {
            operation_id: input.operation_id,
            expected_revision: 1,
            expected_payload_binding: input.payload_binding,
            evidence: TerminalEvidence::new(
                Principal::from_slice(&[3; 29]),
                TerminalEvidenceDecision::Committed,
                [4; 32],
            ),
        };
        let settle_error = ReceiptBackedIntentWorkflow::settle_if_pending(&settle)
            .expect_err("consumer settlement must reject Canic-owned intent");
        assert_reserved_namespace_error(&settle_error);
        assert!(matches!(
            ReceiptBackedIntentWorkflow::settle_canic_owned_if_pending(&settle)
                .expect("Canic-owned settlement succeeds"),
            SettleReceiptBackedIntentResult::Settled { .. }
        ));
    }

    #[test]
    fn consumer_receipt_namespace_remains_available_outside_canic_prefix() {
        let _guard = seams::lock();
        IntentStore::reset_for_tests();
        let mut input = canic_receipt_input();
        input.resource_key = IntentResourceKey::new("app:placement");

        assert!(matches!(
            ReceiptBackedIntentWorkflow::begin_or_load(&input)
                .expect("consumer namespace remains available"),
            BeginReceiptBackedIntentResult::Created { revision: 1 }
        ));
    }
}
