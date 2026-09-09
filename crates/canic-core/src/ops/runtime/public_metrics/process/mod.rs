//! Module: ops::runtime::public_metrics::process
//!
//! Responsibility: collect bounded anonymous process aggregates from existing owners.
//! Does not own: protected diagnostic payloads, timer scheduling, or history.
//! Boundary: fixed labels and bounded source visits; no caller, target, intent, or row identities.

use crate::{
    InternalError,
    domain::public_metrics::PublicMetricKind,
    ids::SystemMetricKind,
    model::public_metrics::PublicMetricSample,
    ops::runtime::metrics::{
        intent::IntentMetrics, placement_index::PlacementIndexMetrics,
        platform_call::PlatformCallMetrics, system::SystemMetrics, wasm_store::WasmStoreMetrics,
    },
};
use std::collections::BTreeMap;

// Each owner has enum-only keys. Reject oversized inputs before scanning or allocating.
const MAX_PROCESS_SOURCE_ROWS: usize = 256;

pub(super) fn operations() -> Result<Vec<PublicMetricSample>, InternalError> {
    let mut rows = process_rows()?;
    let inventory = ic_timers::timer_inventory().map_err(|_| InternalError::invalid_input())?;
    rows.extend(timer_rows(inventory.timers()));
    Ok(rows)
}

fn process_rows() -> Result<Vec<PublicMetricSample>, InternalError> {
    let mut counts = BTreeMap::new();
    for (key, count) in PlatformCallMetrics::bounded_snapshot(MAX_PROCESS_SOURCE_ROWS)? {
        add(
            &mut counts,
            "platform_call",
            key.outcome.metric_label(),
            count,
        );
    }
    for (key, count) in IntentMetrics::bounded_snapshot(MAX_PROCESS_SOURCE_ROWS)? {
        add(&mut counts, "intent", key.outcome.metric_label(), count);
    }
    for (key, count) in PlacementIndexMetrics::bounded_snapshot(MAX_PROCESS_SOURCE_ROWS)? {
        add(
            &mut counts,
            "placement_index",
            key.outcome.metric_label(),
            count,
        );
    }
    for (key, count) in WasmStoreMetrics::bounded_snapshot(MAX_PROCESS_SOURCE_ROWS)? {
        add(&mut counts, "wasm_store", key.outcome.metric_label(), count);
    }
    add(
        &mut counts,
        "inter_canister_call",
        "started",
        SystemMetrics::count(SystemMetricKind::CanisterCall),
    );
    let rows = counts
        .into_iter()
        .map(|((owner, outcome), value)| {
            sample(format!("process.{owner}.{outcome}"), value, "count")
        })
        .collect::<Vec<_>>();
    Ok(rows)
}

fn add(
    counts: &mut BTreeMap<(&'static str, &'static str), u64>,
    owner: &'static str,
    outcome: &'static str,
    count: u64,
) {
    let total = counts.entry((owner, outcome)).or_default();
    *total = total.saturating_add(count);
}

fn timer_rows(timers: &[ic_timers::TimerSnapshot]) -> Vec<PublicMetricSample> {
    let mut counts = BTreeMap::new();
    // The timer owner admits at most 64 declarations, independently of topology size.
    for timer in timers {
        add(&mut counts, "state", timer.process_condition().label(), 1);
        let counters = timer.observability().counters();
        for (name, value) in [
            ("schedule_requests", counters.schedule_requests()),
            ("wakeups_armed", counters.wakeups_armed()),
            ("work_started", counters.work_started()),
            ("work_completed", counters.work_completed()),
            ("retryable_failure", counters.retryable_failure()),
            ("invariant_failure", counters.invariant_failure()),
            ("unacknowledged", counters.unacknowledged()),
            ("coalesced", counters.coalesced()),
        ] {
            add(&mut counts, "events", name, value);
        }
    }
    counts
        .into_iter()
        .map(|((category, name), value)| sample(format!("timer.{category}.{name}"), value, "count"))
        .collect()
}

#[cfg(target_arch = "wasm32")]
pub(super) fn memory() -> Vec<PublicMetricSample> {
    vec![
        sample(
            "memory.wasm_extent".into(),
            (core::arch::wasm32::memory_size::<0>() as u64).saturating_mul(65_536),
            "bytes",
        ),
        sample(
            "memory.stable_extent".into(),
            crate::memory::stable_extent_bytes(),
            "bytes",
        ),
    ]
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) const fn memory() -> Vec<PublicMetricSample> {
    Vec::new()
}

fn sample(name: String, value: u64, unit: &str) -> PublicMetricSample {
    PublicMetricSample {
        name,
        canister_id: None,
        value: u128::from(value),
        unit: unit.into(),
        observed_at_ns: 0,
        kind: PublicMetricKind::Gauge,
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::runtime::metrics::{
        intent::*, placement_index::*, platform_call::*, wasm_store::*,
    };

    #[test]
    fn public_process_rows_expose_only_bounded_outcome_aggregates() {
        PlatformCallMetrics::reset();
        IntentMetrics::reset();
        PlacementIndexMetrics::reset();
        WasmStoreMetrics::reset();
        SystemMetrics::reset();
        PlatformCallMetrics::record(
            PlatformCallMetricSurface::Generic,
            PlatformCallMetricMode::BoundedWait,
            PlatformCallMetricOutcome::Started,
            PlatformCallMetricReason::Ok,
        );
        IntentMetrics::record(
            IntentMetricSurface::Local,
            IntentMetricOperation::Reserve,
            IntentMetricOutcome::Completed,
            IntentMetricReason::Ok,
        );
        PlacementIndexMetrics::record(
            PlacementIndexMetricOperation::Resolve,
            PlacementIndexMetricOutcome::Started,
            PlacementIndexMetricReason::Ok,
        );
        WasmStoreMetrics::record(
            WasmStoreMetricOperation::SourceResolve,
            WasmStoreMetricSource::Bootstrap,
            WasmStoreMetricOutcome::Completed,
            WasmStoreMetricReason::Ok,
        );
        SystemMetrics::increment(SystemMetricKind::CanisterCall);
        let rows = process_rows().unwrap();
        for name in [
            "process.platform_call.started",
            "process.intent.completed",
            "process.placement_index.started",
            "process.wasm_store.completed",
            "process.inter_canister_call.started",
        ] {
            let row = rows.iter().find(|row| row.name == name).unwrap();
            assert_eq!(row.value, 1);
            assert_eq!(row.unit, "count");
            assert_eq!(row.canister_id, None);
        }
        assert!(PlatformCallMetrics::bounded_snapshot(0).is_err());
        assert!(IntentMetrics::bounded_snapshot(0).is_err());
        assert!(PlacementIndexMetrics::bounded_snapshot(0).is_err());
        assert!(WasmStoreMetrics::bounded_snapshot(0).is_err());
        assert!(PlatformCallMetrics::bounded_snapshot(1).is_ok());
    }
}
