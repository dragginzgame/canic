//! Module: ops::storage::placement::scaling
//!
//! Responsibility: provide deterministic access to scaling worker registry records.
//! Does not own: scaling policy, worker orchestration, or endpoint DTOs.
//! Boundary: storage ops facade over stable scaling registry records.

use crate::{
    dto::placement::scaling::{ScalingRegistryEntry, ScalingRegistryResponse},
    model::placement::scaling::ScalingWorkerEntry,
    ops::{placement::scaling::mapper::WorkerEntryRecordMapper, prelude::*},
    storage::stable::scaling::ScalingRegistry,
};

///
/// ScalingRegistryOps
///
/// Storage-ops facade for the scaling worker registry.
///

pub struct ScalingRegistryOps;

impl ScalingRegistryOps {
    pub fn upsert(pid: Principal, worker: ScalingWorkerEntry, created_at_secs: u64) {
        let entry = WorkerEntryRecordMapper::validated_to_record(worker, created_at_secs);
        ScalingRegistry::upsert(pid, entry);
    }

    #[must_use]
    pub fn count_by_pool(pool: &str) -> u32 {
        ScalingRegistry::count_by_pool(pool)
    }

    #[must_use]
    pub fn entries_response() -> ScalingRegistryResponse {
        let entries = ScalingRegistry::export()
            .entries
            .into_iter()
            .map(|record| ScalingRegistryEntry {
                pid: record.pid,
                entry: WorkerEntryRecordMapper::record_to_view(&record.entry),
            })
            .collect();

        ScalingRegistryResponse(entries)
    }
}
