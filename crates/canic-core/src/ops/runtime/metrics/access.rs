use crate::ops::runtime::metrics::store::access::{
    AccessMetricKey as ModelAccessMetricKey, AccessMetricKind as ModelAccessMetricKind,
    AccessMetrics as ModelAccessMetrics,
};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct AccessMetricKey {
    pub endpoint: String,
    pub kind: AccessMetricKind,
}

#[derive(Clone, Debug)]
pub struct AccessMetricsSnapshot {
    pub entries: Vec<(AccessMetricKey, u64)>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AccessMetricKind {
    Auth,
    Guard,
    Rule,
}

pub struct AccessMetrics;

impl AccessMetrics {
    pub fn increment(endpoint: &str, kind: AccessMetricKind) {
        ModelAccessMetrics::increment(endpoint, kind_to_model(kind));
    }
}

#[must_use]
pub fn snapshot() -> AccessMetricsSnapshot {
    let entries = ModelAccessMetrics::export_raw()
        .into_iter()
        .map(|(key, count)| (key.into(), count))
        .collect();
    AccessMetricsSnapshot { entries }
}

const fn kind_to_model(kind: AccessMetricKind) -> ModelAccessMetricKind {
    match kind {
        AccessMetricKind::Auth => ModelAccessMetricKind::Auth,
        AccessMetricKind::Guard => ModelAccessMetricKind::Guard,
        AccessMetricKind::Rule => ModelAccessMetricKind::Rule,
    }
}

const fn kind_from_model(kind: ModelAccessMetricKind) -> AccessMetricKind {
    match kind {
        ModelAccessMetricKind::Auth => AccessMetricKind::Auth,
        ModelAccessMetricKind::Guard => AccessMetricKind::Guard,
        ModelAccessMetricKind::Rule => AccessMetricKind::Rule,
    }
}

impl From<ModelAccessMetricKey> for AccessMetricKey {
    fn from(key: ModelAccessMetricKey) -> Self {
        Self {
            endpoint: key.endpoint,
            kind: kind_from_model(key.kind),
        }
    }
}
