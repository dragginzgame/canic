use crate::ops::runtime::timer::{TimerId, TimerOps};
use std::{future::Future, time::Duration};

///
/// Lifecycle timer façade for macro-expanded entrypoints.
///
/// This function exists **solely** to provide a public, stable surface that
/// macro-generated code in downstream crates can call to schedule timers
/// during canister initialization and upgrade.
///
/// Rationale:
/// - `TimerOps` is intentionally `pub(crate)` to prevent arbitrary use from
///   endpoints and user code.
/// - Macros expand in downstream crates and therefore cannot access `ops`.
/// - Scheduling init/upgrade timers is *infrastructure wiring*, not a domain
///   workflow, so it does **not** belong in `workflow`.
///
/// This wrapper performs **no logic** and enforces **no policy**. It simply
/// forwards to `TimerOps` while preserving layering:
///
/// `macro → api → ops → infra`
///
/// If you are looking for business or orchestration logic, it does not belong
/// here.
///
pub fn set_lifecycle_timer(
    delay: Duration,
    label: &'static str,
    task: impl Future<Output = ()> + 'static,
) {
    TimerOps::set(delay, label, task);
}

/// Optional: exposed only if you need cancellation from macro code.
pub fn clear_lifecycle_timer(id: TimerId) {
    TimerOps::clear(id);
}
