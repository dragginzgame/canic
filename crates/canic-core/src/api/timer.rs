//! Module: api::timer
//!
//! Responsibility: expose the maintained application and lifecycle timer facade.
//! Does not own: timer state, recurrence, arbitration, or domain scheduling policy.
//! Boundary: macro-expanded downstream code delegates to TimerWorkflow.

use crate::workflow::runtime::timer::{ApplicationTimerId, TimerWorkflow};
use std::{future::Future, time::Duration};

/// Opaque, single-owner handle for a cancellable application timer.
#[derive(Debug, Eq, PartialEq)]
pub struct TimerHandle(ApplicationTimerId);

/// Public timer facade used by Canic's macro-expanded entrypoints.
pub struct TimerApi;

impl TimerApi {
    /// Schedule a cancellable application one-shot.
    pub fn set(
        delay: Duration,
        label: impl Into<String>,
        task: impl Future<Output = ()> + 'static,
    ) -> TimerHandle {
        TimerHandle(TimerWorkflow::set_application_once(delay, label, task))
    }

    /// Defer lifecycle work through the same one-shot authority.
    pub fn defer_lifecycle(
        delay: Duration,
        label: impl Into<String>,
        task: impl Future<Output = ()> + 'static,
    ) -> TimerHandle {
        Self::set(delay, label, task)
    }

    /// Schedule a cancellable, non-overlapping after-completion interval.
    pub fn set_interval<F, Fut>(
        interval: Duration,
        label: impl Into<String>,
        task: F,
    ) -> TimerHandle
    where
        F: FnMut() -> Fut + 'static,
        Fut: Future<Output = ()> + 'static,
    {
        TimerHandle(TimerWorkflow::set_application_interval(
            interval, label, task,
        ))
    }

    /// Consume a timer handle and suppress any future invocation.
    #[must_use]
    pub fn cancel(handle: TimerHandle) -> bool {
        TimerWorkflow::cancel_application(handle.0)
    }
}
