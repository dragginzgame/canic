//! Project supplier counters; no recording, persistence or recovery authority.
use canic::metrics::{Counter, Snapshot, Timers};

/// Aggregate public timer diagnostics for this management owner.
/// # Panics
/// Traps if explicit lifecycle initialization has not completed.
#[must_use]
pub fn snapshot() -> Snapshot {
    let (started_at_version, started_at_ns) =
        super::STATE.with_borrow(|state| state.as_ref().expect("initialized manager").timer_window);
    Snapshot {
        observed_at_ns: super::api::time(),
        timers: ic_timers::metrics().ok().map(|m| {
            let [
                accepted,
                completed,
                abandoned,
                expected_failures,
                deferred_submissions,
                provider_faults,
            ] = [
                m.accepted,
                m.completed,
                m.abandoned,
                m.expected_failures,
                m.deferred_submissions,
                m.provider_faults,
            ]
            .map(|counter| Counter {
                total: counter.total(),
                overflowed: counter.total_overflowed(),
            });
            Timers {
                started_at_version,
                started_at_ns,
                accepted,
                completed,
                abandoned,
                expected_failures,
                deferred_submissions,
                provider_faults,
            }
        }),
    }
}
