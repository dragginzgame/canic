//! Module: support::parallel
//!
//! Responsibility: collect read-only CLI reports with bounded, ordered workers.
//! Does not own: target selection, query policy, or per-target error projection.

#[cfg(test)]
mod tests;

use std::thread;

/// Bound simultaneous report queries independently of Fleet size.
const MAX_REPORT_WORKERS: usize = 8;

/// Preserve input order and project worker panics at the target's report owner.
pub fn collect_ordered_reports<T: Sync, R: Send>(
    items: &[T],
    query: impl Fn(&T) -> R + Sync,
    worker_panic: impl Fn(&T) -> R,
) -> Vec<R> {
    let mut reports = Vec::with_capacity(items.len());
    for batch in items.chunks(MAX_REPORT_WORKERS) {
        reports.extend(thread::scope(|scope| {
            let handles = batch
                .iter()
                .map(|item| (item, scope.spawn(|| query(item))))
                .collect::<Vec<_>>();
            handles
                .into_iter()
                .map(|(item, handle)| handle.join().unwrap_or_else(|_| worker_panic(item)))
                .collect::<Vec<_>>()
        }));
    }
    reports
}
