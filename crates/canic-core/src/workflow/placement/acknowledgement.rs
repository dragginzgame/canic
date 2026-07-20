//! Module: workflow::placement::acknowledgement
//!
//! Responsibility: drain durable terminal placement receipts through root acknowledgement.
//! Does not own: placement settlement, root replay receipts, or stable record schemas.
//! Boundary: consumes the placement-only index through ops and schedules only known work.

use crate::{
    InternalError, InternalErrorClass, log,
    log::Topic,
    model::replay::OperationId,
    ops::{
        rpc::request::RequestOps, runtime::env::EnvOps, storage::intent::ReceiptBackedIntentOps,
    },
    workflow::{
        placement::allocation::remove_exact_terminal_intent,
        runtime::timer::{TimerDirective, TimerKey, TimerRunResult, TimerWorkflow},
    },
};
use std::{cell::Cell, time::Duration};

const ACKNOWLEDGEMENT_BATCH_SIZE: usize = 32;
const RETRY_INITIAL: Duration = Duration::from_mins(1);
const RETRY_MAX: Duration = Duration::from_mins(30);

thread_local! {
    static CURSOR: Cell<Option<OperationId>> = const { Cell::new(None) };
    static RETRY_STREAK: Cell<u8> = const { Cell::new(0) };
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DrainDirective {
    Continue,
    Retry,
    Stop,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DrainResult {
    work_count: u64,
    directive: DrainDirective,
}

///
/// PlacementAcknowledgementWorkflow
///
/// Event-driven owner of root placement-receipt release.
///

pub struct PlacementAcknowledgementWorkflow;

impl PlacementAcknowledgementWorkflow {
    /// Reconstruct scheduling from the lifecycle-rebuilt durable index.
    pub fn start() -> Result<(), InternalError> {
        Self::schedule_if_pending()
    }

    /// Advance the worker immediately when exact terminal evidence is added.
    pub fn schedule_if_pending() -> Result<(), InternalError> {
        if ReceiptBackedIntentOps::has_placement_acknowledgements()? {
            Self::schedule(Duration::ZERO);
        } else {
            TimerWorkflow::reconcile_at(
                TimerKey::PlacementReceiptAcknowledgement,
                None,
                Self::run_scheduled,
            );
        }
        Ok(())
    }

    fn schedule(delay: Duration) {
        TimerWorkflow::schedule(
            TimerKey::PlacementReceiptAcknowledgement,
            delay,
            Self::run_scheduled,
        );
    }

    async fn run_scheduled() -> TimerRunResult {
        let result = match Self::drain_batch().await {
            Ok(result) => result,
            Err(err) => {
                CURSOR.set(None);
                log!(
                    Topic::Rpc,
                    Warn,
                    "placement receipt acknowledgement stopped after invariant failure: {err}"
                );
                return TimerRunResult::invariant_failure();
            }
        };

        if result.work_count > 0 {
            RETRY_STREAK.set(0);
        }

        match result.directive {
            DrainDirective::Continue => {
                TimerRunResult::success(result.work_count, TimerDirective::ContinueImmediately)
            }
            DrainDirective::Retry => {
                let streak = RETRY_STREAK.get();
                let delay = retry_delay(streak);
                RETRY_STREAK.set(streak.saturating_add(1));
                TimerRunResult {
                    outcome: crate::domain::runtime::TimerExecutionOutcome::RetryableFailure,
                    work_count: result.work_count,
                    directive: TimerDirective::RetryAfter(delay),
                }
            }
            DrainDirective::Stop if result.work_count == 0 => {
                RETRY_STREAK.set(0);
                TimerRunResult::no_work(TimerDirective::Stop)
            }
            DrainDirective::Stop => {
                RETRY_STREAK.set(0);
                TimerRunResult::success(result.work_count, TimerDirective::Stop)
            }
        }
    }

    async fn drain_batch() -> Result<DrainResult, InternalError> {
        let cursor = CURSOR.get();
        let page = ReceiptBackedIntentOps::list_placement_acknowledgement_page(
            cursor,
            ACKNOWLEDGEMENT_BATCH_SIZE,
        )?;
        let mut work_count = 0u64;
        let root_pid = EnvOps::root_pid().map_err(|err| {
            err.with_diagnostic_context("resolve root before placement receipt acknowledgement")
        })?;

        for intent in page.intents {
            let operation_id = intent.operation_id;
            if let Err(err) =
                RequestOps::acknowledge_placement_receipt(root_pid, operation_id).await
            {
                if is_retryable_root_failure(&err) {
                    log!(
                        Topic::Rpc,
                        Warn,
                        "placement receipt acknowledgement will retry operation_id={operation_id}: {err}"
                    );
                    return Ok(DrainResult {
                        work_count,
                        directive: DrainDirective::Retry,
                    });
                }
                return Err(err.with_diagnostic_context(format!(
                    "root rejected placement receipt acknowledgement for {operation_id}"
                )));
            }

            remove_exact_terminal_intent(&intent)?;
            work_count = work_count.saturating_add(1);
        }

        if page.next_cursor.is_some() {
            CURSOR.set(page.next_cursor);
            return Ok(DrainResult {
                work_count,
                directive: DrainDirective::Continue,
            });
        }

        CURSOR.set(None);
        let directive = if ReceiptBackedIntentOps::has_placement_acknowledgements()? {
            DrainDirective::Continue
        } else {
            DrainDirective::Stop
        };
        Ok(DrainResult {
            work_count,
            directive,
        })
    }
}

const fn is_retryable_root_failure(err: &InternalError) -> bool {
    matches!(
        err.class(),
        InternalErrorClass::Infra | InternalErrorClass::Ops
    )
}

fn retry_delay(streak: u8) -> Duration {
    let exponent = u32::from(streak.min(5));
    let multiplier = 1u32 << exponent;
    RETRY_INITIAL
        .checked_mul(multiplier)
        .unwrap_or(RETRY_MAX)
        .min(RETRY_MAX)
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InternalErrorOrigin;

    #[test]
    fn placement_acknowledgement_retry_backoff_is_bounded_and_deterministic() {
        assert_eq!(retry_delay(0), Duration::from_mins(1));
        assert_eq!(retry_delay(1), Duration::from_mins(2));
        assert_eq!(retry_delay(2), Duration::from_mins(4));
        assert_eq!(retry_delay(3), Duration::from_mins(8));
        assert_eq!(retry_delay(4), Duration::from_mins(16));
        assert_eq!(retry_delay(5), Duration::from_mins(30));
        assert_eq!(retry_delay(u8::MAX), Duration::from_mins(30));
    }

    #[test]
    fn only_transport_classes_are_retryable() {
        assert!(is_retryable_root_failure(&InternalError::ops(
            InternalErrorOrigin::Ops,
            "transport"
        )));
        assert!(is_retryable_root_failure(&InternalError::infra(
            InternalErrorOrigin::Infra,
            "transport"
        )));
        assert!(!is_retryable_root_failure(&InternalError::public(
            crate::dto::error::Error::conflict("root rejection")
        )));
        assert!(!is_retryable_root_failure(&InternalError::invariant(
            InternalErrorOrigin::Workflow,
            "local contradiction"
        )));
    }
}
