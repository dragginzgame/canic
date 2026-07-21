//! Module: workflow::runtime::intent
//!
//! Responsibility: schedule and run cleanup for expired pending intents.
//! Does not own: intent storage schemas, business policy, or endpoint authorization.
//! Boundary: runtime workflow timer coordinating intent storage cleanup and metrics.

use crate::{
    InternalError, InternalErrorOrigin,
    domain::{policy::pure::intent::decide_receipt_replay_window, runtime::FailureSeverity},
    ids::IntentId,
    log,
    log::Topic,
    model::intent::{
        BeginLocalIntentInput, BeginPlacementReceiptBackedIntentInput,
        BeginReceiptBackedIntentInput, BeginReceiptBackedIntentResult, ReceiptBackedIntent,
        SettleReceiptBackedIntentInput, SettleReceiptBackedIntentResult, TerminalEvidenceDecision,
        is_canic_owned_intent_resource_key,
    },
    ops::{
        ic::IcOps,
        runtime::metrics::intent::{
            IntentMetricOperation, IntentMetricOutcome, IntentMetricReason, IntentMetricSurface,
            IntentMetrics,
        },
        runtime::recent_failure::{RecentFailureInput, RecentFailureOps},
        storage::intent::{IntentStoreOps, ReceiptBackedIntentOps},
    },
    workflow::runtime::timer::{TimerDirective, TimerKey, TimerRunResult, TimerWorkflow},
};

const CLEANUP_BATCH_SIZE: usize = 32;
const NANOS_PER_SECOND: u64 = 1_000_000_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct IntentCleanupBatch {
    application_receipts_removed: usize,
    local_intents_aborted: usize,
}

