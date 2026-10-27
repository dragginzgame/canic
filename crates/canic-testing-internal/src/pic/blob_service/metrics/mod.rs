//! Qualify the adapter's sole sampler through the installed public cache.

use canic::dto::{
    page::PageRequest,
    public_status::{
        PublicMetricFamily, PublicMetricKind, PublicMetricsRequest, PublicMetricsSnapshot,
        PublicSnapshotState,
    },
};
use ic_platform::pocket_ic::PocketIc;
use ic_platform::principal::ExternalPrincipal as Principal;
use std::{collections::BTreeSet, time::Duration};

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
    restored_fence: bool,
    application_count: Option<u128>,
) {
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
    if restored_fence {
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
    assert_eq!(names, expected.iter().map(|(name, _, _)| *name).collect());
    assert_eq!(sample.metrics.total, u64::try_from(expected.len()).unwrap());
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
}
