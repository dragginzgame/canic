use crate::{
    dto::placement::scaling::{ScalingRegistryEntry, ScalingRegistryResponse},
    ops::{placement::scaling::mapper::WorkerEntryRecordMapper, prelude::*},
    storage::stable::scaling::{ScalingRegistry, ScalingRegistryRecord, WorkerEntryRecord},
    view::placement::scaling::ScalingWorkerPlanEntry,
};

///
/// ScalingRegistryOps
/// Stable storage wrapper for the scaling worker registry.
///

pub struct ScalingRegistryOps;

impl ScalingRegistryOps {
    #[expect(dead_code)]
    pub fn upsert(pid: Principal, entry: WorkerEntryRecord) {
        ScalingRegistry::upsert(pid, entry);
    }

    pub fn upsert_from_plan(pid: Principal, plan: ScalingWorkerPlanEntry, created_at_secs: u64) {
        let entry = WorkerEntryRecordMapper::validated_to_record(plan, created_at_secs);
        ScalingRegistry::upsert(pid, entry);
    }

    /// Lookup all workers in a given pool
    #[must_use]
    pub fn find_by_pool(pool: &str) -> Vec<(Principal, WorkerEntryRecord)> {
        ScalingRegistry::export()
            .entries
            .into_iter()
            .filter(|(_, entry)| entry.pool.as_ref() == pool)
            .collect()
    }

    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn count_by_pool(pool: &str) -> u32 {
        Self::find_by_pool(pool).len() as u32
    }

    #[must_use]
    #[expect(dead_code)]
    pub fn export() -> ScalingRegistryRecord {
        ScalingRegistry::export()
    }

    #[must_use]
    pub fn entries_response() -> ScalingRegistryResponse {
        let entries = ScalingRegistry::export()
            .entries
            .into_iter()
            .map(|(pid, entry)| ScalingRegistryEntry {
                pid,
                entry: WorkerEntryRecordMapper::record_to_view(&entry),
            })
            .collect();

        ScalingRegistryResponse(entries)
    }
}
