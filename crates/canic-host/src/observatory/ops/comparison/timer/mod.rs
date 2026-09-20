//! Module: observatory::ops::comparison::timer
//!
//! Responsibility: qualify per-registration timer measurements from saved source snapshots.
//! Boundary: independent instruction/sample deltas, never callback rates or billed cycles.

use crate::observatory::{
    model::cost::CostCounterReading,
    ops::comparison::{result, source},
    policy::cost,
    view::*,
};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn compare(
    before: &ObservatoryRoleView,
    after: &ObservatoryRoleView,
) -> Result<Vec<TimerMetricComparisonView>, CostComparisonFailure> {
    source::same_window(before, after)?;
    let old = rows(before)?;
    let new = rows(after)?;
    if old.is_empty() && new.is_empty() {
        return Err(CostComparisonFailure::MissingMetric);
    }
    let names: BTreeSet<_> = old.keys().chain(new.keys()).copied().collect();
    Ok(names
        .into_iter()
        .map(|name| TimerMetricComparisonView {
            name: name.into(),
            movement: result(movement(old.get(name).copied(), new.get(name).copied())),
        })
        .collect())
}

fn rows(
    role: &ObservatoryRoleView,
) -> Result<BTreeMap<&str, &CostMetricView>, CostComparisonFailure> {
    let Observation::Observed {
        source: ObservationSource::PublicMetricCache,
        value,
        ..
    } = &source::evidence(role)?.timer_instructions
    else {
        return Err(CostComparisonFailure::SnapshotUnavailable);
    };
    match value.state {
        CostSampleState::Fresh => (),
        CostSampleState::Stale => return Err(CostComparisonFailure::StaleSample),
        _ => return Err(CostComparisonFailure::SnapshotUnavailable),
    }
    if value.truncated {
        return Err(CostComparisonFailure::TruncatedSample);
    }
    if value.rows.len() > 256 || value.sampled_at_ns.is_none() {
        return Err(CostComparisonFailure::InvalidMetric);
    }
    let window = source::window(role)?;
    let mut rows = BTreeMap::new();
    for row in &value.rows {
        let valid_name = row.name.starts_with("perf.timer.")
            && row.name.len() <= 128
            && !row.name.chars().any(char::is_control);
        let (phase, unit) = if let Some(phase) = row.name.strip_suffix(".calls") {
            (phase, "count")
        } else {
            (row.name.as_str(), "instructions")
        };
        let valid_phase = matches!(phase.rsplit('.').next(), Some("scheduler" | "work"));
        let exact_sample = value.sampled_at_ns == Some(row.observed_at_ns);
        let valid_identity = valid_name && valid_phase && row.canister_id.is_none();
        let valid_observation = exact_sample && row.unit == unit;
        if !valid_identity || !valid_observation {
            return Err(CostComparisonFailure::InvalidMetric);
        }
        if window
            .heap_started_at_ns
            .is_some_and(|start| row.observed_at_ns < start)
        {
            return Err(CostComparisonFailure::SourceWindowChanged);
        }
        let CostMetricKind::TimerCounter { registration, .. } = row.measurement else {
            return Err(CostComparisonFailure::InvalidMetric);
        };
        if registration.sequence == 0 || registration.started_at_ns > row.observed_at_ns {
            return Err(CostComparisonFailure::InvalidMetric);
        }
        if registration.canister_version != window.canister_version {
            return Err(CostComparisonFailure::SourceWindowChanged);
        }
        if rows.insert(row.name.as_str(), row).is_some() {
            return Err(CostComparisonFailure::InvalidMetric);
        }
    }
    Ok(rows)
}

fn movement(
    before: Option<&CostMetricView>,
    after: Option<&CostMetricView>,
) -> Result<TimerMetricMovementView, CostComparisonFailure> {
    let before = before.ok_or(CostComparisonFailure::MissingMetric)?;
    let after = after.ok_or(CostComparisonFailure::MissingMetric)?;
    let (old_registration, old) = reading(before)?;
    let (registration, new) = reading(after)?;
    if old_registration != registration {
        return Err(CostComparisonFailure::TimerRegistrationChanged);
    }
    let delta = cost::counter(old, new).map_err(source::numeric_failure)?;
    Ok(TimerMetricMovementView {
        registration,
        start_ns: delta.interval.start_ns,
        end_ns: delta.interval.end_ns,
        elapsed_ns: delta.interval.end_ns - delta.interval.start_ns,
        unit: after.unit.clone(),
        amount: delta.amount.to_string(),
    })
}

fn reading(
    row: &CostMetricView,
) -> Result<(TimerRegistrationView, CostCounterReading), CostComparisonFailure> {
    let CostMetricKind::TimerCounter {
        registration,
        saturated,
    } = row.measurement
    else {
        return Err(CostComparisonFailure::InvalidMetric);
    };
    let value = source::value(row)?;
    if value > u128::from(u64::MAX) {
        return Err(CostComparisonFailure::InvalidMetric);
    }
    Ok((
        registration,
        CostCounterReading {
            value,
            observed_at_ns: row.observed_at_ns,
            window_id: registration.sequence,
            saturated: saturated || value == u128::from(u64::MAX),
        },
    ))
}
