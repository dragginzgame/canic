use crate::{
    dto::placement::{ScalingRegistryView, ShardingRegistryView},
    ops::storage::placement::{scaling::ScalingRegistryOps, sharding::ShardingRegistryOps},
    workflow::placement::mapper::PlacementMapper,
};

pub(crate) fn scaling_registry_view() -> ScalingRegistryView {
    let data = ScalingRegistryOps::export();

    let view = data
        .entries
        .into_iter()
        .map(|(pid, entry)| (pid, PlacementMapper::worker_entry_to_view(&entry)))
        .collect();

    ScalingRegistryView(view)
}

pub(crate) fn sharding_registry_view() -> ShardingRegistryView {
    let data = ShardingRegistryOps::export();

    let view = data
        .entries
        .into_iter()
        .map(|(pid, entry)| (pid, PlacementMapper::shard_entry_to_view(&entry)))
        .collect();

    ShardingRegistryView(view)
}
