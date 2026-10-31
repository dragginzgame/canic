//! Qualify the adapter's sole sampler through the installed public cache.

use std::{collections::BTreeSet, time::Duration};

use canic::dto::{
    page::PageRequest,
    public_status::{
        PublicMetricFamily, PublicMetricKind, PublicMetricsRequest, PublicMetricsSnapshot,
        PublicSnapshotState,
    },
};
use ic_platform::pocket_ic::PocketIc;
use ic_platform::principal::ExternalPrincipal as Principal;

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Phase {
    Fresh,
    Restored,
    Resumed,
}

pub(super) fn protected_reads(pic: &PocketIc, service: Principal, controller: Principal) {
    let page = PageRequest {
        offset: 0,
        limit: 20,
    };
    for caller in [
        controller,
        Principal::anonymous(),
        Principal::from_slice(&[99; 29]),
    ] {
        let logs: Result<canic::dto::page::Page<canic::dto::log::LogEntry>, canic::Error> =
            ic_testkit::pic::query_wire(
                pic,
                service,
                caller,
                ic_testkit::pic::WireCallSpec {
                    method: canic::protocol::protected_logs::METHOD,
                    method_id: canic::protocol::protected_logs::METHOD_ID,
                    request_limits: canic::protocol::protected_logs::REQUEST_LIMITS,
                    response_limits: canic::protocol::protected_logs::RESPONSE_LIMITS,
                },
                &canic::dto::role::LogStatusRequest {
                    crate_name: None,
                    topic: None,
                    min_level: None,
                    page,
                },
            )
            .expect("installed logs query transport");
        let metrics: Result<
            canic::dto::page::Page<canic::dto::metric_page::MetricEntry>,
            canic::Error,
        > = ic_testkit::pic::query_wire(
            pic,
            service,
            caller,
            ic_testkit::pic::WireCallSpec {
                method: canic::protocol::protected_metrics::METHOD,
                method_id: canic::protocol::protected_metrics::METHOD_ID,
                request_limits: canic::protocol::protected_metrics::REQUEST_LIMITS,
                response_limits: canic::protocol::protected_metrics::RESPONSE_LIMITS,
            },
            &canic::dto::role::MetricsStatusRequest {
                kind: canic::dto::metrics::MetricsKind::Core,
                page,
            },
        )
        .expect("installed metrics query transport");
        if caller == controller {
            assert!(logs.expect("controller logs query").entries.len() <= 20);
            assert!(metrics.expect("controller metrics query").entries.len() <= 20);
        } else {
            let denied = canic::diagnostics::codes::AUTHORITY_UNAVAILABLE.raw_code();
            assert_eq!(
                logs.err().expect("non-controller logs refusal").code(),
                denied
            );
            assert_eq!(
                metrics.expect_err("non-controller metrics refusal").code(),
                denied
            );
        }
    }
}

pub(super) fn admission_replay(
    pic: &PocketIc,
    permission: canic_blob_service::__private::ic_blob_storage_contracts::dto::upload::admission::UploadAdmissionRequest,
) {
    use canic_blob_service::__private::ic_blob_storage_contracts as contracts;
    let bytes = pic
        .update_call(
            permission.upload.service.into(),
            permission.upload.tenant.into(),
            "blob_admit_upload",
            contracts::wire::encode_request("blob_admit_upload", permission).unwrap(),
        )
        .unwrap();
    let replay: Result<
        contracts::dto::upload::admission::UploadAdmissionMutation,
        contracts::dto::upload::admission::UploadAdmissionFailure,
    > = super::tests::decode_blob("blob_admit_upload", &bytes).unwrap();
    assert!(replay.unwrap().replayed);
    // Decoder traps happen before recording: the following exact totals must
    // still contain eight completed RPCs, with one replay and two new grants.
    assert!(
        pic.update_call(
            permission.upload.service.into(),
            permission.upload.tenant.into(),
            "blob_admit_upload",
            vec![0]
        )
        .is_err()
    );
}

fn snapshot(
    pic: &PocketIc,
    service: Principal,
    family: PublicMetricFamily,
) -> PublicMetricsSnapshot {
    ic_testkit::pic::query_wire(
        pic,
        service,
        Principal::anonymous(),
        ic_testkit::pic::WireCallSpec {
            method: canic::protocol::CANIC_PUBLIC_METRICS,
            method_id: canic::protocol::public_metrics::METHOD_ID,
            request_limits: canic::protocol::public_metrics::REQUEST_LIMITS,
            response_limits: canic::protocol::public_metrics::RESPONSE_LIMITS,
        },
        &PublicMetricsRequest {
            family,
            page: PageRequest {
                offset: 0,
                limit: 256,
            },
        },
    )
    .expect("public metrics Wire query")
}

