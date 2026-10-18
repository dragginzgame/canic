use super::*;
use crate::{
    model::public_metrics::MAX_PUBLIC_METRIC_TEXT_BYTES,
    ops::runtime::public_metrics::PublicMetricsOps, perf::PerfKey,
};

fn checkpoint(aggregate: MeasurementAggregate) -> PerfMeasurement {
    PerfMeasurement {
        key: PerfKey::Checkpoint {
            scope: "work".into(),
            label: "completed".into(),
        },
        aggregate,
    }
}

#[test]
fn public_performance_keeps_empty_zero_and_wide_totals() {
    for aggregate in [
        MeasurementAggregate::EMPTY,
        MeasurementAggregate::from_parts(1, 0, Some(0), false, false).unwrap(),
        MeasurementAggregate::from_parts(2, u128::from(u64::MAX) + 1, Some(u64::MAX), false, false)
            .unwrap(),
    ] {
        let rows = project(vec![checkpoint(aggregate)], &[]).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].name, "perf.checkpoint.work.completed.calls");
        assert_eq!(rows[0].value, u128::from(aggregate.samples()));
        assert_eq!(rows[0].unit, "count");
        assert_eq!(rows[1].name, "perf.checkpoint.work.completed");
        assert_eq!(rows[1].value, aggregate.total());
        assert_eq!(rows[1].unit, "instructions");
        for row in rows {
            assert_eq!(
                row.kind,
                PublicMetricKind::Counter {
                    window_id: 0,
                    saturated: false
                }
            );
        }
    }
}

#[test]
fn public_performance_uses_actual_count_and_total_overflow_independently() {
    for (aggregate, flags) in [
        (
            MeasurementAggregate::from_parts(u64::MAX, u128::from(u64::MAX), Some(1), false, false)
                .unwrap(),
            [false, false],
        ),
        (
            MeasurementAggregate::from_parts(u64::MAX, 0, Some(0), true, false).unwrap(),
            [true, false],
        ),
        (
            MeasurementAggregate::from_parts(u64::MAX, u128::MAX, Some(u64::MAX), true, true)
                .unwrap(),
            [true, true],
        ),
    ] {
        let rows = project(vec![checkpoint(aggregate)], &[]).unwrap();
        for (row, saturated) in rows.iter().zip(flags) {
            assert_eq!(
                row.kind,
                PublicMetricKind::Counter {
                    window_id: 0,
                    saturated
                }
            );
        }
    }
}

#[test]
fn timer_registration_is_carried_without_matching_label_text() {
    let registration = TimerMetricRegistration {
        canister_version: 7,
        started_at_ns: 11,
        sequence: 13,
    };
    let aggregate = MeasurementAggregate::from_parts(u64::MAX, 0, Some(0), true, false).unwrap();
    let mut rows = Vec::new();
    append(
        &mut rows,
        vec!["timer".into(), "display".into()],
        aggregate,
        Some(registration),
    )
    .unwrap();
    assert_eq!(
        rows[0].kind,
        PublicMetricKind::TimerCounter {
            registration,
            saturated: true
        }
    );
    assert_eq!(
        rows[1].kind,
        PublicMetricKind::TimerCounter {
            registration,
            saturated: false
        }
    );
}

#[test]
fn performance_projection_limits_sources_and_refuses_oversized_names() {
    let make_measurement = |label: String| PerfMeasurement {
        key: PerfKey::Checkpoint {
            scope: "bounded".into(),
            label,
        },
        aggregate: MeasurementAggregate::EMPTY,
    };
    let too_long = "x".repeat(MAX_PUBLIC_METRIC_TEXT_BYTES + 1);
    assert!(project(vec![make_measurement(too_long.clone())], &[]).is_err());
    let mut measurements = (0..SOURCE_LIMIT)
        .map(|index| make_measurement(index.to_string()))
        .collect::<Vec<_>>();
    measurements.push(make_measurement(too_long));
    let rows = project(measurements, &[]).unwrap();
    assert_eq!(rows.len(), 2 * SOURCE_LIMIT);
    assert_eq!(rows[0].name, "perf.checkpoint.bounded.0.calls");
}

#[test]
fn performance_publication_preserves_wide_source_quality_and_time() {
    perf::reset();
    perf::record_checkpoint("wide", "published", u64::MAX);
    let family = crate::domain::public_metrics::PublicMetricFamily::Performance;
    PublicMetricsOps::sample_family(family, 11).unwrap();
    let exact = crate::model::public_metrics::PublicMetricsCache::snapshot(family).unwrap();
    let exact = exact
        .metrics
        .iter()
        .find(|row| row.name == "perf.checkpoint.wide.published")
        .unwrap();
    assert_eq!(exact.value, u128::from(u64::MAX));
    assert_eq!(
        exact.kind,
        PublicMetricKind::Counter {
            window_id: 0,
            saturated: false
        }
    );
    perf::record_checkpoint("wide", "published", 1);
    PublicMetricsOps::sample_family(
        crate::domain::public_metrics::PublicMetricFamily::Performance,
        17,
    )
    .unwrap();
    let snapshot = crate::model::public_metrics::PublicMetricsCache::snapshot(
        crate::domain::public_metrics::PublicMetricFamily::Performance,
    )
    .unwrap();
    let row = snapshot
        .metrics
        .iter()
        .find(|row| row.name == "perf.checkpoint.wide.published")
        .unwrap();
    assert_eq!(row.value, u128::from(u64::MAX) + 1);
    assert_eq!(
        row.kind,
        PublicMetricKind::Counter {
            window_id: 0,
            saturated: false
        }
    );
    assert_eq!(row.observed_at_ns, 17);
    perf::reset();
}
