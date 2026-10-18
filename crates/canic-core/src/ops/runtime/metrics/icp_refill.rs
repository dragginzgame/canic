//! Module: ops::runtime::metrics::icp_refill
//!
//! Responsibility: record and snapshot low-cardinality runtime metrics for the icp_refill family.
//! Does not own: workflow decisions, persisted records, or endpoint DTOs.
//! Boundary: ops-layer metrics consumed by workflow metrics projection.

use crate::{
    cdk::types::Principal,
    dto::metrics::{MetricEntry, MetricValue},
    ops::storage::icp_refill::{
        IcpRefillMetricSnapshot, IcpRefillRecordOps, metric_error_label, metric_error_phase,
        metric_status_label,
    },
};

///
/// IcpRefillMetrics
///
/// Operations-layer projector for persisted ICP refill records.
///

pub struct IcpRefillMetrics;

impl IcpRefillMetrics {
    #[must_use]
    pub fn entries() -> Vec<MetricEntry> {
        entries_from_snapshot(&IcpRefillRecordOps::metric_snapshot())
    }
}

pub(super) fn entries_from_snapshot(snapshot: &IcpRefillMetricSnapshot) -> Vec<MetricEntry> {
    let mut entries = snapshot
        .statuses
        .iter()
        .map(|status| {
            count_entry(
                vec![
                    "icp_refill".to_string(),
                    status.trigger_label.to_string(),
                    status.phase.to_string(),
                    "status".to_string(),
                    metric_status_label(status.status).to_string(),
                ],
                status.count,
            )
        })
        .collect::<Vec<_>>();
    entries.extend(snapshot.errors.iter().map(|error| {
        count_entry(
            vec![
                "icp_refill".to_string(),
                metric_error_phase(error.error_code).to_string(),
                "error".to_string(),
                metric_error_label(error.error_code).to_string(),
            ],
            error.count,
        )
    }));
    entries.extend(snapshot.targets.iter().map(|target| {
        principal_u128_entry(
            vec![
                "icp_refill".to_string(),
                "transfer".to_string(),
                "amount_e8s".to_string(),
                "target".to_string(),
            ],
            target.target_canister,
            target.amount_e8s,
        )
    }));
    entries.extend(snapshot.targets.iter().filter_map(|target| {
        target.cycles_sent.map(|cycles_sent| {
            principal_u128_entry(
                vec![
                    "icp_refill".to_string(),
                    "notify".to_string(),
                    "cycles_sent".to_string(),
                    "target".to_string(),
                ],
                target.target_canister,
                cycles_sent,
            )
        })
    }));
    entries
}

const fn count_entry(labels: Vec<String>, count: u64) -> MetricEntry {
    MetricEntry {
        labels,
        principal: None,
        value: MetricValue::Count(count),
    }
}

const fn principal_u128_entry(
    labels: Vec<String>,
    principal: Principal,
    value: u128,
) -> MetricEntry {
    MetricEntry {
        labels,
        principal: Some(principal),
        value: MetricValue::U128(value),
    }
}
