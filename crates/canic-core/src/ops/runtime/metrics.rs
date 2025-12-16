pub use crate::model::metrics::{access::*, endpoint::*, http::*, icc::*, system::*, timer::*};
use crate::{
    dto::Page,
    perf::{PerfKey, entries as perf_entries},
    types::PageRequest,
};
use candid::CandidType;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};

///
/// MetricsOps
/// Thin ops-layer facade over volatile metrics state.
///

pub struct MetricsOps;

///
/// EndpointHealthEntry
/// Derived endpoint-level health view joined at read time.
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct EndpointHealthEntry {
    pub endpoint: String,
    pub attempted: u64,
    pub denied: u64,
    pub completed: u64,
    pub ok: u64,
    pub err: u64,
    pub avg_instr: u64,
    pub total_instr: u64,
}

impl MetricsOps {
    /// Export the current metrics snapshot.
    #[must_use]
    pub fn system_snapshot() -> SystemMetricsSnapshot {
        let mut entries = SystemMetrics::snapshot();
        entries.sort_by(|a, b| a.kind.cmp(&b.kind));
        entries
    }

    /// Export the current HTTP metrics snapshot.
    #[must_use]
    pub fn http_snapshot() -> HttpMetricsSnapshot {
        HttpMetrics::snapshot()
    }

    /// Export the current HTTP metrics snapshot as a stable, paged view.
    #[must_use]
    pub fn http_page(request: PageRequest) -> Page<HttpMetricEntry> {
        let mut entries = Self::http_snapshot();
        entries.sort_by(|a, b| a.method.cmp(&b.method).then_with(|| a.url.cmp(&b.url)));
        paginate(entries, request)
    }

    /// Export the current ICC metrics snapshot.
    #[must_use]
    pub fn icc_snapshot() -> IccMetricsSnapshot {
        IccMetrics::snapshot()
    }

    /// Export the current ICC metrics snapshot as a stable, paged view.
    #[must_use]
    pub fn icc_page(request: PageRequest) -> Page<IccMetricEntry> {
        let mut entries = Self::icc_snapshot();
        entries.sort_by(|a, b| {
            a.target
                .as_slice()
                .cmp(b.target.as_slice())
                .then_with(|| a.method.cmp(&b.method))
        });
        paginate(entries, request)
    }

    /// Export the current timer metrics snapshot.
    #[must_use]
    pub fn timer_snapshot() -> TimerMetricsSnapshot {
        TimerMetrics::snapshot()
    }

    /// Export the current timer metrics snapshot as a stable, paged view.
    #[must_use]
    pub fn timer_page(request: PageRequest) -> Page<TimerMetricEntry> {
        let mut entries = Self::timer_snapshot();
        entries.sort_by(|a, b| {
            a.mode
                .cmp(&b.mode)
                .then_with(|| a.delay_ms.cmp(&b.delay_ms))
                .then_with(|| a.label.cmp(&b.label))
        });
        paginate(entries, request)
    }

    /// Export the current access metrics snapshot.
    #[must_use]
    pub fn access_snapshot() -> AccessMetricsSnapshot {
        AccessMetrics::snapshot()
    }

    /// Export the current access metrics snapshot as a stable, paged view.
    #[must_use]
    pub fn access_page(request: PageRequest) -> Page<AccessMetricEntry> {
        let mut entries = Self::access_snapshot();
        entries.sort_by(|a, b| {
            a.endpoint
                .cmp(&b.endpoint)
                .then_with(|| a.kind.cmp(&b.kind))
        });
        paginate(entries, request)
    }

    /// Derived endpoint health view (attempts + denials + results + perf).
    #[must_use]
    pub fn endpoint_health_page(request: PageRequest) -> Page<EndpointHealthEntry> {
        Self::endpoint_health_page_excluding(request, None)
    }

