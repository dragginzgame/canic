use crate::{
    dto::metrics::endpoint::EndpointHealthView,
    model::metrics::{
        access::AccessMetricKey,
        endpoint::{EndpointAttemptCounts, EndpointResultCounts},
    },
};

use std::collections::{BTreeSet, HashMap};

#[must_use]
pub fn endpoint_health_to_view(
    attempts: impl IntoIterator<Item = (&'static str, EndpointAttemptCounts)>,
    results: impl IntoIterator<Item = (&'static str, EndpointResultCounts)>,
    access: impl IntoIterator<Item = (AccessMetricKey, u64)>,
    exclude_endpoint: Option<&str>,
) -> Vec<EndpointHealthView> {
    // Aggregate denied counts per endpoint
    let mut denied: HashMap<String, u64> = HashMap::new();
    for (key, count) in access {
        denied
            .entry(key.endpoint)
            .and_modify(|v| *v = v.saturating_add(count))
            .or_insert(count);
    }

    // Collect all endpoint labels
    let mut endpoints = BTreeSet::<String>::new();

    let attempts: HashMap<&'static str, EndpointAttemptCounts> = attempts
        .into_iter()
        .inspect(|(ep, _)| {
            endpoints.insert((*ep).to_string());
        })
        .collect();

    let results: HashMap<&'static str, EndpointResultCounts> = results
        .into_iter()
        .inspect(|(ep, _)| {
            endpoints.insert((*ep).to_string());
        })
        .collect();

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
                .map_or((0, 0), |c| (c.attempted, c.completed));

            let (ok, err) = results
                .get(endpoint.as_str())
                .map_or((0, 0), |c| (c.ok, c.err));

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
