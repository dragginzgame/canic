//! Module: ops::runtime::public_metrics::performance
//!
//! Responsibility: project bounded typed performance observations into public rows.
//! Does not own: recording, timer attribution, registration or publication cadence.
//! Boundary: preserve wide totals and source quality; labels never recover identity.

use crate::{
    InternalError,
    domain::public_metrics::{PublicMetricKind, TimerMetricRegistration},
    model::public_metrics::{MAX_PUBLIC_METRICS, PublicMetricSample},
    ops::runtime::metrics,
    perf,
    perf::PerfEntry,
};
use ic_metrics::MeasurementAggregate;

#[cfg(test)]
mod tests;

const SOURCE_LIMIT: usize = MAX_PUBLIC_METRICS / 2 + 1;

pub(super) fn sample() -> Result<Vec<PublicMetricSample>, InternalError> {
    let inventory = ic_timers::timer_inventory().ok();
    let timers = inventory.as_ref().map_or(&[][..], |value| value.timers());
    project(perf::bounded_entries(SOURCE_LIMIT)?, timers)
}

fn project(
    measurements: Vec<PerfEntry>,
    timers: &[ic_timers::TimerSnapshot],
) -> Result<Vec<PublicMetricSample>, InternalError> {
    let mut rows = Vec::new();
    for measurement in measurements.into_iter().take(SOURCE_LIMIT) {
        append(
            &mut rows,
            metrics::perf_labels(measurement.key),
            measurement.aggregate,
            None,
        )?;
    }
    for timer in timers {
        let id = timer.registration_id();
        let registration = TimerMetricRegistration {
            canister_version: id.epoch().canister_version(),
            started_at_ns: id.epoch().started_at_ns(),
            sequence: id.sequence(),
        };
        let performance = timer.observability().performance();
        for (phase, summary) in [
            ("scheduler", performance.scheduler_instructions()),
            ("work", performance.work_instructions()),
        ] {
            if rows.len() / 2 >= SOURCE_LIMIT {
                return Ok(rows);
            }
            append(
                &mut rows,
                metrics::timer_perf_labels(timer.identity(), phase),
                summary.aggregate(),
                Some(registration),
            )?;
        }
    }
    Ok(rows)
}

fn append(
    rows: &mut Vec<PublicMetricSample>,
    mut labels: Vec<String>,
    aggregate: MeasurementAggregate,
    registration: Option<TimerMetricRegistration>,
) -> Result<(), InternalError> {
    labels.insert(0, "perf".into());
    let name = super::metric_name(&labels, 6)?;
    for (name, value, unit, saturated) in [
        (
            format!("{name}.calls"),
            u128::from(aggregate.samples()),
            "count",
            aggregate.samples_overflowed(),
        ),
        (
            name,
            aggregate.total(),
            "instructions",
            aggregate.total_overflowed(),
        ),
    ] {
        let kind = registration.map_or(
            PublicMetricKind::Counter {
                window_id: 0,
                saturated,
            },
            |registration| PublicMetricKind::TimerCounter {
                registration,
                saturated,
            },
        );
        rows.push(PublicMetricSample {
            name,
            canister_id: None,
            value,
            unit: unit.into(),
            observed_at_ns: 0,
            kind,
        });
    }
    Ok(())
}
