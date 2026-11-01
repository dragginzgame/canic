//! Commit-before-dispatch and shared-runtime wakeup composition for this application.
use super::{
    model::{self, DeliveryRecord},
    ops,
};

pub use super::ops::{corrupt_for_qualification, receive, view};
use canic::Error;
use ic_timers::{
    TimerCadence, TimerIdentity, WatchdogReconcileState, WatchdogRegistration, WatchdogRunResult,
};
use std::{
    cell::{Cell, RefCell},
    time::Duration,
};

thread_local! {
    static WAKEUP: RefCell<Option<WatchdogRegistration>> = const { RefCell::new(None) };
    // Private fixture faults only; never a reusable framework scheduling policy.
    static FAULT: Cell<u8> = const { Cell::new(0) };
}
pub fn install() {
    ops::open(true);
    reconcile_wakeup().expect("initial queue wakeup");
}
pub fn restore() {
    ops::open(false);
    reconcile_wakeup().expect("restored queue wakeup");
}
pub fn set_fault(mode: u8) -> Result<(), Error> {
    if mode > 2 {
        return Err(model::conflict());
    }
    FAULT.with(|fault| fault.set(mode));
    Ok(())
}
pub fn enqueue(delta: u64, delay_secs: u64) -> Result<u128, Error> {
    let id = ops::enqueue(delta, delay_secs)?;
    reconcile_wakeup().expect("committed queue wakeup");
    Ok(id)
}
pub fn reconcile(job: u128, sequence: u64) -> Result<(), Error> {
    ops::reconcile(job, sequence)?;
    reconcile_wakeup().expect("reconciled queue wakeup");
    Ok(())
}
fn reconcile_wakeup() -> Result<(), Box<dyn std::error::Error>> {
    let due = ops::due().map_err(|_| "invalid queue")?;
    WAKEUP.with_borrow_mut(|slot| {
        if due.is_none() {
            if let Some(registration) = slot.take() {
                registration.unregister()?;
            }
            return Ok(());
        }
        ic_timers::reconcile_watchdog(
            slot,
            &TimerIdentity::try_new("managed-probe", "jobs", "dispatch")?,
            TimerCadence::new(Duration::from_secs(1))?,
            due.map_or(
                WatchdogReconcileState::Inactive,
                WatchdogReconcileState::ScheduledAt,
            ),
            |_| wake(),
        )?;
        Ok(())
    })
}
fn begin() -> Result<ops::Dispatch, Error> {
    canic::__internal::core::api::timer::TimerApi::require_active()
        .map_err(|_| model::conflict())?;
    let dispatch = ops::begin()?;
    if dispatch.delivery.is_some() && FAULT.with(Cell::get) == 1 {
        ic_platform::cdk::api::trap("Jobs qualification: trap after intent write, before submit");
    }
    Ok(dispatch)
}
fn wake() -> WatchdogRunResult {
    let dispatch = begin().expect("validated committed dispatch");
    let work = u64::from(dispatch.delivery.is_some());
    if let Some(delivery) = dispatch.delivery {
        ic_platform::cdk::futures::spawn(deliver(delivery));
    }
    WatchdogRunResult::completed_batch(work, dispatch.next_due_ns)
}
pub async fn drain() -> Result<(), Error> {
    if let Some(delivery) = begin()?.delivery {
        deliver(delivery).await;
    }
    Ok(())
}
async fn deliver(delivery: DeliveryRecord) {
    let confirmed = ops::confirm_delivery(delivery).await;
    if confirmed && FAULT.with(Cell::get) == 2 {
        ic_platform::cdk::api::trap(
            "Jobs qualification: response lost after committed destination effect",
        );
    }
    ops::finish(delivery, confirmed).expect("exact completion from freshly reloaded state");
    reconcile_wakeup().expect("completion wakeup");
}
