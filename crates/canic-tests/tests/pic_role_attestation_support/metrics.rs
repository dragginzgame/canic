use super::*;

pub fn access_metric_count(
    pic: &Pic,
    canister_id: Principal,
    endpoint: &str,
    predicate: &str,
) -> u64 {
    let response: Result<Page<MetricEntry>, Error> = query_call_as(
        pic,
        canister_id,
        Principal::anonymous(),
        "canic_metrics",
        (
            MetricsKind::Access,
            PageRequest {
                limit: 10_000,
                offset: 0,
            },
        ),
    );
    let response = response.expect("query canic_metrics failed");
    response
        .entries
        .into_iter()
        .find_map(|entry| {
            if entry.labels.first().is_some_and(|label| label == endpoint)
                && entry.labels.get(2).is_some_and(|label| label == predicate)
            {
                Some(match entry.value {
                    MetricValue::Count(count) | MetricValue::CountAndU64 { count, .. } => count,
                    MetricValue::U128(_) => 0,
                })
            } else {
                None
            }
        })
        .unwrap_or(0)
}

// Assert a batch of access-metric predicates for a single canister endpoint.
pub fn assert_access_metrics(
    pic: &Pic,
    canister_id: Principal,
    endpoint: &str,
    expected: &[(&str, u64)],
) {
    for (predicate, count) in expected {
        assert_eq!(
            access_metric_count(pic, canister_id, endpoint, predicate),
            *count,
            "unexpected metric count for {endpoint} / {predicate}"
        );
    }
}
