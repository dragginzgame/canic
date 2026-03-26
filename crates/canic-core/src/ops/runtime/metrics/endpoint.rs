use std::{cell::RefCell, collections::HashMap};

thread_local! {
    static ENDPOINT_METRICS: RefCell<HashMap<&'static str, EndpointMetricCounts>> =
        RefCell::new(HashMap::new());
}

///
/// EndpointMetricCounts
///

#[derive(Clone, Default)]
struct EndpointMetricCounts {
    pub attempted: u64,
    pub completed: u64,
    pub ok: u64,
    pub err: u64,
}

///
/// EndpointAttemptMetrics
///
/// Best-effort attempt/completion counters per endpoint.
///

pub struct EndpointAttemptMetrics;

impl EndpointAttemptMetrics {
    pub fn increment_attempted(endpoint: &'static str) {
        ENDPOINT_METRICS.with_borrow_mut(|counts| {
            let entry = counts.entry(endpoint).or_default();
            entry.attempted = entry.attempted.saturating_add(1);
        });
    }

    pub fn increment_completed(endpoint: &'static str) {
        ENDPOINT_METRICS.with_borrow_mut(|counts| {
            let entry = counts.entry(endpoint).or_default();
            entry.completed = entry.completed.saturating_add(1);
        });
    }

    #[cfg(test)]
    #[must_use]
    pub fn export_raw() -> HashMap<&'static str, EndpointAttemptCounts> {
        ENDPOINT_METRICS.with_borrow(|counts| {
            counts
                .iter()
                .map(|(&endpoint, entry)| {
                    (
                        endpoint,
                        EndpointAttemptCounts {
                            attempted: entry.attempted,
                            completed: entry.completed,
                        },
                    )
                })
                .collect()
        })
    }

    #[cfg(test)]
    pub fn reset() {
        ENDPOINT_METRICS.with_borrow_mut(HashMap::clear);
    }
}

///
/// EndpointResultMetrics
///
/// Best-effort ok/err counters per endpoint for Result-returning endpoints.
///

pub struct EndpointResultMetrics;

impl EndpointResultMetrics {
    pub fn increment_ok(endpoint: &'static str) {
        ENDPOINT_METRICS.with_borrow_mut(|counts| {
            let entry = counts.entry(endpoint).or_default();
            entry.ok = entry.ok.saturating_add(1);
        });
    }

    pub fn increment_err(endpoint: &'static str) {
        ENDPOINT_METRICS.with_borrow_mut(|counts| {
            let entry = counts.entry(endpoint).or_default();
            entry.err = entry.err.saturating_add(1);
        });
    }

    #[cfg(test)]
    #[must_use]
    pub fn export_raw() -> HashMap<&'static str, EndpointResultCounts> {
        ENDPOINT_METRICS.with_borrow(|counts| {
            counts
                .iter()
                .map(|(&endpoint, entry)| {
                    (
                        endpoint,
                        EndpointResultCounts {
                            ok: entry.ok,
                            err: entry.err,
                        },
                    )
                })
                .collect()
        })
    }

    #[cfg(test)]
    pub fn reset() {
        ENDPOINT_METRICS.with_borrow_mut(HashMap::clear);
    }
}

///
/// EndpointAttemptCounts
///

#[cfg(test)]
#[derive(Clone, Default)]
pub struct EndpointAttemptCounts {
    pub attempted: u64,
    pub completed: u64,
}

///
/// EndpointResultCounts
///

#[cfg(test)]
#[derive(Clone, Default)]
pub struct EndpointResultCounts {
    pub ok: u64,
    pub err: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    const EP_A: &str = "endpoint_a";
    const EP_B: &str = "endpoint_b";

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

    #[test]
    fn result_metrics_start_empty() {
        EndpointResultMetrics::reset();
        let raw = EndpointResultMetrics::export_raw();
        assert!(raw.is_empty());
    }

    #[test]
    fn increment_ok_and_err_are_tracked_per_endpoint() {
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
