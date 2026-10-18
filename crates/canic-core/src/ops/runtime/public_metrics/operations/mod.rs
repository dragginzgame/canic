//! Bounded public operation aggregates from typed owners, without protected DTOs.
//! Units and history kinds are assigned here; names never recover those facts.

use crate::{
    InternalError,
    domain::public_metrics::PublicMetricKind,
    model::public_metrics::{MAX_PUBLIC_METRICS, PublicMetricSample},
    ops::{
        runtime::{
            env::EnvOps,
            metrics::{
                canister_ops::CanisterOpsMetrics, cycles_funding::CyclesFundingMetrics,
                cycles_topup::CyclesTopupMetrics, lifecycle::LifecycleMetrics,
            },
        },
        storage::icp_refill::{
            IcpRefillMetricSnapshot, IcpRefillRecordOps, metric_error_label, metric_error_phase,
            metric_status_label,
        },
    },
};

#[cfg(test)]
mod tests;

const SOURCE_LIMIT: usize = MAX_PUBLIC_METRICS + 1;

pub(super) fn sample() -> Result<Vec<PublicMetricSample>, InternalError> {
    let mut rows = Vec::new();
    for (key, value) in LifecycleMetrics::bounded_snapshot(SOURCE_LIMIT) {
        rows.push(count(
            format!(
                "lifecycle.{}.{}.{}.{}",
                key.phase.metric_label(),
                key.role.metric_label(),
                key.stage.metric_label(),
                key.outcome.metric_label()
            ),
            value,
        ));
    }
    for (key, value) in CanisterOpsMetrics::bounded_snapshot(SOURCE_LIMIT)? {
        rows.push(count(
            format!(
                "canister_ops.{}.{}.{}.{}",
                key.operation.metric_label(),
                key.role,
                key.outcome.metric_label(),
                key.reason.metric_label()
            ),
            value,
        ));
    }
    for (key, reason, value) in CyclesFundingMetrics::bounded_aggregate_snapshot(SOURCE_LIMIT) {
        let name = reason.map_or_else(
            || format!("cycles_funding.{}", key.metric_label()),
            |reason| {
                format!(
                    "cycles_funding.{}.{}",
                    key.metric_label(),
                    reason.metric_label()
                )
            },
        );
        rows.push(scalar(
            name,
            value,
            "cycles",
            PublicMetricKind::Counter {
                window_id: 0,
                saturated: value == u128::MAX,
            },
        ));
    }
    if EnvOps::is_root() {
        rows.extend(refill_rows(&IcpRefillRecordOps::bounded_metric_snapshot(
            SOURCE_LIMIT,
        )));
    }
    for (key, value) in CyclesTopupMetrics::bounded_snapshot(SOURCE_LIMIT) {
        rows.push(count(format!("cycles_topup.{}", key.metric_label()), value));
    }
    Ok(rows)
}

fn refill_rows(snapshot: &IcpRefillMetricSnapshot) -> Vec<PublicMetricSample> {
    // Derived retained-record counts can decrease. Target amounts remain protected.
    let statuses = snapshot.statuses.iter().map(|status| {
        scalar(
            format!(
                "cycles_funding.icp_refill.{}.{}.status.{}",
                status.trigger_label,
                status.phase,
                metric_status_label(status.status)
            ),
            u128::from(status.count),
            "count",
            PublicMetricKind::Gauge,
        )
    });
    let errors = snapshot.errors.iter().map(|error| {
        scalar(
            format!(
                "cycles_funding.icp_refill.{}.error.{}",
                metric_error_phase(error.error_code),
                metric_error_label(error.error_code)
            ),
            u128::from(error.count),
            "count",
            PublicMetricKind::Gauge,
        )
    });
    statuses.chain(errors).collect()
}

fn count(name: String, value: u64) -> PublicMetricSample {
    scalar(
        name,
        u128::from(value),
        "count",
        PublicMetricKind::Counter {
            window_id: 0,
            saturated: value == u64::MAX,
        },
    )
}

fn scalar(name: String, value: u128, unit: &str, kind: PublicMetricKind) -> PublicMetricSample {
    PublicMetricSample {
        name,
        canister_id: None,
        value,
        unit: unit.into(),
        observed_at_ns: 0,
        kind,
    }
}