    /// Derived endpoint health view (attempts + denials + results + perf), optionally excluding an
    /// endpoint label (useful to avoid self-observation artifacts for the view endpoint itself).
    #[must_use]
    pub fn endpoint_health_page_excluding(
        request: PageRequest,
        exclude_endpoint: Option<&str>,
    ) -> Page<EndpointHealthEntry> {
        let attempt_snapshot = EndpointAttemptMetrics::snapshot();
        let result_snapshot = EndpointResultMetrics::snapshot();
        let access_snapshot = AccessMetrics::snapshot();
        let perf_snapshot = perf_endpoint_snapshot();

        let mut attempts: HashMap<String, (u64, u64)> = HashMap::new();
        for entry in attempt_snapshot {
            attempts.insert(entry.endpoint, (entry.attempted, entry.completed));
        }

        let mut results: HashMap<String, (u64, u64)> = HashMap::new();
        for entry in result_snapshot {
            results.insert(entry.endpoint, (entry.ok, entry.err));
        }

        let mut denied: HashMap<String, u64> = HashMap::new();
        for entry in access_snapshot {
            let counter = denied.entry(entry.endpoint).or_insert(0);
            *counter = counter.saturating_add(entry.count);
        }

        let mut endpoints = BTreeSet::<String>::new();
        endpoints.extend(attempts.keys().cloned());
        endpoints.extend(results.keys().cloned());
        endpoints.extend(denied.keys().cloned());
        endpoints.extend(perf_snapshot.keys().cloned());

        let entries = endpoints
            .into_iter()
            .filter(|endpoint| match exclude_endpoint {
                Some(excluded) => endpoint != excluded,
                None => true,
            })
            .map(|endpoint| {
                let (attempted, completed) = attempts.get(&endpoint).copied().unwrap_or((0, 0));

                // Aggregated access denials (auth + policy), per endpoint.
                let denied = denied.get(&endpoint).copied().unwrap_or(0);
                let (ok, err) = results.get(&endpoint).copied().unwrap_or((0, 0));

                let (perf_count, total_instr) =
                    perf_snapshot.get(&endpoint).copied().unwrap_or((0, 0));
                let avg_instr = if perf_count == 0 {
                    0
                } else {
                    total_instr / perf_count
                };

                EndpointHealthEntry {
                    endpoint,
                    attempted,
                    denied,
                    completed,
                    ok,
                    err,
                    avg_instr,
                    total_instr,
                }
            })
            .collect::<Vec<_>>();

        paginate(entries, request)
    }
}

// -----------------------------------------------------------------------------
// Pagination
// -----------------------------------------------------------------------------

#[must_use]
fn paginate<T>(entries: Vec<T>, request: PageRequest) -> Page<T> {
    let request = request.clamped();
    let total = entries.len() as u64;
    let (start, end) = pagination_bounds(total, request);

    let entries = entries.into_iter().skip(start).take(end - start).collect();

    Page { entries, total }
}

#[allow(clippy::cast_possible_truncation)]
fn pagination_bounds(total: u64, request: PageRequest) -> (usize, usize) {
    let start = request.offset.min(total);
    let end = request.offset.saturating_add(request.limit).min(total);

    let start = start as usize;
    let end = end as usize;

    (start, end)
}

// -----------------------------------------------------------------------------
// Joins
// -----------------------------------------------------------------------------

/// perf_endpoint_snapshot
///
/// NOTE:
/// If perf_entries() ever returns multiple entries per endpoint (e.g.:
/// multiple call sites
/// multiple timers
/// future instrumentation changes),
/// you will silently overwrite earlier values.
#[must_use]
fn perf_endpoint_snapshot() -> HashMap<String, (u64, u64)> {
    let mut out = HashMap::<String, (u64, u64)>::new();

    for entry in perf_entries() {
        let PerfKey::Endpoint(label) = &entry.key else {
            continue;
        };

        let slot = out.entry(label.clone()).or_insert((0, 0));
        slot.0 = slot.0.saturating_add(entry.count);
        slot.1 = slot.1.saturating_add(entry.total_instructions);
    }

    out
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{perf, types::PageRequest};

    #[test]
    fn endpoint_health_joins_tables() {
        EndpointAttemptMetrics::reset();
        EndpointResultMetrics::reset();
        AccessMetrics::reset();
        perf::reset();

        EndpointAttemptMetrics::increment_attempted("a");
        EndpointAttemptMetrics::increment_attempted("a");
        EndpointAttemptMetrics::increment_completed("a");
        EndpointResultMetrics::increment_ok("a");
        perf::record_endpoint("a", 1_000);

        EndpointAttemptMetrics::increment_attempted("b");
        AccessMetrics::increment("b", AccessMetricKind::Auth);

        let page = MetricsOps::endpoint_health_page(PageRequest::new(10, 0));
        assert_eq!(page.total, 2);

        let a = &page.entries[0];
        assert_eq!(a.endpoint, "a");
        assert_eq!(a.attempted, 2);
        assert_eq!(a.denied, 0);
        assert_eq!(a.completed, 1);
        assert_eq!(a.ok, 1);
        assert_eq!(a.err, 0);
        assert_eq!(a.total_instr, 1_000);
        assert_eq!(a.avg_instr, 1_000);

        let b = &page.entries[1];
        assert_eq!(b.endpoint, "b");
        assert_eq!(b.attempted, 1);
        assert_eq!(b.denied, 1);
        assert_eq!(b.completed, 0);
        assert_eq!(b.ok, 0);
        assert_eq!(b.err, 0);
        assert_eq!(b.total_instr, 0);
        assert_eq!(b.avg_instr, 0);
    }
}
