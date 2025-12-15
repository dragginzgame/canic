#![allow(clippy::disallowed_methods)]

pub use crate::cdk::timers::TimerId;

use crate::{
    cdk::timers::{
        clear_timer as cdk_clear_timer, set_timer as cdk_set_timer,
        set_timer_interval as cdk_set_timer_interval,
    },
    model::metrics::{
        system::{SystemMetricKind, SystemMetrics},
        timer::{TimerMetrics, TimerMode},
    },
    ops::runtime::perf::PerfOps,
    perf::perf_counter,
};
use std::{cell::RefCell, future::Future, rc::Rc, time::Duration};

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

        SystemMetrics::increment(SystemMetricKind::TimerScheduled);
        TimerMetrics::ensure(TimerMode::Once, delay, &label);

        cdk_set_timer(delay, async move {
            TimerMetrics::increment(TimerMode::Once, delay, &label);

            let start = perf_counter();
            task.await;
            let end = perf_counter();

            PerfOps::record(&label, end.saturating_sub(start));
        })
    }

    /// Schedules a repeating timer.
    /// The task is a closure that produces a fresh Future on each tick.
    pub fn set_interval<F, Fut>(interval: Duration, label: impl Into<String>, task: F) -> TimerId
    where
        F: FnMut() -> Fut + 'static,
        Fut: Future<Output = ()> + 'static,
    {
        let label = label.into();

        SystemMetrics::increment(SystemMetricKind::TimerScheduled);
        TimerMetrics::ensure(TimerMode::Interval, interval, &label);

        let task = Rc::new(RefCell::new(task));

        cdk_set_timer_interval(interval, move || {
            let label = label.clone();
            let interval = interval;
            let task = Rc::clone(&task);

            async move {
                TimerMetrics::increment(TimerMode::Interval, interval, &label);

                let start = perf_counter();
                let fut = { (task.borrow_mut())() };
                fut.await;
                let end = perf_counter();

                PerfOps::record(&label, end.saturating_sub(start));
            }
        })
    }

    /// Clears a previously scheduled timer.
    pub fn clear(id: TimerId) {
        cdk_clear_timer(id);
    }
}
