use crate::{
    cdk::types::Principal,
    storage::memory::scaling::{ScalingRegistry, ScalingRegistryData, WorkerEntry},
};

///
/// ScalingRegistryOps
/// Stable storage wrapper for the scaling worker registry.
///

pub struct ScalingRegistryOps;

impl ScalingRegistryOps {
    pub(crate) fn upsert(pid: Principal, entry: WorkerEntry) {
        ScalingRegistry::upsert(pid, entry);
    }

    /// Lookup all workers in a given pool
    #[must_use]
    pub(crate) fn find_by_pool(pool: &str) -> Vec<(Principal, WorkerEntry)> {
        ScalingRegistry::export()
            .entries
            .into_iter()
            .filter(|(_, entry)| entry.pool.as_ref() == pool)
            .collect()
    }

    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub(crate) fn count_by_pool(pool: &str) -> u32 {
        Self::find_by_pool(pool).len() as u32
    }

    #[must_use]
    pub fn export() -> ScalingRegistryData {
        ScalingRegistry::export()
    }
}
