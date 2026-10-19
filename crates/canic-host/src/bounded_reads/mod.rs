//! Module: bounded_reads
//!
//! Responsibility: drain bounded independent read batches in input order.
//! Does not own: effects, concurrency policy, retries or error interpretation.
//! Boundary: callers select a positive limit and interpret worker panics.
use std::{any::Any, num::NonZeroUsize, thread};

/// Drain each issued batch before interpreting its results or scheduling more.
pub fn collect<T: Sync, U: Send, E: Send>(
    inputs: &[T],
    limit: NonZeroUsize,
    observe: impl Fn(&T) -> Result<U, E> + Sync,
    on_panic: impl Fn(&T, Box<dyn Any + Send>) -> E,
) -> Result<Vec<U>, E> {
    let mut outputs = Vec::with_capacity(inputs.len());
    for batch in inputs.chunks(limit.get()) {
        let results = thread::scope(|scope| {
            batch
                .iter()
                .map(|input| scope.spawn(|| observe(input)))
                .collect::<Vec<_>>()
                .into_iter()
                .map(thread::ScopedJoinHandle::join)
                .collect::<Vec<_>>()
        });
        for (input, result) in batch.iter().zip(results) {
            outputs.push(result.unwrap_or_else(|panic| Err(on_panic(input, panic)))?);
        }
    }
    Ok(outputs)
}

// Tests

#[cfg(test)]
mod tests {
    use super::*;
    const MAX_IN_FLIGHT: usize = 4;
    #[derive(Debug, Eq, PartialEq)]
    enum Failure {
        First,
        Second,
    }
    fn collect<T: Sync, U: Send, E: Send>(
        inputs: &[T],
        observe: impl Fn(&T) -> Result<U, E> + Sync,
    ) -> Result<Vec<U>, E> {
        super::collect(
            inputs,
            NonZeroUsize::new(MAX_IN_FLIGHT).unwrap(),
            observe,
            |_, panic| std::panic::resume_unwind(panic),
        )
    }
    use std::sync::{
        Barrier,
        atomic::{AtomicUsize, Ordering},
    };

    #[test]
    fn overlaps_reads_with_a_fixed_bound_and_preserves_input_order() {
        let barrier = Barrier::new(MAX_IN_FLIGHT);
        let active = AtomicUsize::new(0);
        let maximum = AtomicUsize::new(0);
        let calls = AtomicUsize::new(0);
        let inputs = (0..MAX_IN_FLIGHT * 2).collect::<Vec<_>>();
        let observations = collect::<_, _, Failure>(&inputs, |input| {
            let count = active.fetch_add(1, Ordering::SeqCst) + 1;
            maximum.fetch_max(count, Ordering::SeqCst);
            calls.fetch_add(1, Ordering::SeqCst);
            barrier.wait();
            active.fetch_sub(1, Ordering::SeqCst);
            Ok(input * 10)
        })
        .expect("independent reads complete");
        assert_eq!(maximum.load(Ordering::SeqCst), MAX_IN_FLIGHT);
        assert_eq!(active.load(Ordering::SeqCst), 0);
        assert_eq!(calls.load(Ordering::SeqCst), inputs.len());
        assert_eq!(
            observations,
            inputs.iter().map(|input| input * 10).collect::<Vec<_>>()
        );
    }

    #[test]
    fn drains_failed_batch_returns_first_input_error_and_stops_scheduling() {
        let barrier = Barrier::new(MAX_IN_FLIGHT);
        let finished = AtomicUsize::new(0);
        let inputs = (0..MAX_IN_FLIGHT * 2).collect::<Vec<_>>();
        let result = collect(&inputs, |input| {
            barrier.wait();
            finished.fetch_add(1, Ordering::SeqCst);
            match input {
                0 => Err(Failure::First),
                1 => Err(Failure::Second),
                _ => Ok(*input),
            }
        });
        assert!(matches!(result, Err(Failure::First)));
        assert_eq!(finished.load(Ordering::SeqCst), MAX_IN_FLIGHT);
    }

    #[test]
    fn empty_and_partial_batches_do_not_reuse_previous_observations() {
        let calls = AtomicUsize::new(0);
        let observe =
            |input: &usize| Ok::<_, Failure>(input + calls.fetch_add(1, Ordering::SeqCst));
        assert!(collect(&[], observe).expect("empty batch").is_empty());
        assert_eq!(collect(&[10], observe).expect("first read"), vec![10]);
        assert_eq!(collect(&[10], observe).expect("fresh read"), vec![11]);
        assert_eq!(
            collect::<_, _, Failure>(&[10, 20, 30], |_| Ok(7)).expect("partial batch"),
            vec![7; 3]
        );
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn panic_interpretation_runs_after_every_issued_worker_finishes() {
        let finished = AtomicUsize::new(0);
        let barrier = Barrier::new(4);
        let result = super::collect(
            &[0, 1, 2, 3, 4],
            NonZeroUsize::new(4).unwrap(),
            |input| {
                barrier.wait();
                finished.fetch_add(1, Ordering::SeqCst);
                assert_ne!(*input, 0, "read failed");
                Ok::<_, usize>(*input)
            },
            |input, _| {
                assert_eq!(finished.load(Ordering::SeqCst), 4);
                *input
            },
        );
        assert_eq!(result, Err(0));
        assert_eq!(finished.load(Ordering::SeqCst), 4);
    }
}
