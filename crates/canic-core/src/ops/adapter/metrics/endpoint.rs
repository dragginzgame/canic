use crate::{
    dto::metrics::endpoint::{
        EndpointAttemptMetricEntry, EndpointHealthView, EndpointResultMetricEntry,
    },
    model::metrics::{
        access::AccessMetricKey,
        endpoint::{EndpointAttemptCounts, EndpointResultCounts},
    },
};

#[must_use]
pub fn endpoint_attempt_metrics_to_view(
    raw: impl IntoIterator<Item = (&'static str, EndpointAttemptCounts)>,
) -> Vec<EndpointAttemptMetricEntry> {
    raw.into_iter()
        .map(|(endpoint, c)| EndpointAttemptMetricEntry {
            endpoint: endpoint.to_string(),
            attempted: c.attempted,
            completed: c.completed,
        })
        .collect()
}

#[must_use]
pub fn endpoint_result_metrics_to_view(
    raw: impl IntoIterator<Item = (&'static str, EndpointResultCounts)>,
) -> Vec<EndpointResultMetricEntry> {
    raw.into_iter()
        .map(|(endpoint, c)| EndpointResultMetricEntry {
            endpoint: endpoint.to_string(),
            ok: c.ok,
            err: c.err,
        })
        .collect()
}

use std::collections::{BTreeSet, HashMap};

#[must_use]
pub fn endpoint_health_to_view(
    attempts: HashMap<&'static str, EndpointAttemptCounts>,
    results: HashMap<&'static str, EndpointResultCounts>,
    access: HashMap<AccessMetricKey, u64>,
    exclude_endpoint: Option<&str>,
) -> Vec<EndpointHealthView> {
    // Aggregate denied counts per endpoint
    let mut denied: HashMap<String, u64> = HashMap::new();
    for (key, count) in access {
        let entry = denied.entry(key.endpoint).or_insert(0);
        *entry = entry.saturating_add(count);
    }

    // Collect all endpoint labels
    let mut endpoints = BTreeSet::<String>::new();
    endpoints.extend(attempts.keys().map(|s| (*s).to_string()));
    endpoints.extend(results.keys().map(|s| (*s).to_string()));
    endpoints.extend(denied.keys().cloned());

    endpoints
        .into_iter()
        .filter(|endpoint| match exclude_endpoint {
            Some(excluded) => endpoint != excluded,
            None => true,
        })
        .map(|endpoint| {
            let (attempted, completed) = attempts
                .get(endpoint.as_str())
                .map(|c| (c.attempted, c.completed))
                .unwrap_or((0, 0));

            let (ok, err) = results
                .get(endpoint.as_str())
                .map(|c| (c.ok, c.err))
                .unwrap_or((0, 0));

            let denied = denied.get(&endpoint).copied().unwrap_or(0);

            EndpointHealthView {
                endpoint,
                attempted,
                denied,
                completed,
                ok,
                err,
            }
        })
        .collect()
}
