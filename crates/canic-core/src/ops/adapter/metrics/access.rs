use crate::{dto::metrics::access::AccessMetricEntry, model::metrics::access::AccessMetricKey};

#[must_use]
pub fn access_metrics_to_view(
    raw: impl IntoIterator<Item = (AccessMetricKey, u64)>,
) -> Vec<AccessMetricEntry> {
    raw.into_iter()
        .map(|(key, count)| AccessMetricEntry {
            endpoint: key.endpoint,
            kind: key.kind,
            count,
        })
        .collect()
}
