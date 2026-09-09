//! Module: fleet_ensure::ops::bounded_observations
//!
//! Responsibility: collect independent reads with bounded concurrency.
//! Does not own: effects, retries, authority decisions, or observation caching.
//! Boundary: join each issued batch before returning or scheduling more reads.

use std::{panic::resume_unwind, thread};

pub(super) const MAX_IN_FLIGHT: usize = 4;

pub(super) fn collect<T: Sync, U: Send, E: Send>(
    inputs: &[T],
    observe: impl Fn(&T) -> Result<U, E> + Sync,
) -> Result<Vec<U>, E> {
    let mut observations = Vec::with_capacity(inputs.len());
    for batch in inputs.chunks(MAX_IN_FLIGHT) {
        let results = thread::scope(|scope| {
            let workers = batch
                .iter()
                .map(|input| scope.spawn(|| observe(input)))
                .collect::<Vec<_>>();
            // Drain every issued read, even when an earlier worker fails or panics.
            workers
                .into_iter()
                .map(thread::ScopedJoinHandle::join)
                .collect::<Vec<_>>()
        });
        for result in results {
            let observation = match result {
                Ok(observation) => observation?,
                Err(panic) => resume_unwind(panic),
            };
            observations.push(observation);
        }
    }
    Ok(observations)
}

// Tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fleet_ensure::ops::current_protocol::CurrentProtocolError;
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
        let observations = collect::<_, _, CurrentProtocolError>(&inputs, |input| {
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
                0 => Err(CurrentProtocolError::CoordinatorUnavailable),
                1 => Err(CurrentProtocolError::RegistryNotActive),
                _ => Ok(*input),
            }
        });
        assert!(matches!(
            result,
            Err(CurrentProtocolError::CoordinatorUnavailable)
        ));
        assert_eq!(finished.load(Ordering::SeqCst), MAX_IN_FLIGHT);
    }

    #[test]
    fn empty_and_partial_batches_do_not_reuse_previous_observations() {
        let calls = AtomicUsize::new(0);
        let observe = |input: &usize| {
            Ok::<_, CurrentProtocolError>(input + calls.fetch_add(1, Ordering::SeqCst))
        };
        assert!(collect(&[], observe).expect("empty batch").is_empty());
        assert_eq!(collect(&[10], observe).expect("first read"), vec![10]);
        assert_eq!(collect(&[10], observe).expect("fresh read"), vec![11]);
        assert_eq!(
            collect::<_, _, CurrentProtocolError>(&[10, 20, 30], |_| Ok(7)).expect("partial batch"),
            vec![7; 3]
        );
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }
}
