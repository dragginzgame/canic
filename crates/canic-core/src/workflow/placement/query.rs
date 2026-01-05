use crate::{
    dto::placement::{
        ScalingRegistryEntryView, ScalingRegistryView, ShardingRegistryEntryView,
        ShardingRegistryView, ShardingTenantsView,
    },
    ops::storage::placement::{scaling::ScalingRegistryOps, sharding::ShardingRegistryOps},
    workflow::{placement::mapper::PlacementMapper, prelude::*},
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
                entry: PlacementMapper::worker_entry_to_view(&entry),
            })
            .collect();

        ScalingRegistryView(view)
    }
}

///
/// ShardingQuery
///

pub struct ShardingQuery;

impl ShardingQuery {
    pub fn registry_view() -> ShardingRegistryView {
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

    pub fn tenants_view(pool: &str, shard: Principal) -> ShardingTenantsView {
        let tenants = ShardingRegistryOps::tenants_in_shard(pool, shard);

        ShardingTenantsView(tenants)
    }
}
