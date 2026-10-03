//! Release refusal preserves producers until read-only timer and paid-owner checks succeed.

use super::*;
use crate::{diagnostics::codes, ops::storage::async_job_recovery::AsyncJobOwner};
use std::cell::{Cell, RefCell};

#[test]
fn busy_release_preflight_never_observes_settlement_or_cancels_producers() {
    for error in [
        TimerError::ActiveJob(AsyncJobOwner::CanisterPoolMaintenance),
        TimerError::RunningClaim("canic/canister_pool/maintain".into()),
        TimerError::CustodyBusy,
    ] {
        let observed = Cell::new(false);
        let suspended = Cell::new(false);
        let failure = quiesce_release(
            || Err(error),
            || {
                observed.set(true);
                Ok(())
            },
            || {
                suspended.set(true);
                Ok(())
            },
        )
        .unwrap_err();
        assert_eq!(failure.code(), codes::STATE_CONFLICT);
        assert!(!observed.get());
        assert!(!suspended.get());
    }
}

#[test]
fn unpaid_obligation_refuses_before_cancellation_and_retry_runs_in_order() {
    let steps = RefCell::new(Vec::new());
    let unsettled = Cell::new(true);
    let attempt = || {
        quiesce_release(
            || {
                steps.borrow_mut().push("preflight");
                Ok(())
            },
            || {
                steps.borrow_mut().push("settlement");
                if unsettled.get() {
                    Err(InternalError::conflict())
                } else {
                    Ok(())
                }
            },
            || {
                steps.borrow_mut().push("suspend");
                Ok(())
            },
        )
    };
    assert_eq!(attempt().unwrap_err().code(), codes::STATE_CONFLICT);
    assert_eq!(*steps.borrow(), ["preflight", "settlement"]);
    steps.borrow_mut().clear();
    unsettled.set(false);
    attempt().unwrap();
    assert_eq!(*steps.borrow(), ["preflight", "settlement", "suspend"]);
}

#[test]
fn unexpected_timer_custody_does_not_become_a_retryable_busy_refusal() {
    let failure = quiesce_release(
        || Err(TimerError::MissingClaim),
        || panic!("preflight refusal must precede settlement"),
        || panic!("preflight refusal must precede cancellation"),
    )
    .unwrap_err();
    assert_eq!(failure.code(), InternalError::invariant().code());
}
