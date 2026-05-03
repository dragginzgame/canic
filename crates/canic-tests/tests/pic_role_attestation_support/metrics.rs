use super::*;

// Query one metric page from a canister through the public metrics endpoint.
pub fn query_metric_page(
    pic: &Pic,
    canister_id: Principal,
    kind: MetricsKind,
) -> Page<MetricEntry> {
    let response: Result<Result<Page<MetricEntry>, Error>, Error> = pic.query_call_as(
        canister_id,
        Principal::anonymous(),
        "canic_metrics",
        (
            kind,
            PageRequest {
                limit: 10_000,
                offset: 0,
            },
        ),
    );

    response
        .expect("metrics transport query failed")
        .expect("metrics application query failed")
}

// Return all metric rows for a metric family using a large deterministic page.
pub fn query_metric_entries(
    pic: &Pic,
    canister_id: Principal,
    kind: MetricsKind,
) -> Vec<MetricEntry> {
    query_metric_page(pic, canister_id, kind).entries
}

// Read a count-shaped metric row by its exact label set.
pub fn metric_count_for_labels(
    pic: &Pic,
    canister_id: Principal,
    kind: MetricsKind,
    labels: &[&str],
) -> u64 {
    query_metric_entries(pic, canister_id, kind)
        .into_iter()
        .find_map(|entry| {
            if labels_match(&entry, labels) {
                Some(metric_count(&entry.value))
            } else {
                None
            }
        })
        .unwrap_or(0)
}

// Match a metric entry against an exact static label set.
fn labels_match(entry: &MetricEntry, labels: &[&str]) -> bool {
    entry.labels.len() == labels.len()
        && entry
            .labels
            .iter()
            .zip(labels.iter())
            .all(|(actual, expected)| actual == expected)
}

// Extract the count dimension from count-bearing metric payloads.
const fn metric_count(value: &MetricValue) -> u64 {
    match value {
        MetricValue::Count(count) | MetricValue::CountAndU64 { count, .. } => *count,
        MetricValue::U128(_) => 0,
    }
}
