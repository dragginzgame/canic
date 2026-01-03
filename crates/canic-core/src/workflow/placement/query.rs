use crate::{
    cdk::types::Principal,
    dto::placement::{
        ScalingRegistryEntryView, ScalingRegistryView, ShardingRegistryEntryView,
        ShardingRegistryView, ShardingTenantsView,
    },
    ops::storage::placement::{scaling::ScalingRegistryOps, sharding::ShardingRegistryOps},
    workflow::placement::mapper::PlacementMapper,
};

pub fn scaling_registry_view() -> ScalingRegistryView {
    let data = ScalingRegistryOps::export();

    let view = data
        .entries
        .into_iter()
        .map(|(pid, entry)| ScalingRegistryEntryView {
            pid,
            entry: PlacementMapper::worker_entry_to_view(&entry),
        })
        .collect();

    ScalingRegistryView(view)
}

pub fn sharding_registry_view() -> ShardingRegistryView {
    let data = ShardingRegistryOps::export();

    let view = data
        .entries
        .into_iter()
        .map(|(pid, entry)| ShardingRegistryEntryView {
            pid,
            entry: PlacementMapper::shard_entry_to_view(&entry),
        })
        .collect();

    ShardingRegistryView(view)
}

pub fn sharding_tenants_view(pool: &str, shard: Principal) -> ShardingTenantsView {
    let tenants = ShardingRegistryOps::tenants_in_shard(pool, shard);

    ShardingTenantsView(tenants)
}
