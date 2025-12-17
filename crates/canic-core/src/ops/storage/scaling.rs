pub use crate::model::memory::scaling::{ScalingRegistryView, WorkerEntry};

use crate::{cdk::types::Principal, model::memory::scaling::ScalingRegistry};

///
/// ScalingWorkerRegistryStorageOps
/// Stable storage wrapper for the scaling worker registry.
///

pub struct ScalingWorkerRegistryStorageOps;

impl ScalingWorkerRegistryStorageOps {
    pub fn insert(pid: Principal, entry: WorkerEntry) {
        ScalingRegistry::insert(pid, entry);
    }

    #[must_use]
    pub fn find_by_pool(pool: &str) -> Vec<(Principal, WorkerEntry)> {
        ScalingRegistry::find_by_pool(pool)
    }

    #[must_use]
    pub fn export() -> ScalingRegistryView {
        ScalingRegistry::export()
    }
}
