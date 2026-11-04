//! Publish anonymous numeric Storage diagnostics without exposing allocation names.

#[cfg(test)]
mod tests;

use crate::{
    InternalError,
    domain::public_metrics::{PublicMetricFamily, PublicMetricKind},
    model::public_metrics::{PublicMetricSample, PublicMetricsCache},
    ops::runtime::{env::EnvOps, memory::MemoryRegistryOps, public_metrics::PublicMetricsOps},
};
use canic_contracts::ids::CanisterRole;
use ic_storage::StorageDiagnostics;

const PREFIX: &str = "memory.allocations.";
const STATE: &str = "memory.allocations.state";

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
    read: impl FnOnce() -> Result<StorageDiagnostics, InternalError>,
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

const fn invalid() -> InternalError {
    InternalError::public(crate::diagnostics::codes::STATE_INVALID)
}

fn project(
    report: &StorageDiagnostics,
    now: u64,
) -> Result<Vec<PublicMetricSample>, InternalError> {
    let accounted = report
        .data_pages
        .checked_add(report.metadata_pages)
        .and_then(|n| n.checked_add(report.unassigned_pages))
        .and_then(|n| n.checked_add(1))
        .ok_or_else(invalid)?;
    if accounted != report.physical_pages
        || report.admitted_allocations > report.allocations
        || report.live_leases > report.allocations
    {
        return Err(invalid());
    }
    let mut rows = vec![state(AllocationSampleState::Available, now)];
    for (name, pages) in [
        ("physical_extent", report.physical_pages),
        ("data_capacity", report.data_pages),
        ("directory_capacity", report.metadata_pages),
        ("unassigned_capacity", report.unassigned_pages),
    ] {
        rows.push(gauge(
            name,
            pages
                .checked_mul(ic_storage::WASM_PAGE_SIZE_BYTES)
                .ok_or_else(invalid)?,
            "bytes",
            now,
        ));
    }
    rows.push(gauge(
        "directory_bytes",
        report.directory_bytes,
        "bytes",
        now,
    ));
    rows.push(gauge("generation", report.generation, "count", now));
    for (name, count) in [
        ("allocations", report.allocations),
        ("admitted_allocations", report.admitted_allocations),
        ("live_leases", report.live_leases),
        ("extents", report.extents),
    ] {
        rows.push(gauge(
            name,
            u64::try_from(count).map_err(|_| invalid())?,
            "count",
            now,
        ));
    }
    Ok(rows)
}
