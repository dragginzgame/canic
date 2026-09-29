//! Aggregate inter-canister call count consumed by process metrics.

use std::cell::Cell;

thread_local! {
    static CANISTER_CALLS: Cell<u64> = const { Cell::new(0) };
}

/// Runtime owner of the aggregate inter-canister call counter.
pub struct SystemMetrics;

impl SystemMetrics {
    /// Read the aggregate without visiting target or method identities.
    pub(crate) fn count() -> u64 {
        CANISTER_CALLS.get()
    }

    /// Record one inter-canister call with saturating accounting.
    pub fn increment() {
        CANISTER_CALLS.set(CANISTER_CALLS.get().saturating_add(1));
    }

    #[cfg(test)]
    pub fn reset() {
        CANISTER_CALLS.set(0);
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calls_accumulate_and_reset() {
        SystemMetrics::reset();
        assert_eq!(SystemMetrics::count(), 0);
        SystemMetrics::increment();
        SystemMetrics::increment();
        assert_eq!(SystemMetrics::count(), 2);
        SystemMetrics::reset();
        assert_eq!(SystemMetrics::count(), 0);
    }

    #[test]
    fn calls_saturate() {
        CANISTER_CALLS.set(u64::MAX);
        SystemMetrics::increment();
        assert_eq!(SystemMetrics::count(), u64::MAX);
        SystemMetrics::reset();
    }
}
