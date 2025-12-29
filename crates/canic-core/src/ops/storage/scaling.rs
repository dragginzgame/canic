use crate::{
    cdk::types::Principal,
    dto::placement::ScalingRegistryView,
    model::memory::scaling::{ScalingRegistry, WorkerEntry},
    ops::adapter::placement::worker_entry_to_view,
};

///
/// ScalingRegistryOps
/// Stable storage wrapper for the scaling worker registry.
///

pub struct ScalingRegistryOps;

impl ScalingRegistryOps {
    pub(crate) fn insert(pid: Principal, entry: WorkerEntry) {
        ScalingRegistry::insert(pid, entry);
    }

    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub(crate) fn count_by_pool(pool: &str) -> u32 {
        ScalingRegistry::find_by_pool(pool).len() as u32
    }

    #[must_use]
    pub fn export_view() -> ScalingRegistryView {
        let data = ScalingRegistry::export();

        data.into_iter()
            .map(|(pid, entry)| (pid, worker_entry_to_view(&entry)))
            .collect()
    }
}
