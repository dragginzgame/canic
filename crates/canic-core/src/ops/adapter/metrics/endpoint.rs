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
