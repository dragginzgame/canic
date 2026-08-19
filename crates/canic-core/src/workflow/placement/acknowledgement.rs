//! Module: workflow::placement::acknowledgement
//!
//! Responsibility: drain durable terminal placement receipts through root acknowledgement.
//! Does not own: placement settlement, root replay receipts, or stable record schemas.
//! Boundary: consumes the placement-only index through ops and schedules only known work.

use crate::{
    InternalError, log,
    log::Topic,
    model::replay::OperationId,
    ops::{
        rpc::request::RequestOps, runtime::env::EnvOps, storage::async_job_recovery::AsyncJobOwner,
        storage::intent::ReceiptBackedIntentOps,
    },
    workflow::{
        placement::allocation::remove_exact_terminal_intent,
        runtime::{
            async_job::AsyncJobWorkflow,
            timer::{
                TimerAuthorityWorkflow, TimerError, require_active, retain_owned_once,
                with_owned_once,
            },
        },
    },
};
use ic_timers::{
    DeclarationLifetime, OnceContext, OnceRegistration, TimerCompletion, TimerDirective,
    TimerIdentity, TimerRunResult, TimerSchedule, register_once,
};
use std::{
    cell::{Cell, RefCell},
    time::Duration,
};

const ACKNOWLEDGEMENT_BATCH_SIZE: usize = 32;
const RETRY_INITIAL: Duration = Duration::from_mins(1);
const RETRY_MAX: Duration = Duration::from_mins(30);

