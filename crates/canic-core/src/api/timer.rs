//! Module: api::timer
//!
//! Responsibility: expose the maintained application and lifecycle timer facade.
//! Does not own: timer state, recurrence, arbitration, or domain scheduling policy.
//! Boundary: macro-expanded downstream code delegates to TimerWorkflow.

use crate::workflow::runtime::timer::{TimerClaimId, TimerWorkflow};
use std::{future::Future, time::Duration};

pub use crate::workflow::runtime::timer::{TimerDirective, TimerError, TimerRunResult};

/// Opaque, single-owner handle for a cancellable application timer.
///
/// Dropping this handle detaches caller control; it does not cancel the timer.
/// Pass it to [`TimerApi::cancel`] to suppress future invocations.
#[must_use = "dropping a timer handle detaches control without cancelling the timer"]
#[derive(Debug, Eq, PartialEq)]
pub struct TimerHandle(TimerClaimId);

impl TimerHandle {
    /// Relinquish caller control while leaving the timer registered.
    pub const fn detach(self) {}
}

/// Public timer facade used by Canic's macro-expanded entrypoints.
pub struct TimerApi;

impl TimerApi {
    /// Initialize the shared runtime and fixed non-root declarations during lifecycle restore.
    #[doc(hidden)]
    pub fn initialize_nonroot_runtime_required() {
        TimerWorkflow::initialize_nonroot_runtime()
            .unwrap_or_else(|error| ic_cdk::trap(format!("timer runtime init failed: {error}")));
    }

    /// Initialize the shared runtime and fixed root declarations during lifecycle restore.
    #[doc(hidden)]
    pub fn initialize_root_runtime_required() {
        TimerWorkflow::initialize_root_runtime()
            .unwrap_or_else(|error| ic_cdk::trap(format!("timer runtime init failed: {error}")));
    }

    /// Initialize the shared runtime for a canister with no Canic runtime jobs.
    #[doc(hidden)]
    pub fn initialize_shared_runtime_required() {
        TimerWorkflow::initialize_shared_runtime()
            .unwrap_or_else(|error| ic_cdk::trap(format!("timer runtime init failed: {error}")));
    }

    /// Restore volatile suspension from the durable authority fence.
    #[doc(hidden)]
    pub fn restore_snapshot_suspension(sealed: bool) {
        TimerWorkflow::restore_snapshot_suspension(sealed);
    }

    /// Register the root control-plane's snapshot-resume timer reconciler.
    #[doc(hidden)]
    pub fn register_snapshot_resume_participant(participant: fn() -> Result<(), TimerError>) {
        TimerWorkflow::register_snapshot_resume_participant(participant);
    }

    /// Register the root control-plane's synchronous async-recovery dispatcher.
    #[doc(hidden)]
    pub fn register_async_recovery_participant(participant: fn() -> bool) {
        TimerWorkflow::register_async_recovery_participant(participant);
    }

    /// Arm the internal watchdog after a recovery owner reconstructs durable demand.
    #[doc(hidden)]
    pub fn ensure_async_recovery_watchdog_required() {
        TimerWorkflow::ensure_async_recovery_watchdog().unwrap_or_else(|error| {
            ic_cdk::trap(format!("async recovery watchdog rejected: {error}"))
        });
    }

    /// Schedule a cancellable application one-shot.
    pub fn set(
        delay: Duration,
        label: impl Into<String>,
        task: impl Future<Output = ()> + 'static,
    ) -> Result<TimerHandle, TimerError> {
        TimerWorkflow::set_application_once(delay, label, task).map(TimerHandle)
    }

    /// Defer lifecycle work through the same one-shot authority.
    pub fn defer_lifecycle(
        delay: Duration,
        label: impl Into<String>,
        task: impl Future<Output = ()> + 'static,
    ) -> Result<TimerHandle, TimerError> {
        TimerWorkflow::set_lifecycle_once(delay, label, task).map(TimerHandle)
    }

    /// Schedule framework-owned lifecycle work and trap if runtime invariants reject it.
    #[doc(hidden)]
    pub fn defer_lifecycle_required(
        delay: Duration,
        label: impl Into<String>,
        task: impl Future<Output = ()> + 'static,
    ) -> TimerHandle {
        Self::defer_lifecycle(delay, label, task)
            .unwrap_or_else(|error| ic_cdk::trap(format!("lifecycle timer rejected: {error}")))
    }

    /// Schedule framework lifecycle work with a truthful typed completion.
    #[doc(hidden)]
    pub fn defer_lifecycle_result_required(
        delay: Duration,
        label: impl Into<String>,
        task: impl Future<Output = TimerRunResult> + 'static,
    ) -> TimerHandle {
        TimerWorkflow::set_lifecycle_result_once(delay, label, task).map_or_else(
            |error| ic_cdk::trap(format!("lifecycle timer rejected: {error}")),
            TimerHandle,
        )
    }

    /// Schedule a cancellable, non-overlapping after-completion interval.
    pub fn set_interval<F, Fut>(
        interval: Duration,
        label: impl Into<String>,
        task: F,
    ) -> Result<TimerHandle, TimerError>
    where
        F: FnMut() -> Fut + 'static,
        Fut: Future<Output = ()> + 'static,
    {
        TimerWorkflow::set_application_interval(interval, label, task).map(TimerHandle)
    }

    /// Declare or re-arm root canister-pool maintenance.
    #[doc(hidden)]
    pub fn set_canister_pool_maintenance<F, Fut>(
        interval: Duration,
        task: F,
    ) -> Result<TimerHandle, TimerError>
    where
        F: FnMut() -> Fut + 'static,
        Fut: Future<Output = TimerRunResult> + 'static,
    {
        TimerWorkflow::set_canister_pool_maintenance(interval, task).map(TimerHandle)
    }

    /// Reserve inactive root canister-pool maintenance before application hooks.
    #[doc(hidden)]
    pub fn declare_canister_pool_maintenance<F, Fut>(
        interval: Duration,
        task: F,
    ) -> Result<TimerHandle, TimerError>
    where
        F: FnMut() -> Fut + 'static,
        Fut: Future<Output = TimerRunResult> + 'static,
    {
        TimerWorkflow::declare_canister_pool_maintenance(interval, task).map(TimerHandle)
    }

    /// Consume a timer handle and suppress any future invocation.
    pub fn cancel(handle: TimerHandle) -> Result<(), TimerError> {
        TimerWorkflow::cancel(handle.0)
    }
}
