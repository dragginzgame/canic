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
        ENDPOINT_ATTEMPT_METRICS.with_borrow(|counts| counts.clone())
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
        ENDPOINT_RESULT_METRICS.with_borrow(|counts| counts.clone())
    }

    #[cfg(test)]
    pub fn reset() {
        ENDPOINT_RESULT_METRICS.with_borrow_mut(HashMap::clear);
    }
}
