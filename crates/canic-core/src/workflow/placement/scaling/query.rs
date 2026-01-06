use crate::{
    dto::placement::scaling::{ScalingRegistryEntryView, ScalingRegistryView},
    ops::storage::placement::scaling::ScalingRegistryOps,
    workflow::placement::scaling::mapper::ScalingMapper,
};

///
/// ScalingQuery
///

pub struct ScalingQuery;

impl ScalingQuery {
    pub fn registry_view() -> ScalingRegistryView {
        let data = ScalingRegistryOps::export();

        let view = data
            .entries
            .into_iter()
            .map(|(pid, entry)| ScalingRegistryEntryView {
                pid,
                entry: ScalingMapper::worker_entry_to_view(&entry),
            })
            .collect();

        ScalingRegistryView(view)
    }
}
