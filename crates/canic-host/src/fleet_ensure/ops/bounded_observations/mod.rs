//! Module: fleet_ensure::ops::bounded_observations
//!
//! Responsibility: collect independent reads with bounded concurrency.
//! Does not own: effects, retries, authority decisions, or observation caching.
//! Boundary: join each issued batch before returning or scheduling more reads.

use std::panic::resume_unwind;

pub(super) const MAX_IN_FLIGHT: usize = 4;

pub(super) fn collect<T: Sync, U: Send, E: Send>(
    inputs: &[T],
    observe: impl Fn(&T) -> Result<U, E> + Sync,
) -> Result<Vec<U>, E> {
    crate::bounded_reads::collect(
        inputs,
        std::num::NonZeroUsize::new(MAX_IN_FLIGHT).expect("nonzero read concurrency"),
        observe,
        |_, panic| resume_unwind(panic),
    )
}
