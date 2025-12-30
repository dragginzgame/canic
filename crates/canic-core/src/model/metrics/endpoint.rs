use std::{cell::RefCell, collections::HashMap};

thread_local! {
    static ENDPOINT_ATTEMPT_METRICS: RefCell<HashMap<&'static str, EndpointAttemptCounts>> =
        RefCell::new(HashMap::new());

    static ENDPOINT_RESULT_METRICS: RefCell<HashMap<&'static str, EndpointResultCounts>> =
        RefCell::new(HashMap::new());
}

// -----------------------------------------------------------------------------
// Internal counter types (private)
// -----------------------------------------------------------------------------

///
/// EndpointAttemptCounts
/// Internal attempt/completion counters.
///

#[derive(Clone, Default)]
pub struct EndpointAttemptCounts {
    pub attempted: u64,
    pub completed: u64,
}

///
/// EndpointResultCounts
/// Internal ok/err counters for Result-returning endpoints.
///

#[derive(Clone, Default)]
pub struct EndpointResultCounts {
    pub ok: u64,
    pub err: u64,
}

// -----------------------------------------------------------------------------
// Metrics state + operations
// -----------------------------------------------------------------------------

///
/// EndpointAttemptMetrics
/// Best-effort attempt/completion counters per endpoint.
///
/// Intended uses:
/// - Derive access-denial rate via attempted vs denied.
/// - Detect suspected traps via attempted vs (denied + completed).
///

pub struct EndpointAttemptMetrics;

impl EndpointAttemptMetrics {
    pub fn increment_attempted(endpoint: &'static str) {
        ENDPOINT_ATTEMPT_METRICS.with_borrow_mut(|counts| {
            let entry = counts.entry(endpoint).or_default();
            entry.attempted = entry.attempted.saturating_add(1);
        });
    }

    pub fn increment_completed(endpoint: &'static str) {
        ENDPOINT_ATTEMPT_METRICS.with_borrow_mut(|counts| {
            let entry = counts.entry(endpoint).or_default();
            entry.completed = entry.completed.saturating_add(1);
        });
    }

    #[must_use]
    pub fn export_raw() -> HashMap<&'static str, EndpointAttemptCounts> {
        ENDPOINT_ATTEMPT_METRICS.with_borrow(std::clone::Clone::clone)
    }

    #[cfg(test)]
    pub fn reset() {
        ENDPOINT_ATTEMPT_METRICS.with_borrow_mut(HashMap::clear);
    }
}

///
/// EndpointResultMetrics
/// Best-effort ok/err counters per endpoint for Result-returning endpoints.
///
/// Notes:
/// - Access-denied errors are excluded (pre-dispatch).
///

pub struct EndpointResultMetrics;

impl EndpointResultMetrics {
    pub fn increment_ok(endpoint: &'static str) {
        ENDPOINT_RESULT_METRICS.with_borrow_mut(|counts| {
            let entry = counts.entry(endpoint).or_default();
            entry.ok = entry.ok.saturating_add(1);
        });
    }

    pub fn increment_err(endpoint: &'static str) {
        ENDPOINT_RESULT_METRICS.with_borrow_mut(|counts| {
            let entry = counts.entry(endpoint).or_default();
            entry.err = entry.err.saturating_add(1);
        });
    }

