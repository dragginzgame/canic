use super::*;
use std::sync::{
    Barrier,
    atomic::{AtomicUsize, Ordering},
};

#[test]
fn report_collection_bounds_queries_and_preserves_input_order() {
    let inputs = (0..MAX_REPORT_WORKERS * 3).collect::<Vec<_>>();
    let barrier = Barrier::new(MAX_REPORT_WORKERS);
    let active = AtomicUsize::new(0);
    let peak = AtomicUsize::new(0);
    let reports = collect_ordered_reports(
        &inputs,
        |item| {
            let count = active.fetch_add(1, Ordering::SeqCst) + 1;
            peak.fetch_max(count, Ordering::SeqCst);
            barrier.wait();
            active.fetch_sub(1, Ordering::SeqCst);
            item * 2
        },
        |_, _| panic!("query must complete"),
    );
    assert_eq!(
        reports,
        inputs.iter().map(|item| item * 2).collect::<Vec<_>>()
    );
    assert_eq!(peak.load(Ordering::SeqCst), MAX_REPORT_WORKERS);
    assert_eq!(active.load(Ordering::SeqCst), 0);
}

#[test]
fn report_collection_preserves_query_failures_and_isolates_panics() {
    let reports = collect_ordered_reports(
        &[0, 1, 2],
        |item| match item {
            0 => Ok(*item),
            1 => panic!("worker failure"),
            _ => Err(*item),
        },
        |_, failure| {
            std::assert_matches!(failure, ReportWorkerFailure::Panicked);
            Err(usize::MAX)
        },
    );
    assert_eq!(reports, [Ok(0), Err(usize::MAX), Err(2)]);
    let empty = collect_ordered_reports::<usize, usize>(
        &[],
        |_| panic!("empty query"),
        |_, _| panic!("empty fallback"),
    );
    assert!(empty.is_empty());
}

#[test]
fn worker_start_failure_preserves_other_reports_and_target_order() {
    #[derive(Debug, Eq, PartialEq)]
    enum ReportError {
        Panicked,
        Query,
        Start(io::ErrorKind),
    }

    let reports = thread::scope(|scope| {
        let workers = [
            (0, thread::Builder::new().spawn_scoped(scope, || Ok(10))),
            (1, Err(io::Error::from(io::ErrorKind::WouldBlock))),
            (
                2,
                thread::Builder::new().spawn_scoped(scope, || Err(ReportError::Query)),
            ),
            (
                3,
                thread::Builder::new().spawn_scoped(scope, || panic!("worker failure")),
            ),
            (4, thread::Builder::new().spawn_scoped(scope, || Ok(14))),
        ];
        workers
            .into_iter()
            .map(|(item, worker)| {
                let report = finish_report(&item, worker, &|_, failure| match failure {
                    ReportWorkerFailure::Panicked => Err(ReportError::Panicked),
                    ReportWorkerFailure::Start(error) => Err(ReportError::Start(error.kind())),
                });
                (item, report)
            })
            .collect::<Vec<_>>()
    });

    assert_eq!(
        reports,
        [
            (0, Ok(10)),
            (1, Err(ReportError::Start(io::ErrorKind::WouldBlock))),
            (2, Err(ReportError::Query)),
            (3, Err(ReportError::Panicked)),
            (4, Ok(14)),
        ]
    );
}