pub(super) fn check(
    pic: &PocketIc,
    service: Principal,
    reservations: u128,
    phase: Phase,
    application_count: Option<u128>,
    durable: bool,
) -> u64 {
    let after = pic.get_time().as_nanos_since_unix_epoch();
    // Production sampling is a five-minute timer. Queries must not refresh it.
    pic.advance_time(Duration::from_secs(300));
    let mut sample = snapshot(pic, service, PublicMetricFamily::Application);
    for _ in 0..30 {
        if sample.sampled_at_ns.is_some_and(|time| time > after) {
            break;
        }
        pic.tick();
        sample = snapshot(pic, service, PublicMetricFamily::Application);
    }
    assert_eq!(sample.family, PublicMetricFamily::Application);
    assert_eq!(sample.state, PublicSnapshotState::Fresh);
    let sampled_at = sample
        .sampled_at_ns
        .expect("sample was published by the timer");
    assert!(sampled_at > after);
    assert!(!sample.truncated);
    let configuration: Result<
        canic_blob_service::dto::configuration::HostConfigurationView,
        canic_blob_service::dto::configuration::HostFailure,
    > = super::tests::decode_blob(
        "blob_configuration",
        &pic.query_call(
            service,
            Principal::from_slice(&[7; 29]),
            "blob_configuration",
            canic_blob_service::__private::ic_blob_storage_contracts::wire::encode_request(
                "blob_configuration",
                (),
            )
            .unwrap(),
        )
        .unwrap(),
    )
    .unwrap();
    let fenced = configuration.unwrap().fenced;
    if phase == Phase::Restored {
        assert!(fenced);
    }
    let mut expected = vec![
        ("blob.active_reservations", reservations, "count"),
        ("blob.fenced", u128::from(fenced), "boolean"),
        ("blob.liability_bytes", reservations, "bytes"),
        ("blob.logical_bytes", reservations, "bytes"),
        ("blob.operation_slots", reservations, "count"),
        ("blob.physical_bytes", reservations, "bytes"),
        ("blob.reserved_bytes", reservations, "bytes"),
        ("blob.metrics.durable", u128::from(durable), "boolean"),
    ];
    if let Some(count) = application_count {
        expected.push(("app.count", count, "count"));
    }
    let names = sample
        .metrics
        .entries
        .iter()
        .map(|row| row.name.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(names.len(), sample.metrics.entries.len());
    let mut expected_names = expected
        .iter()
        .map(|(name, _, _)| (*name).to_owned())
        .collect::<BTreeSet<_>>();
    let window = check_counters(
        &sample,
        phase == Phase::Fresh || durable,
        &mut expected_names,
    );
    assert_eq!(names, expected_names.iter().map(String::as_str).collect());
    assert_eq!(
        sample.metrics.total,
        u64::try_from(expected_names.len()).unwrap()
    );
    for (name, value, unit) in expected {
        let row = sample
            .metrics
            .entries
            .iter()
            .find(|row| row.name == name)
            .unwrap();
        assert_eq!(row.value, value, "{name}");
        assert_eq!(row.unit, unit);
        assert_eq!(row.kind, PublicMetricKind::Gauge);
        assert_eq!(row.canister_id, Some(service.into()));
        assert_eq!(row.observed_at_ns, sampled_at);
    }
    let repeated = snapshot(pic, service, PublicMetricFamily::Application);
    assert_eq!(repeated.sampled_at_ns, sample.sampled_at_ns);
    assert_eq!(repeated.metrics.entries, sample.metrics.entries);
    let disabled = snapshot(pic, service, PublicMetricFamily::Cycles);
    assert_eq!(disabled.state, PublicSnapshotState::Disabled);
    assert!(disabled.metrics.entries.is_empty());
    assert_eq!(disabled.sampled_at_ns, None);
    window
}

fn check_counters(
    sample: &PublicMetricsSnapshot,
    retained: bool,
    names: &mut BTreeSet<String>,
) -> u64 {
    let row = |name: &str| {
        sample
            .metrics
            .entries
            .iter()
            .find(|row| row.name == name)
            .unwrap()
    };
    let PublicMetricKind::Counter {
        window_id,
        saturated: false,
    } = row("blob.admission.attempts").kind
    else {
        panic!("exact counter quality/window");
    };
    for domain in ["admission", "reference"] {
        let attempts = if retained && domain == "admission" {
            8
        } else {
            0
        };
        for (suffix, expected, unit) in [
            ("attempts", Some(attempts), "events"),
            ("applied", Some(if attempts > 0 { 2 } else { 0 }), "events"),
            ("replayed", Some(u128::from(attempts > 0)), "events"),
            ("refused", Some(if attempts > 0 { 5 } else { 0 }), "events"),
            (
                "request_bytes",
                if attempts == 0 { Some(0) } else { None },
                "bytes",
            ),
            (
                "instructions",
                if attempts == 0 { Some(0) } else { None },
                "instructions",
            ),
            ("instructions.samples", Some(attempts), "events"),
        ] {
            let name = format!("blob.{domain}.{suffix}");
            let value = row(&name);
            assert_eq!(value.unit, unit);
            assert_eq!(
                value.kind,
                PublicMetricKind::Counter {
                    window_id,
                    saturated: false
                }
            );
            if let Some(expected) = expected {
                assert_eq!(value.value, expected, "{name}");
            } else {
                assert!(value.value > 0, "{name}");
            }
            names.insert(name);
        }
        if attempts > 0 {
            let name = format!("blob.{domain}.instructions.maximum");
            assert_eq!(row(&name).kind, PublicMetricKind::Gauge);
            assert!(row(&name).value > 0);
            names.insert(name);
        }
    }
    window_id
}
