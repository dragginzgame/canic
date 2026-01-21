//!
//! Time helpers abstracting over host/IC execution so call sites can request
//! UNIX epoch timestamps at various precisions.
//!

use std::time::SystemTime;

// time_nanos
#[allow(unreachable_code)]
fn time_nanos() -> u128 {
    #[cfg(target_arch = "wasm32")]
    {
        return crate::api::time() as u128;
    }

    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(d) => d.as_nanos(),
        Err(_) => 0,
    }
}

/// Returns the current UNIX epoch time in seconds.
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn now_secs() -> u64 {
    (time_nanos() / 1_000_000_000) as u64
}

/// Returns the current UNIX epoch time in milliseconds.
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn now_millis() -> u64 {
    (time_nanos() / 1_000_000) as u64
}

/// Returns the current UNIX epoch time in microseconds.
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn now_micros() -> u64 {
    (time_nanos() / 1_000) as u64
}

/// Returns the current UNIX epoch time in nanoseconds.
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn now_nanos() -> u64 {
    time_nanos() as u64
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_now_secs_sanity() {
        let now = now_secs();
        let current_year_secs = 1_700_000_000; // ≈ Oct 2023
        assert!(now > current_year_secs);
    }
}
