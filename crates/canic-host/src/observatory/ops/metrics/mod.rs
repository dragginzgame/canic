//! Bound cached metric pages and preserve source identities, times and units.

#[cfg(test)]
mod tests;

use crate::observatory::view::*;
use canic_core::dto::public_status::{
    PublicMetricFamily, PublicMetricKind, PublicMetricsSnapshot, PublicSnapshotState,
};
use std::collections::BTreeSet;

pub(super) fn samples(
    snapshot: PublicMetricsSnapshot,
    family: PublicMetricFamily,
) -> Result<MetricSamplesView, ObservationFailure> {
    if snapshot.family != family
        || snapshot.metrics.entries.len() > 256
        || snapshot.metrics.total < snapshot.metrics.entries.len() as u64
    {
        return Err(ObservationFailure::InvalidResponse);
    }
    let mut identities = BTreeSet::new();
    for row in &snapshot.metrics.entries {
        let valid_text = [&row.name, &row.unit].into_iter().all(|text| {
            !text.is_empty() && text.len() <= 128 && !text.chars().any(char::is_control)
        });
        if !valid_text
            || !identities.insert((&row.name, row.canister_id))
            || snapshot
                .sampled_at_ns
                .is_none_or(|at| row.observed_at_ns < at)
        {
            return Err(ObservationFailure::InvalidResponse);
        }
    }
    let truncated =
        snapshot.truncated || snapshot.metrics.total > snapshot.metrics.entries.len() as u64;
    let rows = snapshot
        .metrics
        .entries
        .into_iter()
        .filter(|row| match family {
            PublicMetricFamily::Cycles => row.name == "balance",
            PublicMetricFamily::Operations => {
                row.name.starts_with("timer.events.")
                    || row.name == "cycles_funding.cycles_granted_total"
                    || row.name == "cycles_funding.cycles_granted_to_child"
            }
            PublicMetricFamily::Performance => row.name.starts_with("perf.timer."),
            PublicMetricFamily::Application => true,
            PublicMetricFamily::ShardOccupancy => false,
        })
        .map(|row| MetricView {
            name: row.name,
            canister_id: row.canister_id.map(|id| id.to_text()),
            value: row.value.to_string(),
            unit: row.unit,
            observed_at_ns: row.observed_at_ns,
            measurement: match row.kind {
                PublicMetricKind::Gauge => MetricKind::Gauge,
                PublicMetricKind::Counter {
                    window_id,
                    saturated,
                } => MetricKind::Counter {
                    window_id,
                    saturated,
                },
                PublicMetricKind::TimerCounter {
                    registration,
                    saturated,
                } => MetricKind::TimerCounter {
                    registration: TimerRegistrationView {
                        canister_version: registration.canister_version,
                        started_at_ns: registration.started_at_ns,
                        sequence: registration.sequence,
                    },
                    saturated,
                },
            },
        })
        .collect();
    Ok(MetricSamplesView {
        state: match snapshot.state {
            PublicSnapshotState::Disabled => MetricSampleState::Disabled,
            PublicSnapshotState::Unavailable => MetricSampleState::Unavailable,
            PublicSnapshotState::Fresh => MetricSampleState::Fresh,
            PublicSnapshotState::Stale => MetricSampleState::Stale,
        },
        sampled_at_ns: snapshot.sampled_at_ns,
        stale_after_ns: snapshot.stale_after_ns,
        truncated,
        rows,
    })
}
