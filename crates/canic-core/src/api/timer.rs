//! Module: api::timer
//!
//! Responsibility: expose hidden lifecycle adapters for macro-generated entrypoints.
//! Does not own: timer state, recurrence, arbitration, or domain scheduling policy.
//! Boundary: framework lifecycle and the exact Root owner delegate to timer authority.

use crate::workflow::runtime::timer::{
    TimerAuthorityWorkflow, recovery_watchdog_identity, require_active,
};
use ic_timers::TimerRunResult;
use std::{future::Future, time::Duration};

pub use crate::workflow::runtime::timer::TimerError;

/// Hidden timer adapter used by Canic's macro-expanded lifecycle entrypoints.
#[doc(hidden)]
pub struct TimerApi;

impl TimerApi {
    /// Initialize the shared runtime and fixed non-root declarations during lifecycle restore.
    #[doc(hidden)]
    pub fn initialize_nonroot_runtime_required() {
        TimerAuthorityWorkflow::initialize_nonroot_runtime()
            .unwrap_or_else(|error| ic_cdk::trap(format!("timer runtime init failed: {error}")));
    }

    /// Initialize the shared runtime and fixed root declarations during lifecycle restore.
    #[doc(hidden)]
    pub fn initialize_root_runtime_required() {
        TimerAuthorityWorkflow::initialize_root_runtime()
            .unwrap_or_else(|error| ic_cdk::trap(format!("timer runtime init failed: {error}")));
    }

    /// Initialize the shared runtime for a canister with no Canic runtime jobs.
    #[doc(hidden)]
    pub fn initialize_shared_runtime_required() {
        TimerAuthorityWorkflow::initialize_shared_runtime()
            .unwrap_or_else(|error| ic_cdk::trap(format!("timer runtime init failed: {error}")));
    }

    /// Restore volatile suspension from the durable authority fence.
    #[doc(hidden)]
    pub fn restore_snapshot_suspension(sealed: bool) {
        TimerAuthorityWorkflow::restore_snapshot_suspension(sealed);
    }

    /// Prove exact Root-native claims and business attempts are safe to suspend.
    #[doc(hidden)]
    pub fn require_root_authority_snapshot_resumable() -> Result<(), TimerError> {
        TimerAuthorityWorkflow::require_root_resumable()
    }

    /// Reject Root-owned scheduling while the durable snapshot fence is sealed.
    #[doc(hidden)]
    pub fn require_active() -> Result<(), TimerError> {
        require_active()
    }

    /// Return the exact identity shared by each role's sole recovery watchdog.
    #[doc(hidden)]
    pub fn recovery_watchdog_identity() -> Result<ic_timers::TimerIdentity, TimerError> {
        recovery_watchdog_identity()
    }

    /// Recover expired core business attempts for the Root-owned watchdog.
    #[doc(hidden)]
    #[must_use]
    pub fn recover_expired_async_jobs(now_ns: u64) -> u64 {
        TimerAuthorityWorkflow::recover_expired_async_jobs(now_ns)
    }

    /// Schedule framework-owned lifecycle work and trap if runtime invariants reject it.
    #[doc(hidden)]
    pub fn defer_lifecycle_required(
        delay: Duration,
        label: impl Into<String>,
        task: impl Future<Output = ()> + 'static,
    ) {
        TimerAuthorityWorkflow::defer_lifecycle_once(delay, label, task)
            .unwrap_or_else(|error| ic_cdk::trap(format!("lifecycle timer rejected: {error}")));
    }

    /// Schedule framework lifecycle work with a truthful typed completion.
    #[doc(hidden)]
    pub fn defer_lifecycle_result_required(
        delay: Duration,
        label: impl Into<String>,
        task: impl Future<Output = TimerRunResult> + 'static,
    ) {
        TimerAuthorityWorkflow::defer_lifecycle_result_once(delay, label, task)
            .unwrap_or_else(|error| ic_cdk::trap(format!("lifecycle timer rejected: {error}")));
    }
}