thread_local! {
    static ACKNOWLEDGEMENT_TIMER: RefCell<Option<OnceRegistration>> = const { RefCell::new(None) };
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
    /// Return the exact placement-acknowledgement native identity.
    pub(crate) fn timer_identity() -> Result<TimerIdentity, TimerError> {
        TimerIdentity::try_new("canic", "placement", "receipt_ack").map_err(Into::into)
    }

    /// Return the claimed placement-acknowledgement identity, when declared.
    pub(crate) fn claimed_timer_identity() -> Result<Option<TimerIdentity>, TimerError> {
        with_owned_once(&ACKNOWLEDGEMENT_TIMER, |registration| {
            registration.identity().clone()
        })
    }

    /// Cancel the retained placement-acknowledgement registration for suspension.
    pub(crate) fn cancel_timer() -> Result<(), TimerError> {
        if let Some(result) = with_owned_once(&ACKNOWLEDGEMENT_TIMER, OnceRegistration::cancel)? {
            result?;
        }
        Ok(())
    }

    /// Recover one expired acknowledgement attempt from the durable receipt index.
    pub(crate) fn recover_expired_timer(now_ns: u64) -> bool {
        let owner = AsyncJobOwner::PlacementReceiptAcknowledgement;
        if !AsyncJobWorkflow::has_expired_attempt(owner, now_ns) {
            return false;
        }
        match ReceiptBackedIntentOps::has_placement_acknowledgements() {
            Ok(true) => {}
            Ok(false) => return AsyncJobWorkflow::abandon_expired(owner, now_ns),
            Err(_) => return false,
        }
        let Some(attempt) = AsyncJobWorkflow::claim_expired(owner, now_ns) else {
            return false;
        };
        ic_cdk::futures::spawn(async move {
            let result = Self::run_scheduled().await;
            let _ = AsyncJobWorkflow::finish(attempt, result);
        });
        true
    }

    /// Reconstruct scheduling from the lifecycle-rebuilt durable index.
    pub fn start() -> Result<(), InternalError> {
        Self::schedule_if_pending()
    }

    /// Advance the worker immediately when exact terminal evidence is added.
    pub fn schedule_if_pending() -> Result<(), InternalError> {
        if ReceiptBackedIntentOps::has_placement_acknowledgements()? {
            Self::schedule(Duration::ZERO)?;
        } else {
            Self::reconcile_timer(None)?;
        }
        Ok(())
    }

    fn schedule(delay: Duration) -> Result<(), InternalError> {
        require_active()?;
        Self::declare_timer()?;
        TimerAuthorityWorkflow::ensure_async_job_recovery_watchdog()?;
        let result = with_owned_once(&ACKNOWLEDGEMENT_TIMER, |registration| {
            registration.ensure_scheduled(TimerSchedule::After(delay))
        })?
        .ok_or(TimerError::MissingClaim)?;
        result.map_err(TimerError::from)?;
        Ok(())
    }

    async fn run_registered() -> TimerRunResult {
        let attempt = match AsyncJobWorkflow::claim(AsyncJobOwner::PlacementReceiptAcknowledgement)
        {
            Ok(attempt) => attempt,
            Err(result) => return result,
        };
        let result = Self::run_scheduled().await;
        AsyncJobWorkflow::finish(attempt, result)
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
                return TimerRunResult::new(
                    TimerCompletion::invariant_failure(0),
                    TimerDirective::Stop,
                );
            }
        };

        if result.work_count > 0 {
            RETRY_STREAK.set(0);
        }

        match result.directive {
            DrainDirective::Continue => TimerRunResult::new(
                TimerCompletion::success(result.work_count),
                TimerDirective::ContinueImmediately,
            ),
            DrainDirective::Retry => {
                let streak = RETRY_STREAK.get();
                let delay = retry_delay(streak);
                RETRY_STREAK.set(streak.saturating_add(1));
                TimerRunResult::new(
                    TimerCompletion::retryable_failure(result.work_count),
                    TimerDirective::RetryAfter(delay),
                )
            }
            DrainDirective::Stop if result.work_count == 0 => {
                RETRY_STREAK.set(0);
                TimerRunResult::new(TimerCompletion::no_work(), TimerDirective::Stop)
            }
            DrainDirective::Stop => {
                RETRY_STREAK.set(0);
                TimerRunResult::new(
                    TimerCompletion::success(result.work_count),
                    TimerDirective::Stop,
                )
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
        let root_pid = EnvOps::root_pid()?;

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
                return Err(err);
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

    fn reconcile_timer(deadline_ns: Option<u64>) -> Result<(), TimerError> {
        require_active()?;
        if deadline_ns.is_some() {
            Self::declare_timer()?;
        }
        if deadline_ns.is_some()
            || AsyncJobWorkflow::has_active_attempt(AsyncJobOwner::PlacementReceiptAcknowledgement)
        {
            TimerAuthorityWorkflow::ensure_async_job_recovery_watchdog()?;
        }
        if let Some(result) = with_owned_once(&ACKNOWLEDGEMENT_TIMER, |registration| {
            registration.reconcile_schedule(deadline_ns.map(TimerSchedule::At))
        })? {
            result?;
        }
        Ok(())
    }

    fn declare_timer() -> Result<(), TimerError> {
        require_active()?;
        if with_owned_once(&ACKNOWLEDGEMENT_TIMER, |_| ())?.is_some() {
            return Ok(());
        }
        let registration = register_once(
            Self::timer_identity()?,
            DeclarationLifetime::Retained,
            |_context: OnceContext| async { Self::run_registered().await },
        )?;
        retain_owned_once(&ACKNOWLEDGEMENT_TIMER, registration)
    }
}

const fn is_retryable_root_failure(err: &InternalError) -> bool {
    err.code().raw_code().raw() == crate::diagnostics::codes::PLATFORM_FAILED.raw_code().raw()
        || err.code().raw_code().raw() == crate::diagnostics::codes::STATE_FAILED.raw_code().raw()
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
        assert!(is_retryable_root_failure(&InternalError::state_failure()));
        assert!(is_retryable_root_failure(&InternalError::platform_failure()));
        assert!(!is_retryable_root_failure(&InternalError::public(
            crate::diagnostics::codes::STATE_CONFLICT
        )));
        assert!(!is_retryable_root_failure(&InternalError::invariant()));
    }
}
