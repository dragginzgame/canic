use crate::ops::runtime::timer::{TimerId, TimerOps};
use std::{cell::RefCell, future::Future, thread::LocalKey, time::Duration};

///
/// TimerSlot
///

pub type TimerSlot = LocalKey<RefCell<Option<TimerId>>>;

/// Lifecycle timer façade for macro-expanded entrypoints.
///
/// Public, stable surface for macro-expanded code in downstream crates.
/// Performs no logic; delegates to internal TimerOps.
///
/// Layering: macro → api → ops → infra
pub fn set_lifecycle_timer(
    delay: Duration,
    label: impl Into<String>,
    task: impl Future<Output = ()> + 'static,
) -> TimerId {
    TimerOps::set(delay, label, task)
}

/// Schedule a one-shot timer only if the slot is empty.
/// Returns true when a new timer was scheduled.
pub fn set_guarded(
    slot: &'static TimerSlot,
    delay: Duration,
    label: impl Into<String>,
    task: impl Future<Output = ()> + 'static,
) -> bool {
    TimerOps::set_guarded(slot, delay, label, task)
}

/// Schedule a repeating timer. Task produces a fresh Future on each tick.
pub fn set_interval<F, Fut>(interval: Duration, label: impl Into<String>, task: F) -> TimerId
where
    F: FnMut() -> Fut + 'static,
    Fut: Future<Output = ()> + 'static,
{
    TimerOps::set_interval(interval, label, task)
}

/// Schedule a guarded init timer that installs a repeating interval timer.
pub fn set_guarded_interval<FInit, InitFut, FTick, TickFut>(
    slot: &'static TimerSlot,
    init_delay: Duration,
    init_label: impl Into<String>,
    init_task: FInit,
    interval: Duration,
    interval_label: impl Into<String>,
    tick_task: FTick,
) -> bool
where
    FInit: FnOnce() -> InitFut + 'static,
    InitFut: Future<Output = ()> + 'static,
    FTick: FnMut() -> TickFut + 'static,
    TickFut: Future<Output = ()> + 'static,
{
    TimerOps::set_guarded_interval(
        slot,
        init_delay,
        init_label,
        init_task,
        interval,
        interval_label,
        tick_task,
    )
}

/// Optional cancellation.
pub fn clear_lifecycle_timer(id: TimerId) {
    TimerOps::clear(id);
}
