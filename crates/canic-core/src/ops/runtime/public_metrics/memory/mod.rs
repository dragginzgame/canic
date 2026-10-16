//! Module: ops::runtime::public_metrics::memory
//!
//! Responsibility: publish bounded anonymous allocation gauges from the memory owner.
//! Does not own: diagnostics access, memory initialization, stores, timers or history.
//! Boundary: fixed labels only; failed collection preserves original successful source times.

#[cfg(test)]
mod tests;

use crate::{
    InternalError,
    domain::public_metrics::{PublicMetricFamily, PublicMetricKind},
    model::public_metrics::{PublicMetricSample, PublicMetricsCache},
    ops::runtime::{env::EnvOps, memory::MemoryRegistryOps, public_metrics::PublicMetricsOps},
};
use canic_contracts::ids::CanisterRole;
use ic_memory::MemoryAllocationSummary;

const PREFIX: &str = "memory.allocations.";
const STATE: &str = "memory.allocations.state";

/// Numeric state gauge; absence means no allocation sample has been published.
#[derive(Clone, Copy)]
enum AllocationSampleState {
    Available = 1,
    Unsupported = 2,
    Failed = 3,
}

pub(super) fn sample(now: u64) -> Vec<PublicMetricSample> {
    collect(
        PublicMetricsOps::enabled().contains(&PublicMetricFamily::Performance),
        EnvOps::canister_role().is_ok_and(|role| role != CanisterRole::WASM_STORE),
        now,
        MemoryRegistryOps::allocation_summary,
    )
}

fn collect(
    enabled: bool,
    supported: bool,
    now: u64,
    read: impl FnOnce() -> Result<MemoryAllocationSummary, InternalError>,
) -> Vec<PublicMetricSample> {
    if !enabled {
        return Vec::new();
    }
    if !supported {
        return vec![state(AllocationSampleState::Unsupported, now)];
    }
    if let Ok(rows) = read().and_then(|report| project(&report, now)) {
        return rows;
    }
    let mut rows = vec![state(AllocationSampleState::Failed, now)];
    if let Some(previous) = PublicMetricsCache::snapshot(PublicMetricFamily::Performance) {
        rows.extend(
            previous
                .metrics
                .into_iter()
                .filter(|row| row.name.starts_with(PREFIX) && row.name != STATE),
        );
    }
    rows
}

fn state(value: AllocationSampleState, now: u64) -> PublicMetricSample {
    gauge("state", value as u64, "state", now)
}

fn gauge(name: &str, value: u64, unit: &str, now: u64) -> PublicMetricSample {
    PublicMetricSample {
        name: format!("{PREFIX}{name}"),
        canister_id: None,
        value: u128::from(value),
        unit: unit.into(),
        observed_at_ns: now,
        kind: PublicMetricKind::Gauge,
    }
}

fn project(
    report: &MemoryAllocationSummary,
    now: u64,
) -> Result<Vec<PublicMetricSample>, InternalError> {
    let groups = validate(report)?;
    let mut rows = vec![state(AllocationSampleState::Available, now)];
    for (name, value) in [
        ("physical_extent", report.physical_extent.bytes()),
        ("bucket_size", report.bucket_size_bytes()),
        ("allocated_bucket_bytes", report.allocated_bucket_bytes()),
        ("virtual_extent", report.virtual_extent.bytes()),
        ("bucket_slack", report.bucket_slack_bytes()),
        (
            "manager_metadata",
            ic_memory::MemoryAllocationSummary::MANAGER_METADATA_BYTES,
        ),
        ("known_binding", report.known_binding_bytes()),
        ("unknown_binding", report.unknown_binding_bytes()),
        ("unmanaged", report.unmanaged_bytes()),
        (
            "metadata_read",
            ic_memory::MemoryAllocationSummary::METADATA_BYTES_READ,
        ),
    ] {
        rows.push(gauge(name, value, "bytes", now));
    }
    for (name, value) in [
        ("allocated_buckets", u64::from(report.allocated_buckets)),
        (
            "bucket_capacity",
            u64::from(ic_memory::MemoryAllocationSummary::BUCKET_CAPACITY),
        ),
        ("remaining_buckets", u64::from(report.remaining_buckets())),
        (
            "ids_measured",
            u64::from(ic_memory::MemoryAllocationSummary::MEMORIES_MEASURED),
        ),
        ("ids_total", u64::from(ic_memory::MEMORY_MANAGER_INVALID_ID)),
        ("payload_available", 0),
    ] {
        rows.push(gauge(name, value, "count", now));
    }
    // Binding provenance is authoritative. Owner strings and range claims never become labels.
    for (group, (allocated, slack)) in ["current_binding", "ledger_binding", "unknown_binding"]
        .into_iter()
        .zip(groups)
    {
        rows.push(gauge(
            &format!("{group}.allocated"),
            allocated,
            "bytes",
            now,
        ));
        rows.push(gauge(&format!("{group}.slack"), slack, "bytes", now));
    }
    Ok(rows)
}

const fn invalid() -> InternalError {
    InternalError::public(crate::diagnostics::codes::STATE_INVALID)
}

fn sum(a: u64, b: u64) -> Result<u64, InternalError> {
    a.checked_add(b).ok_or_else(invalid)
}

/// Independently verify physical, capacity and binding partitions before publishing them.
fn validate(report: &MemoryAllocationSummary) -> Result<[(u64, u64); 3], InternalError> {
    if report.current_generation.is_none()
        || report.bucket_size_pages == 0
        || u32::from(report.allocated_buckets) > MemoryAllocationSummary::BUCKET_CAPACITY
    {
        return Err(invalid());
    }
    let bindings = [
        report.current_binding,
        report.ledger_binding,
        report.unknown_binding,
    ];
    let mut groups = [(0, 0); 3];
    let mut buckets = 0;
    let mut virtual_pages = 0;
    for (binding, group) in bindings.into_iter().zip(&mut groups) {
        buckets = sum(buckets, u64::from(binding.allocated_buckets))?;
        virtual_pages = sum(virtual_pages, binding.virtual_extent.wasm_pages)?;
        let allocated = binding.allocated_bytes(report.bucket_size_pages);
        let virtual_bytes = binding
            .virtual_extent
            .wasm_pages
            .checked_mul(ic_memory::WASM_PAGE_SIZE_BYTES)
            .ok_or_else(invalid)?;
        *group = (
            allocated,
            allocated.checked_sub(virtual_bytes).ok_or_else(invalid)?,
        );
    }
    let physical_bytes = report
        .physical_extent
        .wasm_pages
        .checked_mul(ic_memory::WASM_PAGE_SIZE_BYTES)
        .ok_or_else(invalid)?;
    let accounted = sum(
        MemoryAllocationSummary::MANAGER_METADATA_BYTES,
        report.allocated_bucket_bytes(),
    )?;
    if buckets != u64::from(report.allocated_buckets)
        || virtual_pages != report.virtual_extent.wasm_pages
        || physical_bytes < accounted
    {
        return Err(invalid());
    }
    Ok(groups)
}
