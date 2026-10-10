use super::*;
use canic_contracts::dto::{page::Page, public_status::PublicMetric};

fn snapshot(family: PublicMetricFamily, names: &[&str]) -> PublicMetricsSnapshot {
    PublicMetricsSnapshot {
        family,
        state: PublicSnapshotState::Fresh,
        sampled_at_ns: Some(90),
        stale_after_ns: 500,
        truncated: false,
        metrics: Page {
            total: names.len() as u64,
            entries: names
                .iter()
                .map(|name| PublicMetric {
                    name: (*name).into(),
                    canister_id: None,
                    value: u128::MAX,
                    unit: "cycles".into(),
                    observed_at_ns: 90,
                    kind: PublicMetricKind::Counter {
                        window_id: 7,
                        saturated: true,
                    },
                })
                .collect(),
        },
    }
}

#[test]
fn grants_keep_precision_source_time_and_saturation_without_inventing_consumption() {
    let family = PublicMetricFamily::Operations;
    let mut reply = snapshot(
        family,
        &[
            "cycles_funding.cycles_granted_total",
            "cycles_funding.cycles_granted_to_child",
            "cycles_funding.cycles_requested_total",
            "timer.events.scheduler_started",
        ],
    );
    reply.state = PublicSnapshotState::Stale;
    reply.metrics.entries[1].canister_id = Some(candid::Principal::anonymous());
    let view = samples(reply, family).unwrap();
    assert_eq!(view.state, MetricSampleState::Stale);
    assert_eq!(view.rows.len(), 3);
    assert_eq!(view.rows[0].value, u128::MAX.to_string());
    assert_eq!(view.rows[0].observed_at_ns, 90);
    assert_eq!(
        view.rows[0].measurement,
        MetricKind::Counter {
            window_id: 7,
            saturated: true
        }
    );
    assert_eq!(view.rows[1].canister_id.as_deref(), Some("2vxsx-fae"));
    assert_eq!(view.sampled_at_ns, Some(90));
}

#[test]
fn timer_measurements_keep_source_registration_and_both_phases() {
    let family = PublicMetricFamily::Performance;
    let mut reply = snapshot(
        family,
        &[
            "perf.timer.app.checkpoint.tick.scheduler",
            "perf.timer.app.checkpoint.tick.scheduler.calls",
            "perf.timer.app.checkpoint.tick.work",
            "perf.timer.app.checkpoint.tick.work.calls",
            "perf.endpoint.query.status",
        ],
    );
    for row in &mut reply.metrics.entries {
        row.kind = PublicMetricKind::TimerCounter {
            registration: canic_contracts::dto::public_status::TimerMetricRegistration {
                canister_version: 3,
                started_at_ns: 10,
                sequence: 7,
            },
            saturated: true,
        };
    }
    let view = samples(reply, family).unwrap();
    assert_eq!(view.rows.len(), 4);
    assert!(view.rows.iter().all(|row| row.measurement
        == MetricKind::TimerCounter {
            registration: TimerRegistrationView {
                canister_version: 3,
                started_at_ns: 10,
                sequence: 7
            },
            saturated: true,
        }));
    let json = serde_json::to_vec(&view).unwrap();
    assert_eq!(
        serde_json::from_slice::<MetricSamplesView>(&json).unwrap(),
        view
    );
}

#[test]
fn pagination_and_source_truncation_survive_filtering() {
    let family = PublicMetricFamily::Operations;
    let mut reply = snapshot(family, &["unselected"]);
    reply.metrics.total = 2;
    let view = samples(reply, family).unwrap();
    assert!(view.truncated);
    assert!(view.rows.is_empty());
    let mut reply = snapshot(family, &[]);
    reply.truncated = true;
    assert!(samples(reply, family).unwrap().truncated);
}

#[test]
fn malformed_pages_reject_before_projection() {
    let family = PublicMetricFamily::Cycles;
    assert_eq!(
        samples(
            snapshot(family, &["balance"]),
            PublicMetricFamily::Performance
        ),
        Err(ObservationFailure::InvalidResponse)
    );
    assert_eq!(
        samples(snapshot(family, &["balance", "balance"]), family),
        Err(ObservationFailure::InvalidResponse)
    );
    let mut reply = snapshot(family, &["balance"]);
    reply.metrics.entries[0].observed_at_ns = 89;
    assert_eq!(
        samples(reply, family),
        Err(ObservationFailure::InvalidResponse)
    );
    let names = vec!["balance"; 257];
    assert_eq!(
        samples(snapshot(family, &names), family),
        Err(ObservationFailure::InvalidResponse)
    );
    let mut reply = snapshot(family, &["balance"]);
    reply.metrics.total = 0;
    assert_eq!(
        samples(reply, family),
        Err(ObservationFailure::InvalidResponse)
    );
}

#[test]
fn disabled_empty_family_remains_disabled() {
    let family = PublicMetricFamily::Cycles;
    let mut reply = snapshot(family, &[]);
    reply.state = PublicSnapshotState::Disabled;
    reply.sampled_at_ns = None;
    let view = samples(reply, family).unwrap();
    assert_eq!(view.state, MetricSampleState::Disabled);
    assert_eq!(view.sampled_at_ns, None);
    assert!(view.rows.is_empty());
}

#[test]
fn application_metrics_preserve_arbitrary_names_units_and_mixed_source_times() {
    let family = PublicMetricFamily::Application;
    let mut reply = snapshot(family, &["app.queue_depth", "app.accounted_bytes"]);
    reply.metrics.entries[0].kind = PublicMetricKind::Gauge;
    reply.metrics.entries[0].unit = "count".into();
    reply.metrics.entries[1].observed_at_ns = 95;
    reply.metrics.entries[1].unit = "bytes".into();
    reply.metrics.total = 3;
    let view = samples(reply, family).unwrap();
    assert_eq!(view.rows[0].name, "app.queue_depth");
    assert_eq!(view.rows[0].unit, "count");
    assert_eq!(view.rows[0].measurement, MetricKind::Gauge);
    assert_eq!(view.rows[1].name, "app.accounted_bytes");
    assert_eq!(view.rows[1].unit, "bytes");
    assert_eq!(view.rows[1].value, u128::MAX.to_string());
    assert_eq!(view.rows[1].observed_at_ns, 95);
    assert_eq!(view.sampled_at_ns, Some(90));
    assert!(view.truncated);
}

#[test]
fn application_metrics_reject_ambiguous_or_unbounded_rows() {
    let family = PublicMetricFamily::Application;
    for name in [String::new(), "x".repeat(129), "line\nbreak".into()] {
        assert_eq!(
            samples(snapshot(family, &[&name]), family),
            Err(ObservationFailure::InvalidResponse)
        );
    }
    let mut reply = snapshot(family, &["app.queue_depth"]);
    reply.sampled_at_ns = None;
    assert_eq!(
        samples(reply, family),
        Err(ObservationFailure::InvalidResponse)
    );
    assert_eq!(
        samples(snapshot(family, &["same", "same"]), family),
        Err(ObservationFailure::InvalidResponse)
    );
}
