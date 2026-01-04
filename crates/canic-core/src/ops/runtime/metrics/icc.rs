use crate::ops::{
    prelude::*,
    runtime::metrics::{
        store::icc::{IccMetricKey as ModelIccMetricKey, IccMetrics},
        system::{SystemMetricKind, record_system_metric},
    },
};

#[derive(Clone, Debug)]
pub struct IccMetricsSnapshot {
    pub entries: Vec<(IccMetricKey, u64)>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct IccMetricKey {
    pub target: Principal,
    pub method: String,
}

#[must_use]
pub fn snapshot() -> IccMetricsSnapshot {
    let entries = IccMetrics::export_raw()
        .into_iter()
        .map(|(key, count)| (key.into(), count))
        .collect();
    IccMetricsSnapshot { entries }
}

/// Record an inter-canister call in system + ICC metrics.
pub fn record_icc_call(canister_id: impl Into<Principal>, method: &str) {
    let canister_id: Principal = canister_id.into();

    record_system_metric(SystemMetricKind::CanisterCall);
    IccMetrics::increment(canister_id, method);
}

impl From<ModelIccMetricKey> for IccMetricKey {
    fn from(key: ModelIccMetricKey) -> Self {
        Self {
            target: key.target,
            method: key.method,
        }
    }
}
