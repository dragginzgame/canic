use crate::{
    cdk::timers::{
        clear_timer as cdk_clear_timer, set_timer as cdk_set_timer,
        set_timer_interval as cdk_set_timer_interval,
    },
    model::metrics::{SystemMetricKind, SystemMetrics, TimerMetrics, TimerMode},
};

pub use crate::cdk::timers::TimerId;
use std::{future::Future, time::Duration};

///
/// Timer
///

pub struct Timer;

impl Timer {
    /// Schedule a one-shot timer and record metrics.
    pub fn set(delay: Duration, label: &str, task: impl Future<Output = ()> + 'static) -> TimerId {
        SystemMetrics::increment(SystemMetricKind::TimerScheduled);
        TimerMetrics::increment(TimerMode::Once, delay, label);

        cdk_set_timer(delay, task)
    }

    /// Schedule a repeating timer and record metrics.
    pub fn set_interval<F, Fut>(interval: Duration, label: &str, task: F) -> TimerId
    where
        F: FnMut() -> Fut + 'static,
        Fut: Future<Output = ()> + 'static,
    {
        SystemMetrics::increment(SystemMetricKind::TimerScheduled);
        TimerMetrics::increment(TimerMode::Interval, interval, label);

        cdk_set_timer_interval(interval, task)
    }

    /// Clear a timer without recording metrics.
    pub fn clear(id: TimerId) {
        cdk_clear_timer(id);
    }
}
