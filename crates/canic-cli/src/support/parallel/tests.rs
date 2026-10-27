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
        |_| panic!("query must complete"),
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
        |_| Err(usize::MAX),
    );
    assert_eq!(reports, [Ok(0), Err(usize::MAX), Err(2)]);
    let empty = collect_ordered_reports::<usize, usize>(
        &[],
        |_| panic!("empty query"),
        |_| panic!("empty fallback"),
    );
    assert!(empty.is_empty());
}
