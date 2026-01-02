use crate::{
    cdk::candid::Principal,
    storage::metrics::{
        icc::{IccMetricKey, IccMetrics},
        system::{SystemMetricKind, SystemMetrics},
    },
};

#[derive(Clone, Debug)]
pub struct IccMetricsSnapshot {
    pub entries: Vec<(IccMetricKey, u64)>,
}

#[must_use]
pub fn snapshot() -> IccMetricsSnapshot {
    let entries = IccMetrics::export_raw().into_iter().collect();
    IccMetricsSnapshot { entries }
}

/// Record an inter-canister call in system + ICC metrics.
pub fn record_icc_call(canister_id: impl Into<Principal>, method: &str) {
    let canister_id: Principal = canister_id.into();

    SystemMetrics::increment(SystemMetricKind::CanisterCall);
    IccMetrics::increment(canister_id, method);
}
