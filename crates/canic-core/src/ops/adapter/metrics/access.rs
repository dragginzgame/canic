use crate::{
    dto::metrics::access::{AccessMetricEntry, AccessMetricKind},
    model::metrics::access::{AccessMetricKey, AccessMetricKind as ModelAccessMetricKind},
};

const fn access_metric_kind_to_view(kind: ModelAccessMetricKind) -> AccessMetricKind {
    match kind {
        ModelAccessMetricKind::Auth => AccessMetricKind::Auth,
        ModelAccessMetricKind::Guard => AccessMetricKind::Guard,
        ModelAccessMetricKind::Policy => AccessMetricKind::Policy,
    }
}

#[must_use]
pub fn access_metrics_to_view(
    raw: impl IntoIterator<Item = (AccessMetricKey, u64)>,
) -> Vec<AccessMetricEntry> {
    raw.into_iter()
        .map(|(key, count)| AccessMetricEntry {
            endpoint: key.endpoint,
            kind: access_metric_kind_to_view(key.kind),
            count,
        })
        .collect()
}