    #[must_use]
    pub fn export_raw() -> HashMap<&'static str, EndpointResultCounts> {
        ENDPOINT_RESULT_METRICS.with_borrow(std::clone::Clone::clone)
    }

    #[cfg(test)]
    pub fn reset() {
        ENDPOINT_RESULT_METRICS.with_borrow_mut(HashMap::clear);
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;

    const EP_A: &str = "endpoint_a";
    const EP_B: &str = "endpoint_b";

    // -------------------------------------------------------------------------
    // EndpointAttemptMetrics
    // -------------------------------------------------------------------------

    #[test]
    fn attempt_metrics_start_empty() {
        EndpointAttemptMetrics::reset();

        let raw = EndpointAttemptMetrics::export_raw();
        assert!(raw.is_empty());
    }

    #[test]
    fn increment_attempted_increases_attempted_only() {
        EndpointAttemptMetrics::reset();

        EndpointAttemptMetrics::increment_attempted(EP_A);

        let raw = EndpointAttemptMetrics::export_raw();
        let counts = raw.get(EP_A).unwrap();

        assert_eq!(counts.attempted, 1);
        assert_eq!(counts.completed, 0);
    }

    #[test]
    fn increment_completed_increases_completed_only() {
        EndpointAttemptMetrics::reset();

        EndpointAttemptMetrics::increment_completed(EP_A);

        let raw = EndpointAttemptMetrics::export_raw();
        let counts = raw.get(EP_A).unwrap();

        assert_eq!(counts.attempted, 0);
        assert_eq!(counts.completed, 1);
    }

    #[test]
    fn attempt_and_completed_accumulate_independently() {
        EndpointAttemptMetrics::reset();

        EndpointAttemptMetrics::increment_attempted(EP_A);
        EndpointAttemptMetrics::increment_attempted(EP_A);
        EndpointAttemptMetrics::increment_completed(EP_A);

        let raw = EndpointAttemptMetrics::export_raw();
        let counts = raw.get(EP_A).unwrap();

        assert_eq!(counts.attempted, 2);
        assert_eq!(counts.completed, 1);
    }

    #[test]
    fn attempt_metrics_are_per_endpoint() {
        EndpointAttemptMetrics::reset();

        EndpointAttemptMetrics::increment_attempted(EP_A);
        EndpointAttemptMetrics::increment_completed(EP_B);

        let raw = EndpointAttemptMetrics::export_raw();

        let a = raw.get(EP_A).unwrap();
        let b = raw.get(EP_B).unwrap();

        assert_eq!(a.attempted, 1);
        assert_eq!(a.completed, 0);

        assert_eq!(b.attempted, 0);
        assert_eq!(b.completed, 1);
    }

    // -------------------------------------------------------------------------
    // EndpointResultMetrics
    // -------------------------------------------------------------------------

    #[test]
    fn result_metrics_start_empty() {
        EndpointResultMetrics::reset();

        let raw = EndpointResultMetrics::export_raw();
        assert!(raw.is_empty());
    }

    #[test]
    fn increment_ok_increases_ok_only() {
        EndpointResultMetrics::reset();

        EndpointResultMetrics::increment_ok(EP_A);

        let raw = EndpointResultMetrics::export_raw();
        let counts = raw.get(EP_A).unwrap();

        assert_eq!(counts.ok, 1);
        assert_eq!(counts.err, 0);
    }

    #[test]
    fn increment_err_increases_err_only() {
        EndpointResultMetrics::reset();

        EndpointResultMetrics::increment_err(EP_A);

        let raw = EndpointResultMetrics::export_raw();
        let counts = raw.get(EP_A).unwrap();

        assert_eq!(counts.ok, 0);
        assert_eq!(counts.err, 1);
    }

    #[test]
    fn ok_and_err_accumulate_independently() {
        EndpointResultMetrics::reset();

        EndpointResultMetrics::increment_ok(EP_A);
        EndpointResultMetrics::increment_ok(EP_A);
        EndpointResultMetrics::increment_err(EP_A);

        let raw = EndpointResultMetrics::export_raw();
        let counts = raw.get(EP_A).unwrap();

        assert_eq!(counts.ok, 2);
        assert_eq!(counts.err, 1);
    }

    #[test]
    fn result_metrics_are_per_endpoint() {
        EndpointResultMetrics::reset();

        EndpointResultMetrics::increment_ok(EP_A);
        EndpointResultMetrics::increment_err(EP_B);

        let raw = EndpointResultMetrics::export_raw();

        let a = raw.get(EP_A).unwrap();
        let b = raw.get(EP_B).unwrap();

        assert_eq!(a.ok, 1);
        assert_eq!(a.err, 0);

        assert_eq!(b.ok, 0);
        assert_eq!(b.err, 1);
    }
}
