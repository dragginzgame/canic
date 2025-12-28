use serde::{Deserialize, Serialize};
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

#[derive(Default)]
struct EndpointAttemptCounts {
    attempted: u64,
    completed: u64,
}

///
/// EndpointResultCounts
/// Internal ok/err counters for Result-returning endpoints.
///

#[derive(Default)]
struct EndpointResultCounts {
    ok: u64,
    err: u64,
}

// -----------------------------------------------------------------------------
// Public metric DTOs
// -----------------------------------------------------------------------------

///
/// EndpointAttemptMetricEntry
/// Public metric entry for endpoint attempt/completion.
///
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EndpointAttemptMetricEntry {
    pub endpoint: String,
    pub attempted: u64,
    pub completed: u64,
}

///
/// EndpointAttemptMetricsSnapshot
///

pub type EndpointAttemptMetricsSnapshot = Vec<EndpointAttemptMetricEntry>;

///
/// EndpointResultMetricEntry
/// Public metric entry for endpoint ok/err outcomes.
///

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EndpointResultMetricEntry {
    pub endpoint: String,
    pub ok: u64,
    pub err: u64,
}

///
/// EndpointResultMetricsSnapshot
///

pub type EndpointResultMetricsSnapshot = Vec<EndpointResultMetricEntry>;

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
    pub fn snapshot() -> EndpointAttemptMetricsSnapshot {
        ENDPOINT_ATTEMPT_METRICS.with_borrow(|counts| {
            counts
                .iter()
                .map(|(endpoint, c)| EndpointAttemptMetricEntry {
                    endpoint: (*endpoint).to_string(),
                    attempted: c.attempted,
                    completed: c.completed,
                })
                .collect()
        })
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
    pub fn snapshot() -> EndpointResultMetricsSnapshot {
        ENDPOINT_RESULT_METRICS.with_borrow(|counts| {
            counts
                .iter()
                .map(|(endpoint, c)| EndpointResultMetricEntry {
                    endpoint: (*endpoint).to_string(),
                    ok: c.ok,
                    err: c.err,
                })
                .collect()
        })
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
    use std::collections::HashMap;

    #[test]
    fn endpoint_attempt_metrics_track_attempted_and_completed() {
        EndpointAttemptMetrics::reset();

        EndpointAttemptMetrics::increment_attempted("a");
        EndpointAttemptMetrics::increment_attempted("a");
        EndpointAttemptMetrics::increment_attempted("b");
        EndpointAttemptMetrics::increment_completed("a");

        let snapshot = EndpointAttemptMetrics::snapshot();
        let mut map: HashMap<String, (u64, u64)> = snapshot
            .into_iter()
            .map(|e| (e.endpoint, (e.attempted, e.completed)))
            .collect();

        assert_eq!(map.remove("a"), Some((2, 1)));
        assert_eq!(map.remove("b"), Some((1, 0)));
        assert!(map.is_empty());
    }

    #[test]
    fn endpoint_result_metrics_track_ok_and_err() {
        EndpointResultMetrics::reset();

        EndpointResultMetrics::increment_ok("a");
        EndpointResultMetrics::increment_ok("a");
        EndpointResultMetrics::increment_err("a");
        EndpointResultMetrics::increment_err("b");

        let snapshot = EndpointResultMetrics::snapshot();
        let mut map: HashMap<String, (u64, u64)> = snapshot
            .into_iter()
            .map(|e| (e.endpoint, (e.ok, e.err)))
            .collect();

        assert_eq!(map.remove("a"), Some((2, 1)));
        assert_eq!(map.remove("b"), Some((0, 1)));
        assert!(map.is_empty());
    }
}
