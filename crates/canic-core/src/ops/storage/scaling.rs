use crate::{
    cdk::types::Principal,
    model::memory::scaling::{ScalingRegistry, ScalingRegistryData, WorkerEntry},
};

///
/// ScalingRegistryOps
/// Stable storage wrapper for the scaling worker registry.
///

pub struct ScalingRegistryOps;

impl ScalingRegistryOps {
    pub fn insert(pid: Principal, entry: WorkerEntry) {
        ScalingRegistry::insert(pid, entry);
    }

    #[must_use]
    pub fn find_by_pool(pool: &str) -> Vec<(Principal, WorkerEntry)> {
        ScalingRegistry::find_by_pool(pool)
    }

    #[must_use]
    pub fn export() -> ScalingRegistryData {
        ScalingRegistry::export()
    }
}
