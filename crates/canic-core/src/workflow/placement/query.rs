use crate::{
    cdk::types::Principal,
    dto::placement::{ScalingRegistryView, ShardingRegistryView, ShardingTenantsView},
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

pub(crate) fn sharding_tenants_view(pool: &str, shard: Principal) -> ShardingTenantsView {
    let tenants = ShardingRegistryOps::tenants_in_shard(pool, shard);

    ShardingTenantsView(tenants)
}
