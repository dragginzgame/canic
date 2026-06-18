//! Module: cdk::utils::time
//!
//! Responsibility: timestamp helpers that work under host and IC execution.
//! Does not own: timer scheduling, lifecycle hooks, or clock policy.
//! Boundary: exposes UNIX epoch timestamps at common precisions.

use std::time::SystemTime;

/// Return the current UNIX epoch time in nanoseconds as the internal base unit.
#[cfg_attr(target_arch = "wasm32", expect(unreachable_code))]
fn time_nanos() -> u128 {
    #[cfg(target_arch = "wasm32")]
    {
        return u128::from(crate::cdk::api::time());
    }

    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(d) => d.as_nanos(),
        Err(_) => 0,
    }
}

/// Returns the current UNIX epoch time in seconds.
#[must_use]
#[expect(clippy::cast_possible_truncation)]
pub fn now_secs() -> u64 {
    (time_nanos() / 1_000_000_000) as u64
}

/// Returns the current UNIX epoch time in milliseconds.
#[must_use]
#[expect(clippy::cast_possible_truncation)]
pub fn now_millis() -> u64 {
    (time_nanos() / 1_000_000) as u64
}

/// Returns the current UNIX epoch time in microseconds.
#[must_use]
#[expect(clippy::cast_possible_truncation)]
pub fn now_micros() -> u64 {
    (time_nanos() / 1_000) as u64
}

/// Returns the current UNIX epoch time in nanoseconds.
#[must_use]
#[expect(clippy::cast_possible_truncation)]
pub fn now_nanos() -> u64 {
    time_nanos() as u64
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_now_secs_sanity() {
        let now = now_secs();
        let current_year_secs = 1_700_000_000; // roughly Oct 2023
        assert!(now > current_year_secs);
    }
}
