use crate::{
    cdk::timers::{
        clear_timer as cdk_clear_timer, set_timer as cdk_set_timer,
        set_timer_interval as cdk_set_timer_interval,
    },
    ops::{
        perf::PerfOps,
        runtime::metrics::timer::{TimerMetrics, TimerMode},
    },
    perf::perf_counter,
};
use std::{cell::RefCell, future::Future, rc::Rc, thread::LocalKey, time::Duration};

///
/// TimerId
/// Opaque ops-owned timer handle
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimerId(crate::cdk::timers::TimerId);

///
/// TimerOps
///

pub struct TimerOps;

impl TimerOps {
    /// Schedules a one-shot timer.
    /// The task is a single Future, consumed exactly once.
    pub fn set(
        delay: Duration,
        label: impl Into<String>,
        task: impl Future<Output = ()> + 'static,
    ) -> TimerId {
        let label = label.into();

        TimerMetrics::record_timer_scheduled(TimerMode::Once, delay, label.as_str());

        let id = cdk_set_timer(delay, async move {
            TimerMetrics::record_timer_tick(TimerMode::Once, delay, label.as_str());

            let start = perf_counter();
            task.await;
            let end = perf_counter();

            PerfOps::record(label.as_str(), end.saturating_sub(start));
        });

        TimerId(id)
    }

    /// Schedules a repeating timer.
    /// The task is a closure that produces a fresh Future on each tick.
    pub fn set_interval<F, Fut>(interval: Duration, label: impl Into<String>, task: F) -> TimerId
    where
        F: FnMut() -> Fut + 'static,
        Fut: Future<Output = ()> + 'static,
    {
        // Avoid cloning the String every tick.
        let label = Rc::new(label.into());

        TimerMetrics::record_timer_scheduled(TimerMode::Interval, interval, label.as_str());

        let task = Rc::new(RefCell::new(task));

        let id = cdk_set_timer_interval(interval, move || {
            let label = Rc::clone(&label);
            let task = Rc::clone(&task);

            async move {
                TimerMetrics::record_timer_tick(TimerMode::Interval, interval, label.as_str());

                let start = perf_counter();
                let fut = { (task.borrow_mut())() };
                fut.await;
                let end = perf_counter();

                PerfOps::record(label.as_str(), end.saturating_sub(start));
            }
        });

        TimerId(id)
    }

    /// Clears a previously scheduled timer.
    pub fn clear(id: TimerId) {
        cdk_clear_timer(id.0);
    }

    /// Schedule a one-shot timer only if the slot is empty.
    /// Returns true when a new timer was scheduled.
    pub fn set_guarded(
        slot: &'static LocalKey<RefCell<Option<TimerId>>>,
        delay: Duration,
        label: impl Into<String>,
        task: impl Future<Output = ()> + 'static,
    ) -> bool {
        slot.with_borrow_mut(|entry| {
            if entry.is_some() {
                return false;
            }

            let id = Self::set(delay, label, task);
            *entry = Some(id);
            true
        })
    }

    /// Schedule a guarded init timer that installs a repeating interval timer.
    /// Returns true when a new timer was scheduled.
    /// The interval is only installed if the slot still holds the init timer.
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
        let init_label = init_label.into();
        let interval_label = interval_label.into();

        slot.with_borrow_mut(|entry| {
            if entry.is_some() {
                return false;
            }

            let init_id_cell = Rc::new(RefCell::new(None));
            let init_id_cell_task = Rc::clone(&init_id_cell);

            let init_id = Self::set(init_delay, init_label, async move {
                init_task().await;

                let init_id = init_id_cell_task.borrow();
                let Some(init_id) = init_id.as_ref() else {
                    return;
                };

                let still_armed = slot.with_borrow(|slot_val| slot_val.as_ref() == Some(init_id));
                if !still_armed {
                    return;
                }

                let interval_id = Self::set_interval(interval, interval_label, tick_task);

                // Atomically replace the slot value and clear the previous timer id.
                // This prevents orphaned timers if callers clear around the handover.
                slot.with_borrow_mut(|slot_val| {
                    let old = slot_val.replace(interval_id);
                    if let Some(old_id) = old
                        && old_id != interval_id
                    {
                        Self::clear(old_id);
                    }
                });
            });

            *init_id_cell.borrow_mut() = Some(init_id);
            *entry = Some(init_id);
            true
        })
    }

    /// Clear a guarded timer slot if present.
    /// Returns true when a timer was cleared.
    ///
    /// NOTE: guarded one-shot timers do not auto-clear their slot on completion.
    /// Callers must clear the slot explicitly when the timer fires.
    #[must_use]
    pub fn clear_guarded(slot: &'static LocalKey<RefCell<Option<TimerId>>>) -> bool {
        slot.with_borrow_mut(|entry| {
            entry.take().is_some_and(|id| {
                Self::clear(id);
                true
            })
        })
    }
}
