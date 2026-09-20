//! Module: observatory::ops::cost
//!
//! Responsibility: bounded projection of existing metric owners into private cost evidence.
//! Boundary: read cached measurements; never schedule work or infer cycle consumption.

#[cfg(test)]
mod tests;

use crate::{
    observatory::{
        ops::{outcome, transport::ObservatoryTransport},
        view::*,
    },
    registry::RegistryEntry,
};
use canic_core::dto::public_status::{
    PublicMetricFamily, PublicMetricKind, PublicMetricsSnapshot, PublicSnapshotState,
};
use std::collections::BTreeSet;

pub(super) fn collect(
    entry: &RegistryEntry,
    transport: &mut impl ObservatoryTransport,
) -> RoleCostEvidenceView {
    let source = ObservationSource::PublicMetricCache;
    let mut evidence = RoleCostEvidenceView {
        balance: outcome(
            transport.cost_samples(entry, PublicMetricFamily::Cycles),
            source,
        ),
        funding_and_callbacks: outcome(
            transport.cost_samples(entry, PublicMetricFamily::Operations),
            source,
        ),
        timer_instructions: outcome(
            transport.cost_samples(entry, PublicMetricFamily::Performance),
            source,
        ),
        // Read after the families: a restart between reads invalidates samples older than this heap.
        window: outcome(transport.cost_window(entry), source),
        limitations: vec![
            CostEvidenceLimitation::SingleSnapshot,
            CostEvidenceLimitation::TimerRegistrationResetUnobservable,
            CostEvidenceLimitation::TransferCoverageIncomplete,
            CostEvidenceLimitation::UnattributedExecutionMessageStorage,
        ],
    };
    check_window(&mut evidence);
    evidence
}

fn check_window(evidence: &mut RoleCostEvidenceView) {
    let Observation::Observed {
        value:
            CostWindowView {
                heap_started_at_ns: Some(start),
                ..
            },
        ..
    } = &evidence.window
    else {
        evidence
            .limitations
            .push(CostEvidenceLimitation::SourceWindowUnavailable);
        return;
    };
    let changed = [
        &evidence.balance,
        &evidence.funding_and_callbacks,
        &evidence.timer_instructions,
    ]
    .into_iter()
    .any(|observation| match observation {
        Observation::Observed { value, .. } => {
            value.sampled_at_ns.is_some_and(|at| at < *start)
                || value.rows.iter().any(|row| row.observed_at_ns < *start)
        }
        Observation::Unavailable { .. } => false,
    });
    if changed {
        evidence
            .limitations
            .push(CostEvidenceLimitation::SourceWindowChanged);
    }
}

pub(super) fn samples(
    snapshot: PublicMetricsSnapshot,
    family: PublicMetricFamily,
) -> Result<CostSamplesView, ObservationFailure> {
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
                .is_none_or(|at| row.observed_at_ns > at)
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
            _ => false,
        })
        .map(|row| CostMetricView {
            name: row.name,
            canister_id: row.canister_id.map(|id| id.to_text()),
            value: row.value.to_string(),
            unit: row.unit,
            observed_at_ns: row.observed_at_ns,
            measurement: match row.kind {
                PublicMetricKind::Gauge => CostMetricKind::Gauge,
                PublicMetricKind::Counter {
                    window_id,
                    saturated,
                } => CostMetricKind::Counter {
                    window_id,
                    saturated,
                },
            },
        })
        .collect();
    Ok(CostSamplesView {
        state: match snapshot.state {
            PublicSnapshotState::Disabled => CostSampleState::Disabled,
            PublicSnapshotState::Unavailable => CostSampleState::Unavailable,
            PublicSnapshotState::Fresh => CostSampleState::Fresh,
            PublicSnapshotState::Stale => CostSampleState::Stale,
        },
        sampled_at_ns: snapshot.sampled_at_ns,
        stale_after_ns: snapshot.stale_after_ns,
        truncated,
        rows,
    })
}
