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
    ids::CanisterRole,
    model::public_metrics::{PublicMetricSample, PublicMetricsCache},
    ops::runtime::{env::EnvOps, memory::MemoryRegistryOps, public_metrics::PublicMetricsOps},
};
use ic_memory::{AllocationBinding, MemoryAllocations};

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
        MemoryRegistryOps::allocation_report,
    )
}

fn collect(
    enabled: bool,
    supported: bool,
    now: u64,
    read: impl FnOnce() -> Result<MemoryAllocations, InternalError>,
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

fn project(report: &MemoryAllocations, now: u64) -> Result<Vec<PublicMetricSample>, InternalError> {
    let groups = validate(report)?;
    let mut rows = vec![state(AllocationSampleState::Available, now)];
    for (name, value) in [
        ("physical_extent", report.physical_extent.bytes),
        ("bucket_size", report.bucket_size_bytes),
        ("allocated_bucket_bytes", report.allocated_bucket_bytes),
        ("virtual_extent", report.virtual_extent.bytes),
        ("bucket_slack", report.bucket_slack_bytes),
        ("manager_metadata", report.manager_metadata_bytes),
        ("known_binding", report.known_binding_bytes),
        ("unknown_binding", report.unknown_binding_bytes),
        ("unmanaged", report.unmanaged_bytes),
        ("metadata_read", report.metadata_bytes_read),
    ] {
        rows.push(gauge(name, value, "bytes", now));
    }
    for (name, value) in [
        ("allocated_buckets", u64::from(report.allocated_buckets)),
        ("bucket_capacity", u64::from(report.bucket_capacity)),
        ("remaining_buckets", u64::from(report.remaining_buckets)),
        ("ids_measured", report.memories.len() as u64),
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
fn validate(report: &MemoryAllocations) -> Result<[(u64, u64); 3], InternalError> {
    if report.current_generation.is_none()
        || report.memories.len() != usize::from(ic_memory::MEMORY_MANAGER_INVALID_ID)
    {
        return Err(invalid());
    }
    let mut groups = [(0, 0); 3];
    let mut virtual_bytes = 0;
    let mut buckets = 0;
    for (id, entry) in report.memories.iter().enumerate() {
        if usize::from(entry.memory_manager_id) != id
            || entry.payload_bytes.is_some()
            || entry.allocated_bytes != sum(entry.virtual_extent.bytes, entry.bucket_slack_bytes)?
        {
            return Err(invalid());
        }
        let index = match entry.binding {
            AllocationBinding::Current { .. } => 0,
            AllocationBinding::Ledger { .. } => 1,
            AllocationBinding::Unknown => 2,
        };
        groups[index].0 = sum(groups[index].0, entry.allocated_bytes)?;
        groups[index].1 = sum(groups[index].1, entry.bucket_slack_bytes)?;
        virtual_bytes = sum(virtual_bytes, entry.virtual_extent.bytes)?;
        buckets = sum(buckets, u64::from(entry.allocated_buckets))?;
    }
    let known = sum(groups[0].0, groups[1].0)?;
    let allocated = sum(known, groups[2].0)?;
    let slack = sum(sum(groups[0].1, groups[1].1)?, groups[2].1)?;
    let physical = sum(
        sum(report.manager_metadata_bytes, allocated)?,
        report.unmanaged_bytes,
    )?;
    let valid_partitions = report.physical_extent.bytes == physical
        && report.allocated_bucket_bytes == allocated
        && report.known_binding_bytes == known
        && report.unknown_binding_bytes == groups[2].0;
    let valid_capacity = report.virtual_extent.bytes == virtual_bytes
        && report.bucket_slack_bytes == slack
        && allocated == sum(virtual_bytes, slack)?
        && u64::from(report.allocated_buckets) == buckets
        && report.bucket_size_bytes == u64::from(report.bucket_size_pages) * 65_536
        && allocated
            == buckets
                .checked_mul(report.bucket_size_bytes)
                .ok_or_else(invalid)?
        && sum(buckets, u64::from(report.remaining_buckets))? == u64::from(report.bucket_capacity);
    if !valid_partitions || !valid_capacity {
        return Err(invalid());
    }
    Ok(groups)
}