impl IntentCleanupBatch {
    fn work_count(self) -> Result<u64, InternalError> {
        let total = self
            .application_receipts_removed
            .checked_add(self.local_intents_aborted)
            .ok_or_else(|| {
                InternalError::invariant(
                    InternalErrorOrigin::Workflow,
                    "intent cleanup batch count overflow",
                )
            })?;
        u64::try_from(total).map_err(|_| {
            InternalError::invariant(
                InternalErrorOrigin::Workflow,
                "intent cleanup batch count exceeds u64",
            )
        })
    }
}

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
        IntentCleanupWorkflow::schedule_intent(intent_id)?;
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
        IntentCleanupWorkflow::reconcile_after_terminal();
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
        IntentCleanupWorkflow::reconcile_after_terminal();
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
        let now_ns = IcOps::now_nanos();
        let replay_window = decide_receipt_replay_window(now_ns, input.replay_deadline_ns);
        Self::record_begin_result(ReceiptBackedIntentOps::begin_or_load(
            input,
            now_ns,
            replay_window,
        ))
    }

    pub(crate) fn begin_placement_or_load(
        input: &BeginPlacementReceiptBackedIntentInput,
    ) -> Result<BeginReceiptBackedIntentResult, InternalError> {
        ensure_canic_owned_resource_key(&input.resource_key)?;
        Self::record_begin_result(ReceiptBackedIntentOps::begin_placement_or_load(
            input,
            IcOps::now_nanos(),
        ))
    }

    fn record_begin_result(
        result: Result<BeginReceiptBackedIntentResult, InternalError>,
    ) -> Result<BeginReceiptBackedIntentResult, InternalError> {
        let result = result.inspect_err(|_| {
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
            IntentCleanupWorkflow::reconcile_after_terminal();
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
    /// Reconstruct the sole live cleanup deadline from the stable expiry index.
    pub fn start() -> Result<(), InternalError> {
        Self::reconcile()
    }

    fn run_due_batch() -> TimerRunResult {
        Self::run_due_batch_at(IcOps::now_nanos())
    }

    fn run_due_batch_at(now_ns: u64) -> TimerRunResult {
        let result = Self::cleanup_due_batch(now_ns);
        let batch = match result {
            Ok(batch) => batch,
            Err(err) => {
                record_cleanup_failure(&err);
                record_cleanup_intent(
                    IntentMetricOperation::Cleanup,
                    IntentMetricOutcome::Failed,
                    IntentMetricReason::StorageFailed,
                );
                log!(Topic::Memory, Warn, "intent cleanup batch failed: {err}");
                return TimerRunResult::invariant_failure();
            }
        };

        let directive = match Self::next_directive(now_ns) {
            Ok(directive) => directive,
            Err(err) => {
                record_cleanup_failure(&err);
                log!(
                    Topic::Memory,
                    Warn,
                    "intent cleanup deadline reconciliation failed: {err}"
                );
                return TimerRunResult {
                    outcome: crate::domain::runtime::TimerExecutionOutcome::InvariantFailure,
                    work_count: batch.work_count().unwrap_or(u64::MAX),
                    directive: TimerDirective::Stop,
                };
            }
        };

        let work_count = match batch.work_count() {
            Ok(work_count) => work_count,
            Err(err) => {
                record_cleanup_failure(&err);
                log!(Topic::Memory, Warn, "intent cleanup count failed: {err}");
                return TimerRunResult::invariant_failure();
            }
        };
        if work_count == 0 {
            record_cleanup_intent(
                IntentMetricOperation::Cleanup,
                IntentMetricOutcome::Completed,
                IntentMetricReason::NoExpired,
            );
            TimerRunResult::no_work(directive)
        } else {
            record_cleanup_intent(
                IntentMetricOperation::Cleanup,
                IntentMetricOutcome::Completed,
                IntentMetricReason::Ok,
            );
            log!(
                Topic::Memory,
                Info,
                "intent cleanup: application_receipts_removed={} local_intents_aborted={} batch_limit={CLEANUP_BATCH_SIZE}",
                batch.application_receipts_removed,
                batch.local_intents_aborted,
            );
            TimerRunResult::success(work_count, directive)
        }
    }

    fn cleanup_due_batch(now_ns: u64) -> Result<IntentCleanupBatch, InternalError> {
        let application =
            ReceiptBackedIntentOps::reclaim_due_application_receipts(now_ns, CLEANUP_BATCH_SIZE)?;
        let application_receipts_removed =
            usize::try_from(application.removed_records).map_err(|_| {
                InternalError::invariant(
                    InternalErrorOrigin::Workflow,
                    "application receipt cleanup count exceeds usize",
                )
            })?;
        let local_limit = CLEANUP_BATCH_SIZE
            .checked_sub(application_receipts_removed)
            .ok_or_else(|| {
                InternalError::invariant(
                    InternalErrorOrigin::Workflow,
                    "application receipt cleanup exceeded its batch limit",
                )
            })?;
        let now_secs = now_ns / NANOS_PER_SECOND;
        let due = IntentStoreOps::list_due_expiry_intents(now_secs, local_limit)?;
        let mut aborted = 0;
        for intent_id in due {
            match IntentStoreOps::abort_intent_if_pending(intent_id) {
                Ok(true) => {
                    record_cleanup_intent(
                        IntentMetricOperation::Abort,
                        IntentMetricOutcome::Completed,
                        IntentMetricReason::Expired,
                    );
                    aborted += 1;
                }
                Ok(false) => {
                    return Err(InternalError::invariant(
                        InternalErrorOrigin::Workflow,
                        format!("due intent {intent_id} ceased to be pending before cleanup"),
                    ));
                }
                Err(err) => {
                    record_cleanup_intent(
                        IntentMetricOperation::Abort,
                        IntentMetricOutcome::Failed,
                        IntentMetricReason::StorageFailed,
                    );
                    return Err(
                        err.with_diagnostic_context(format!("abort due local intent {intent_id}"))
                    );
                }
            }
        }
        Ok(IntentCleanupBatch {
            application_receipts_removed,
            local_intents_aborted: aborted,
        })
    }

    pub(crate) fn schedule_intent(intent_id: IntentId) -> Result<(), InternalError> {
        if let Some(due_at_secs) = IntentStoreOps::cleanup_due_at_secs(intent_id)? {
            Self::schedule_at(due_at_secs)?;
        }
        Ok(())
    }

    pub(crate) fn reconcile_after_terminal() {
        if let Err(err) = Self::reconcile() {
            record_cleanup_failure(&err);
            record_cleanup_intent(
                IntentMetricOperation::Cleanup,
                IntentMetricOutcome::Failed,
                IntentMetricReason::StorageFailed,
            );
            log!(
                Topic::Memory,
                Warn,
                "intent cleanup timer reconciliation failed after terminal transition: {err}"
            );
        }
    }

    fn reconcile() -> Result<(), InternalError> {
        let deadline_ns = Self::next_cleanup_deadline_ns()?;
        TimerWorkflow::reconcile_at(TimerKey::IntentCleanup, deadline_ns, || async {
            Self::run_due_batch()
        });
        Ok(())
    }

    fn schedule_at(due_at_secs: u64) -> Result<(), InternalError> {
        let deadline_ns = Self::deadline_ns(due_at_secs)?;
        TimerWorkflow::schedule_at(TimerKey::IntentCleanup, deadline_ns, || async {
            Self::run_due_batch()
        });
        Ok(())
    }

    fn next_directive(now_ns: u64) -> Result<TimerDirective, InternalError> {
        match Self::next_cleanup_deadline_ns()? {
            None => Ok(TimerDirective::Stop),
            Some(deadline_ns) if deadline_ns <= now_ns => Ok(TimerDirective::ContinueImmediately),
            Some(deadline_ns) => Ok(TimerDirective::ScheduleAt(deadline_ns)),
        }
    }

    fn next_cleanup_deadline_ns() -> Result<Option<u64>, InternalError> {
        let local = IntentStoreOps::next_expiry_at_secs()?
            .map(Self::deadline_ns)
            .transpose()?;
        let application = ReceiptBackedIntentOps::receipt_capacity()?.next_eligibility_at_ns;
        Ok(match (local, application) {
            (Some(local), Some(application)) => Some(local.min(application)),
            (Some(deadline), None) | (None, Some(deadline)) => Some(deadline),
            (None, None) => None,
        })
    }

    fn deadline_ns(due_at_secs: u64) -> Result<u64, InternalError> {
        due_at_secs.checked_mul(NANOS_PER_SECOND).ok_or_else(|| {
            InternalError::invariant(
                InternalErrorOrigin::Workflow,
                format!("intent cleanup deadline overflows nanoseconds: {due_at_secs}"),
            )
        })
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

fn record_cleanup_failure(err: &InternalError) {
    let (class, origin) = err.log_fields();
    RecentFailureOps::record(RecentFailureInput {
        occurred_at_ns: IcOps::now_nanos(),
        subsystem: "intent_cleanup".to_string(),
        code: "intent_cleanup_invariant".to_string(),
        severity: FailureSeverity::Error,
        summary: format!("class={class} origin={origin}: {err}"),
        correlation_id: None,
    });
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
        test::seams,
    };

    fn canic_receipt_input() -> BeginReceiptBackedIntentInput {
        BeginReceiptBackedIntentInput {
            operation_id: OperationId::from_bytes([1; 32]),
            payload_binding: PayloadBinding::new([2; 32]),
            resource_key: IntentResourceKey::new(format!("canic:placement:{}", "a".repeat(64))),
            quantity: 1,
            reservation_limit: 1,
            replay_deadline_ns: u64::MAX,
        }
    }

    fn application_receipt_input(seed: u8) -> BeginReceiptBackedIntentInput {
        BeginReceiptBackedIntentInput {
            operation_id: OperationId::from_bytes([seed; 32]),
            payload_binding: PayloadBinding::new([seed.wrapping_add(1); 32]),
            resource_key: IntentResourceKey::new(format!("application:{seed}")),
            quantity: 1,
            reservation_limit: 1,
            replay_deadline_ns: 1_000,
        }
    }

    fn placement_receipt_input() -> BeginPlacementReceiptBackedIntentInput {
        let input = canic_receipt_input();
        BeginPlacementReceiptBackedIntentInput {
            operation_id: input.operation_id,
            payload_binding: input.payload_binding,
            resource_key: input.resource_key,
            quantity: input.quantity,
            reservation_limit: input.reservation_limit,
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
        IntentStoreOps::reset_for_tests();

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
        assert!(
            ReceiptBackedIntentOps::load(canic_receipt_input().operation_id)
                .expect("load receipt-backed intent")
                .is_none()
        );
    }

    #[test]
    fn consumer_cannot_commit_or_rollback_canic_owned_local_intent() {
        let _guard = seams::lock();
        IntentStoreOps::reset_for_tests();
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
        assert!(
            IntentStoreOps::is_pending_for_tests(intent_id)
                .expect("internal intent state remains readable")
        );
    }

    #[test]
    fn consumer_receipt_operations_cannot_observe_or_settle_canic_owned_intent() {
        let _guard = seams::lock();
        IntentStoreOps::reset_for_tests();
        let input = placement_receipt_input();

        assert!(matches!(
            ReceiptBackedIntentWorkflow::begin_placement_or_load(&input)
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
        IntentStoreOps::reset_for_tests();
        let mut input = canic_receipt_input();
        input.resource_key = IntentResourceKey::new("app:placement");
        input.replay_deadline_ns = IcOps::now_nanos().saturating_add(NANOS_PER_SECOND);

        assert!(matches!(
            ReceiptBackedIntentWorkflow::begin_or_load(&input)
                .expect("consumer namespace remains available"),
            BeginReceiptBackedIntentResult::Created { revision: 1 }
        ));
    }

    #[test]
    fn cleanup_due_work_is_bounded_and_continues_until_the_expiry_index_is_empty() {
        let _guard = seams::lock();
        IntentStoreOps::reset_for_tests();
        let resource_key = IntentResourceKey::new("cleanup:bounded");
        for seed in 1..=33 {
            IntentStoreOps::try_reserve(IntentId(seed), resource_key.clone(), 1, 10, Some(0), 10)
                .expect("reserve due intent");
        }
        IntentStoreOps::try_reserve(IntentId(34), resource_key, 1, 10, None, 10)
            .expect("reserve TTL-free intent");

        assert_eq!(
            IntentCleanupWorkflow::cleanup_due_batch(11 * NANOS_PER_SECOND)
                .expect("first bounded batch"),
            IntentCleanupBatch {
                application_receipts_removed: 0,
                local_intents_aborted: CLEANUP_BATCH_SIZE,
            }
        );
        assert!(matches!(
            IntentCleanupWorkflow::next_directive(11 * NANOS_PER_SECOND)
                .expect("continuation directive"),
            TimerDirective::ContinueImmediately
        ));
        assert_eq!(
            IntentCleanupWorkflow::cleanup_due_batch(11 * NANOS_PER_SECOND)
                .expect("second bounded batch"),
            IntentCleanupBatch {
                application_receipts_removed: 0,
                local_intents_aborted: 1,
            }
        );
        assert!(matches!(
            IntentCleanupWorkflow::next_directive(11 * NANOS_PER_SECOND)
                .expect("terminal directive"),
            TimerDirective::Stop
        ));
        assert_eq!(IntentStoreOps::pending_total().expect("pending total"), 1);
        assert_eq!(IntentStoreOps::expiry_index_total_for_tests(), 0);
    }

    #[test]
    fn application_and_local_cleanup_share_one_exact_batch_budget() {
        let _guard = seams::lock();
        IntentStoreOps::reset_for_tests();
        for seed in 1..=33 {
            let input = application_receipt_input(seed);
            ReceiptBackedIntentOps::begin_or_load(
                &input,
                100,
                crate::model::intent::ReceiptReplayWindowDecision::Open,
            )
            .expect("create application receipt");
            ReceiptBackedIntentOps::settle_if_pending(
                &SettleReceiptBackedIntentInput {
                    operation_id: input.operation_id,
                    expected_revision: 1,
                    expected_payload_binding: input.payload_binding,
                    evidence: TerminalEvidence::new(
                        Principal::from_slice(&[3; 29]),
                        TerminalEvidenceDecision::Committed,
                        [seed; 32],
                    ),
                },
                200,
            )
            .expect("settle application receipt");
        }
        IntentStoreOps::try_reserve(
            IntentId(50),
            IntentResourceKey::new("cleanup:shared"),
            1,
            10,
            Some(0),
            10,
        )
        .expect("reserve due local intent");
        let due_at_ns = 200 + crate::model::intent::RECEIPT_TERMINAL_OBSERVATION_GRACE_NS;

        assert_eq!(
            IntentCleanupWorkflow::cleanup_due_batch(due_at_ns).expect("first shared batch"),
            IntentCleanupBatch {
                application_receipts_removed: CLEANUP_BATCH_SIZE,
                local_intents_aborted: 0,
            }
        );
        assert!(matches!(
            IntentCleanupWorkflow::next_directive(due_at_ns).expect("continue shared cleanup"),
            TimerDirective::ContinueImmediately
        ));
        assert!(IntentStoreOps::is_pending_for_tests(IntentId(50)).expect("local intent remains"));

        assert_eq!(
            IntentCleanupWorkflow::cleanup_due_batch(due_at_ns).expect("second shared batch"),
            IntentCleanupBatch {
                application_receipts_removed: 1,
                local_intents_aborted: 1,
            }
        );
        assert!(matches!(
            IntentCleanupWorkflow::next_directive(due_at_ns).expect("finish shared cleanup"),
            TimerDirective::Stop
        ));
        assert_eq!(
            ReceiptBackedIntentOps::receipt_capacity()
                .expect("empty application capacity")
                .application_records,
            0
        );
        assert!(!IntentStoreOps::is_pending_for_tests(IntentId(50)).expect("local intent removed"));
    }

    #[test]
    fn cleanup_failure_preserves_the_due_intent_and_exact_expiry_entry() {
        let _guard = seams::lock();
        IntentStoreOps::reset_for_tests();
        RecentFailureOps::reset();
        let intent_id = IntentId(41);
        IntentStoreOps::try_reserve(
            intent_id,
            IntentResourceKey::new("cleanup:failure"),
            1,
            10,
            Some(0),
            10,
        )
        .expect("reserve due intent");
        IntentStoreOps::clear_totals_for_tests();

        IntentCleanupWorkflow::cleanup_due_batch(11 * NANOS_PER_SECOND)
            .expect_err("missing totals must preserve typed storage failure");
        assert!(IntentStoreOps::is_pending_for_tests(intent_id).expect("pending intent remains"));
        assert_eq!(IntentStoreOps::expiry_index_total_for_tests(), 1);
        assert_eq!(
            IntentStoreOps::list_due_expiry_intents(11, CLEANUP_BATCH_SIZE)
                .expect("due intent remains indexed"),
            vec![intent_id]
        );

        let result = IntentCleanupWorkflow::run_due_batch_at(11 * NANOS_PER_SECOND);
        assert_eq!(
            result.outcome,
            crate::domain::runtime::TimerExecutionOutcome::InvariantFailure
        );
        assert_eq!(result.directive, TimerDirective::Stop);
        let failure = RecentFailureOps::snapshot()
            .into_iter()
            .next()
            .expect("protected cleanup diagnostic");
        assert_eq!(failure.subsystem, "intent_cleanup");
        assert_eq!(failure.code, "intent_cleanup_invariant");
        assert_eq!(failure.severity, FailureSeverity::Error);
        RecentFailureOps::reset();
    }
}
