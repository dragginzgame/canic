use crate::ops::runtime::timer::{TimerId, TimerOps};
use std::{cell::RefCell, future::Future, rc::Rc, thread::LocalKey, time::Duration};

///
/// TimerHandle
/// Opaque handle for scheduled timers (no direct access to TimerId).
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimerHandle(TimerId);

///
/// TimerSlot
/// Opaque timer slot handle for guarded scheduling.
///

pub type TimerSlot = LocalKey<RefCell<Option<TimerHandle>>>;

///
/// TimerApi
/// Lifecycle timer api façade for macro-expanded entrypoints.
///

pub struct TimerApi;

impl TimerApi {
    /// Public, stable surface for macro-expanded code in downstream crates.
    /// Performs no logic; delegates to internal TimerOps.
    pub fn set_lifecycle_timer(
        delay: Duration,
        label: impl Into<String>,
        task: impl Future<Output = ()> + 'static,
    ) -> TimerHandle {
        TimerHandle(TimerOps::set(delay, label, task))
    }

    /// Schedule a one-shot timer only if the slot is empty.
    /// Returns true when a new timer was scheduled.
    pub fn set_guarded(
        slot: &'static TimerSlot,
        delay: Duration,
        label: impl Into<String>,
        task: impl Future<Output = ()> + 'static,
    ) -> bool {
        slot.with_borrow_mut(|entry| {
            if entry.is_some() {
                return false;
            }

            let id = TimerOps::set(delay, label, task);
            *entry = Some(TimerHandle(id));
            true
        })
    }

    /// Schedule a repeating timer. Task produces a fresh Future on each tick.
    pub fn set_interval<F, Fut>(
        interval: Duration,
        label: impl Into<String>,
        task: F,
    ) -> TimerHandle
    where
        F: FnMut() -> Fut + 'static,
        Fut: Future<Output = ()> + 'static,
    {
        TimerHandle(TimerOps::set_interval(interval, label, task))
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
        let init_label = init_label.into();
        let interval_label = interval_label.into();

        slot.with_borrow_mut(|entry| {
            if entry.is_some() {
                return false;
            }

            let init_id_cell = Rc::new(RefCell::new(None));
            let init_id_cell_task = Rc::clone(&init_id_cell);

            let init_id = TimerOps::set(init_delay, init_label, async move {
                init_task().await;

                let init_id = init_id_cell_task.borrow();
                let Some(init_id) = init_id.as_ref() else {
                    return;
                };

                let still_armed = slot.with_borrow(|slot_val| slot_val.as_ref() == Some(init_id));
                if !still_armed {
                    return;
                }

                let interval_id = TimerOps::set_interval(interval, interval_label, tick_task);
                let interval_handle = TimerHandle(interval_id);

                // Atomically replace the slot value and clear the previous timer id.
                // This prevents orphaned timers if callers clear around the handover.
                slot.with_borrow_mut(|slot_val| {
                    let old = slot_val.replace(interval_handle);
                    if let Some(old_id) = old
                        && old_id != interval_handle
                    {
                        TimerOps::clear(old_id.0);
                    }
                });
            });

            let init_handle = TimerHandle(init_id);
            *init_id_cell.borrow_mut() = Some(init_handle);
            *entry = Some(init_handle);

            true
        })
    }

    /// Optional cancellation.
    pub fn clear_lifecycle_timer(handle: TimerHandle) {
        TimerOps::clear(handle.0);
    }
}
