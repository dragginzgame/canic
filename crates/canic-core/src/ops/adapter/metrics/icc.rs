use crate::{dto::metrics::icc::IccMetricEntry, model::metrics::icc::IccMetricKey};

#[must_use]
pub fn icc_metrics_to_view(
    raw: impl IntoIterator<Item = (IccMetricKey, u64)>,
) -> Vec<IccMetricEntry> {
    raw.into_iter()
        .map(|(key, count)| IccMetricEntry {
            target: key.target,
            method: key.method,
            count,
        })
        .collect()
}
