//! Module: support::parallel
//!
//! Responsibility: collect read-only CLI reports with bounded, ordered workers.
//! Does not own: target selection, query policy, or per-target error projection.

#[cfg(test)]
mod tests;

use std::{io, thread};
use thiserror::Error as ThisError;

/// Bound simultaneous report queries independently of Fleet size.
const MAX_REPORT_WORKERS: usize = 8;

///
/// ReportWorkerFailure
///
/// Collector-owned worker failure projected by each diagnostic report owner.
///

#[derive(Debug, ThisError)]
pub enum ReportWorkerFailure {
    #[error("query worker panicked")]
    Panicked,

    #[error("failed to start query worker: {0}")]
    Start(#[source] io::Error),
}

/// Preserve input order and project worker failures at the target's report owner.
pub fn collect_ordered_reports<T: Sync, R: Send>(
    items: &[T],
    query: impl Fn(&T) -> R + Sync,
    worker_failure: impl Fn(&T, ReportWorkerFailure) -> R,
) -> Vec<R> {
    let mut reports = Vec::with_capacity(items.len());
    for batch in items.chunks(MAX_REPORT_WORKERS) {
        reports.extend(thread::scope(|scope| {
            let handles = batch
                .iter()
                .map(|item| {
                    (
                        item,
                        thread::Builder::new().spawn_scoped(scope, || query(item)),
                    )
                })
                .collect::<Vec<_>>();
            handles
                .into_iter()
                .map(|(item, worker)| finish_report(item, worker, &worker_failure))
                .collect::<Vec<_>>()
        }));
    }
    reports
}

fn finish_report<T, R>(
    item: &T,
    worker: io::Result<thread::ScopedJoinHandle<'_, R>>,
    worker_failure: &impl Fn(&T, ReportWorkerFailure) -> R,
) -> R {
    match worker {
        Ok(handle) => handle
            .join()
            .unwrap_or_else(|_| worker_failure(item, ReportWorkerFailure::Panicked)),
        Err(error) => worker_failure(item, ReportWorkerFailure::Start(error)),
    }
}
