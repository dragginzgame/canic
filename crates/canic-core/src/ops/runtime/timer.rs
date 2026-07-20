//! Module: ops::runtime::timer
//!
//! Responsibility: own the direct IC timer platform boundary.
//! Does not own: recurrence, timer identity, arbitration, or task policy.
//! Boundary: the common timer workflow is the only production caller.

use crate::{
    domain::runtime::TimerMode,
    ops::{perf::PerfOps, runtime::metrics::timer::TimerMetrics},
    perf::perf_counter,
};
use ic_cdk_timers::{
    TimerId as CdkTimerId, clear_timer as cdk_clear_timer, set_timer as cdk_set_timer,
};
use std::{future::Future, time::Duration};

/// Opaque operations-layer platform handle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimerId(CdkTimerId);

/// Direct platform operations for one-shot timers.
pub struct TimerOps;

impl TimerOps {
    /// Arm one measured platform callback.
    pub fn set(
        delay: Duration,
        mode: TimerMode,
        label: impl Into<String>,
        task: impl Future<Output = ()> + 'static,
    ) -> TimerId {
        let label = label.into();
        TimerMetrics::record_timer_scheduled(mode, delay, label.as_str());

        let id = cdk_set_timer(delay, async move {
            TimerMetrics::record_timer_tick(mode, delay, label.as_str());

            let start = perf_counter();
            task.await;
            let end = perf_counter();

            PerfOps::record(label.as_str(), end.saturating_sub(start));
        });

        TimerId(id)
    }

    /// Clear one still-armed platform callback.
    pub fn clear(id: TimerId) {
        cdk_clear_timer(id.0);
    }
}
