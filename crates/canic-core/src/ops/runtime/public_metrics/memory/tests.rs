#![cfg(test)]

use super::*;
use crate::{
    dto::{page::PageRequest, public_status::PublicMetricsRequest},
    model::public_metrics::{PUBLIC_METRICS_STALE_AFTER_NS, PublicHistoryCache},
};
use ic_memory::ic_stable_structures::{
    Memory, VectorMemory,
    memory_manager::{MemoryId, MemoryManager},
};
use std::{collections::BTreeSet, convert::Infallible};

struct FixturePolicy;

impl ic_memory::AllocationPolicy for FixturePolicy {
    type Error = Infallible;

    fn validate_key(&self, _: &ic_memory::StableKey) -> Result<(), Infallible> {
        Ok(())
    }

    fn validate_slot(
        &self,
        _: &ic_memory::StableKey,
        _: &ic_memory::AllocationSlotDescriptor,
    ) -> Result<(), Infallible> {
        Ok(())
    }

    fn validate_reserved_slot(
        &self,
        _: &ic_memory::StableKey,
        _: &ic_memory::AllocationSlotDescriptor,
    ) -> Result<(), Infallible> {
        Ok(())
    }
}

impl ic_memory::RuntimeBootstrapPolicy for FixturePolicy {
    fn runtime_bootstrap_identity(
        &self,
    ) -> Result<ic_memory::PolicyIdentity, ic_memory::PolicyIdentityError> {
        ic_memory::PolicyIdentity::new("public-memory-fixture", 1)
    }
}

#[test]
fn measured_nondefault_buckets_cover_every_usable_id_without_growth() {
    let backing = VectorMemory::default();
    let manager = MemoryManager::init_with_bucket_size(backing.clone(), 1);
    for id in 1..ic_memory::MEMORY_MANAGER_INVALID_ID {
        assert_eq!(manager.get(MemoryId::new(id)).grow(1), 0);
    }
    drop(manager);
    let mut runtime = ic_memory::MemoryRuntime::new_with_config(
        backing.clone(),
        ic_memory::MemoryManagerConfig::new(1).unwrap(),
    )
    .unwrap();
    runtime
        .bootstrap(
            &ic_memory::sealed_declaration_snapshot().unwrap(),
            &FixturePolicy,
        )
        .unwrap();
    let before = backing.borrow().clone();
    let report = runtime.memory_allocations().unwrap();
    let rows = project(&report, 10).unwrap();
    assert_eq!(*backing.borrow(), before);
    assert_eq!(value(&rows, "bucket_size"), 65_536);
    assert_eq!(
        value(&rows, "ids_measured"),
        u128::from(ic_memory::MEMORY_MANAGER_INVALID_ID)
    );
    assert!(
        report
            .memories
            .iter()
            .all(|entry| entry.allocated_bytes > 0)
    );
    assert!(value(&rows, "unknown_binding") > 0);
    assert_eq!(value(&rows, "physical_extent"), before.len() as u128);
}

fn report() -> MemoryAllocations {
    MemoryRegistryOps::init_registry().unwrap();
    MemoryRegistryOps::allocation_report().unwrap()
}

fn value(rows: &[PublicMetricSample], name: &str) -> u128 {
    rows.iter()
        .find(|row| row.name == format!("{PREFIX}{name}"))
        .unwrap()
        .value
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
fn public_projection_conserves_independent_partitions_without_mutating_memory() {
    let before = report();
    let rows = project(&before, 10).unwrap();
    assert_eq!(MemoryRegistryOps::allocation_report().unwrap(), before);
    assert_eq!(value(&rows, "state"), 1);
    assert_eq!(
        value(&rows, "physical_extent"),
        value(&rows, "manager_metadata")
            + value(&rows, "allocated_bucket_bytes")
            + value(&rows, "unmanaged")
    );
    assert_eq!(
        value(&rows, "allocated_bucket_bytes"),
        value(&rows, "virtual_extent") + value(&rows, "bucket_slack")
    );
    assert_eq!(
        value(&rows, "allocated_bucket_bytes"),
        value(&rows, "known_binding") + value(&rows, "unknown_binding")
    );
    assert_eq!(
        value(&rows, "known_binding"),
        value(&rows, "current_binding.allocated") + value(&rows, "ledger_binding.allocated")
    );
    assert_eq!(value(&rows, "ids_measured"), value(&rows, "ids_total"));
    assert_eq!(value(&rows, "payload_available"), 0);
    assert!(!rows.iter().any(|row| row.name.ends_with("payload_bytes")));
    assert!(
        rows.iter()
            .all(|row| row.canister_id.is_none() && row.observed_at_ns == 10)
    );
}

#[test]
fn unknown_binding_and_private_labels_do_not_change_public_label_inventory() {
    let mut source = report();
    let expected = project(&source, 10).unwrap();
    for entry in &mut source.memories {
        entry.binding = AllocationBinding::Unknown;
        entry.range_claim = Some(ic_memory::AllocationRangeClaim {
            authority: "private-user-controlled-owner".into(),
            mode: ic_memory::MemoryManagerRangeMode::Reserved,
        });
    }
    source.known_binding_bytes = 0;
    source.unknown_binding_bytes = source.allocated_bucket_bytes;
    let rows = project(&source, 20).unwrap();
    assert_eq!(
        value(&rows, "unknown_binding"),
        u128::from(source.allocated_bucket_bytes)
    );
    assert_eq!(value(&rows, "known_binding"), 0);
    assert_eq!(
        rows.iter().map(|row| &row.name).collect::<Vec<_>>(),
        expected.iter().map(|row| &row.name).collect::<Vec<_>>()
    );
}

#[test]
fn incomplete_or_inconsistent_reports_fail_each_conservation_partition() {
    let good = report();
    let mutations: [fn(&mut MemoryAllocations); 8] = [
        |r| r.physical_extent.bytes += 1,
        |r| r.virtual_extent.bytes += 1,
        |r| r.known_binding_bytes += 1,
        |r| r.remaining_buckets += 1,
        |r| r.bucket_size_bytes += 1,
        |r| {
            r.memories.pop();
        },
        |r| r.memories[1].memory_manager_id = 0,
        |r| r.memories[0].payload_bytes = Some(0),
    ];
    for mutate in mutations {
        let mut source = good.clone();
        mutate(&mut source);
        assert_eq!(
            project(&source, 20).unwrap_err().code(),
            crate::diagnostics::codes::STATE_INVALID
        );
    }
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
