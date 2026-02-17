use crate::{
    InternalError,
    dto::placement::sharding::{
        ShardingPartitionKeysResponse, ShardingRegistryEntry, ShardingRegistryResponse,
    },
    ops::{
        placement::sharding::mapper::ShardEntryMapper,
        storage::placement::sharding::ShardingRegistryOps,
    },
    workflow::prelude::*,
};

///
/// ShardingQuery
/// Public, read-only query APIs for shard placement and inspection.
///

pub struct ShardingQuery;

impl ShardingQuery {
    /// Lookup the shard assigned to a partition_key in a pool, if any.
    #[must_use]
    pub fn lookup_partition_key(pool: &str, partition_key: &str) -> Option<Principal> {
        ShardingRegistryOps::partition_key_shard(pool, partition_key)
    }

    /// Return the shard assigned to a partition_key in a pool, or an error if unassigned.
    pub fn resolve_shard_for_key(
        pool: &str,
        partition_key: &str,
    ) -> Result<Principal, InternalError> {
        ShardingRegistryOps::partition_key_shard_required(pool, partition_key)
    }

    /// Return a view of the full sharding registry.
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

    /// Return all partition_keys currently assigned to a shard.
    #[must_use]
    pub fn partition_keys(pool: &str, shard: Principal) -> ShardingPartitionKeysResponse {
        let partition_keys = ShardingRegistryOps::partition_keys_in_shard(pool, shard);

        ShardingPartitionKeysResponse(partition_keys)
    }
}
