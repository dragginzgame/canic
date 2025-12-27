use crate::{
    cdk::types::Principal, dto::placement::WorkerEntryView, model::memory::scaling::ScalingRegistry,
};

///
/// ScalingRegistryOps
/// Stable storage wrapper for the scaling worker registry.
///

pub struct ScalingRegistryOps;

impl ScalingRegistryOps {
    pub fn insert(pid: Principal, entry: WorkerEntryView) {
        ScalingRegistry::insert(pid, entry);
    }

    #[must_use]
    pub fn find_by_pool(pool: &str) -> Vec<(Principal, WorkerEntryView)> {
        ScalingRegistry::find_by_pool(pool)
    }

    #[must_use]
    pub fn export() -> ScalingRegistryView {
        ScalingRegistry::export()
    }
}
