use crate::mapper::ShardEntryMapper;
use canic_core::{
    __sharding_core as sharding_core,
    cdk::types::Principal,
    dto::placement::sharding::{
        ShardingPartitionKeysResponse, ShardingRegistryEntry, ShardingRegistryResponse,
    },
    error::InternalError,
};
use sharding_core::ops::storage::placement::sharding::ShardingRegistryOps;

pub struct ShardingQuery;

impl ShardingQuery {
    #[must_use]
    pub fn lookup_partition_key(pool: &str, partition_key: &str) -> Option<Principal> {
        ShardingRegistryOps::partition_key_shard(pool, partition_key)
    }

    pub fn resolve_shard_for_key(
        pool: &str,
        partition_key: &str,
    ) -> Result<Principal, InternalError> {
        ShardingRegistryOps::partition_key_shard_required(pool, partition_key)
    }

    #[must_use]
    pub fn registry() -> ShardingRegistryResponse {
        let data = ShardingRegistryOps::export();

        let view = data
            .entries
            .into_iter()
            .map(|(pid, entry)| ShardingRegistryEntry {
                pid,
                entry: ShardEntryMapper::record_to_view(&entry),
            })
            .collect();

        ShardingRegistryResponse(view)
    }

    #[must_use]
    pub fn partition_keys(pool: &str, shard: Principal) -> ShardingPartitionKeysResponse {
        let partition_keys = ShardingRegistryOps::partition_keys_in_shard(pool, shard);
        ShardingPartitionKeysResponse(partition_keys)
    }
}
