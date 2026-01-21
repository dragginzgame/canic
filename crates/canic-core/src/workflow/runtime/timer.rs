use crate::ops::runtime::timer::TimerOps;
use std::{cell::RefCell, future::Future, thread::LocalKey, time::Duration};

// re-exports
pub use crate::ops::runtime::timer::TimerId;

///
/// TimerWorkflow
///

pub struct TimerWorkflow;

impl TimerWorkflow {
    pub fn set(
        delay: Duration,
        label: impl Into<String>,
        task: impl Future<Output = ()> + 'static,
    ) -> TimerId {
        TimerOps::set(delay, label, task)
    }

    pub fn set_interval<F, Fut>(interval: Duration, label: impl Into<String>, task: F) -> TimerId
    where
        F: FnMut() -> Fut + 'static,
        Fut: Future<Output = ()> + 'static,
    {
        TimerOps::set_interval(interval, label, task)
    }

    pub fn clear(id: TimerId) {
        TimerOps::clear(id);
    }

    pub fn set_guarded(
        slot: &'static LocalKey<RefCell<Option<TimerId>>>,
        delay: Duration,
        label: impl Into<String>,
        task: impl Future<Output = ()> + 'static,
    ) -> bool {
        TimerOps::set_guarded(slot, delay, label, task)
    }

    pub fn set_guarded_interval<FInit, InitFut, FTick, TickFut>(
        slot: &'static LocalKey<RefCell<Option<TimerId>>>,
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

    #[must_use]
    pub fn clear_guarded(slot: &'static LocalKey<RefCell<Option<TimerId>>>) -> bool {
        TimerOps::clear_guarded(slot)
    }
}
