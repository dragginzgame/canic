//! Module: observatory::ops::cost
//!
//! Responsibility: bounded projection of existing metric owners into private cost evidence.
//! Boundary: read cached measurements; never schedule work or infer cycle consumption.

use crate::{
    observatory::{
        ops::{outcome, transport::ObservatoryTransport},
        view::*,
    },
    registry::RegistryEntry,
};
use canic_core::dto::public_status::PublicMetricFamily;

pub(super) fn collect(
    entry: &RegistryEntry,
    transport: &mut impl ObservatoryTransport,
) -> RoleCostEvidenceView {
    let source = ObservationSource::PublicMetricCache;
    let mut evidence = RoleCostEvidenceView {
        balance: outcome(
            transport.metric_samples(entry, PublicMetricFamily::Cycles),
            source,
        ),
        funding_and_callbacks: outcome(
            transport.metric_samples(entry, PublicMetricFamily::Operations),
            source,
        ),
        timer_instructions: outcome(
            transport.metric_samples(entry, PublicMetricFamily::Performance),
            source,
        ),
        // Read after the families: a restart between reads invalidates samples older than this heap.
        window: outcome(transport.cost_window(entry), source),
        limitations: vec![
            CostEvidenceLimitation::SingleSnapshot,
            CostEvidenceLimitation::AggregateTimerCallbacksUnqualified,
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
