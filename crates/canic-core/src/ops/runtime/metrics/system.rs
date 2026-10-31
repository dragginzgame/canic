//! Aggregate inter-canister call count consumed by process metrics.

use ic_metrics::MeasurementCounter;
use std::cell::Cell;

thread_local! {
    static CANISTER_CALLS: Cell<MeasurementCounter> = const { Cell::new(MeasurementCounter::ZERO) };
}

/// Runtime owner of the aggregate inter-canister call counter.
pub struct SystemMetrics;

impl SystemMetrics {
    /// Read the aggregate without visiting target or method identities.
    pub(crate) fn count() -> MeasurementCounter {
        CANISTER_CALLS.get()
    }

    /// Record one inter-canister call with saturating accounting.
    pub fn increment() {
        let mut count = CANISTER_CALLS.get();
        count.add(1);
        CANISTER_CALLS.set(count);
    }

    #[cfg(test)]
    pub fn reset() {
        CANISTER_CALLS.set(MeasurementCounter::ZERO);
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
        assert_eq!(SystemMetrics::count().exact_total(), Ok(0));
        SystemMetrics::increment();
        SystemMetrics::increment();
        assert_eq!(SystemMetrics::count().exact_total(), Ok(2));
        SystemMetrics::reset();
        assert_eq!(SystemMetrics::count().exact_total(), Ok(0));
    }

    #[test]
    fn calls_saturate() {
        CANISTER_CALLS.set(MeasurementCounter::from_parts(u128::MAX, false).unwrap());
        SystemMetrics::increment();
        assert_eq!(SystemMetrics::count().total(), u128::MAX);
        assert!(SystemMetrics::count().total_overflowed());
        SystemMetrics::reset();
    }
}
