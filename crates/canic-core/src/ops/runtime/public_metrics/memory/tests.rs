#![cfg(test)]
use super::*;
use crate::{
    dto::{page::PageRequest, public_status::PublicMetricsRequest},
    model::public_metrics::{PUBLIC_METRICS_STALE_AFTER_NS, PublicHistoryCache},
};
use std::collections::BTreeSet;
fn report() -> StorageDiagnostics {
    MemoryRegistryOps::init_registry().unwrap();
    MemoryRegistryOps::allocation_summary().unwrap()
}
fn value(rows: &[PublicMetricSample], name: &str) -> u128 {
    rows.iter()
        .find(|row| row.name == format!("{PREFIX}{name}"))
        .unwrap()
        .value
}
#[test]
fn directory_metrics_conserve_capacity_without_opening_allocations() {
    let before = report();
    let detailed = MemoryRegistryOps::allocation_snapshot().unwrap();
    let rows = project(&before, 10).unwrap();
    assert_eq!(
        value(&rows, "physical_extent"),
        u128::from(before.physical_pages) * 65_536
    );
    assert_eq!(
        value(&rows, "physical_extent"),
        value(&rows, "data_capacity")
            + value(&rows, "directory_capacity")
            + value(&rows, "unassigned_capacity")
            + 65_536
    );
    assert_eq!(value(&rows, "allocations"), detailed.memories.len() as u128);
    assert_eq!(value(&rows, "live_leases"), 0);
    assert!(
        rows.iter()
            .all(|row| row.canister_id.is_none() && row.observed_at_ns == 10)
    );
    assert_eq!(report(), before);
    assert_eq!(MemoryRegistryOps::allocation_snapshot().unwrap(), detailed);
}
#[test]
fn inconsistent_capacity_or_counts_refuse_projection() {
    let good = report();
    let mut capacity = good;
    capacity.physical_pages += 1;
    let mut counts = good;
    counts.admitted_allocations = counts.allocations + 1;
    let mut leases = good;
    leases.live_leases = leases.allocations + 1;
    let mut overflow = good;
    overflow.data_pages = u64::MAX;
    for invalid_report in [capacity, counts, leases, overflow] {
        assert_eq!(
            project(&invalid_report, 10).unwrap_err().code(),
            invalid().code()
        );
    }
}

#[test]
fn disabled_and_unsupported_never_collect_or_retain_values() {
    let prior = project(&report(), 10).unwrap();
    PublicMetricsCache::replace(PublicMetricFamily::Performance, 10, prior).unwrap();
    assert!(collect(false, true, 20, || panic!("disabled collection")).is_empty());
    let unsupported = collect(true, false, 20, || panic!("unsupported collection"));
    assert_eq!(
        unsupported,
        vec![state(AllocationSampleState::Unsupported, 20)]
    );
    let request = PublicMetricsRequest {
        family: PublicMetricFamily::Performance,
        page: PageRequest {
            offset: 0,
            limit: 256,
        },
    };
    let hidden = PublicMetricsOps::project(request, &BTreeSet::default(), 20);
    assert!(hidden.metrics.entries.is_empty());
    assert_eq!(hidden.sampled_at_ns, None);
}

#[test]
fn failed_collection_keeps_source_time_and_history_then_recovers() {
    let source = report();
    let first = project(&source, 10).unwrap();
    PublicMetricsCache::replace(PublicMetricFamily::Performance, 10, first.clone()).unwrap();
    let now = 10 + PUBLIC_METRICS_STALE_AFTER_NS;
    let failed = collect(true, true, now, || Err(invalid()));
    assert_eq!(value(&failed, "state"), 3);
    for previous in first.iter().filter(|row| row.name != STATE) {
        assert_eq!(
            failed.iter().find(|row| row.name == previous.name),
            Some(previous)
        );
    }
    PublicMetricsCache::replace(PublicMetricFamily::Performance, now, failed).unwrap();
    assert_eq!(
        PublicMetricsCache::snapshot(PublicMetricFamily::Performance)
            .unwrap()
            .sampled_at_ns,
        10
    );
    let history = PublicHistoryCache::series(
        PublicMetricFamily::Performance,
        "memory.allocations.physical_extent".into(),
        None,
    )
    .unwrap();
    assert_eq!(history.slots.last().unwrap().observed_at_ns, 10);
    let recovered = collect(true, true, now + 1, || Ok(source));
    assert_eq!(value(&recovered, "state"), 1);
    assert!(recovered.iter().all(|row| row.observed_at_ns == now + 1));
}

#[test]
fn first_collection_failure_has_no_fabricated_zero_measurements() {
    let rows = collect(true, true, 10, || Err(invalid()));
    assert_eq!(rows, vec![state(AllocationSampleState::Failed, 10)]);
}
