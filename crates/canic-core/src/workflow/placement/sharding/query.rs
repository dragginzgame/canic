//! Module: workflow::placement::sharding::query
//!
//! Responsibility: expose read-only sharding registry query projections.
//! Does not own: registry mutation, assignment policy, or endpoint authorization.
//! Boundary: maps storage records into sharding DTO responses.

use crate::{
    InternalError,
    cdk::types::Principal,
    dto::placement::sharding::{
        ShardingPartitionKeysResponse, ShardingRegistryEntry, ShardingRegistryResponse,
    },
    ops::{
        placement::sharding::mapper::ShardEntryMapper,
        storage::placement::sharding::ShardingRegistryOps,
    },
};

///
/// ShardingQuery
///
/// Read-only query facade for sharding registry state.
///
pub struct ShardingQuery;

impl ShardingQuery {
    #[must_use]
    pub fn lookup_partition_key(pool: &str, partition_key: &str) -> Option<Principal> {
        ShardingRegistryOps::assignment_for_key(pool, partition_key)
            .filter(ShardingRegistryOps::assignment_is_current)
            .map(|record| record.shard)
    }

    pub fn resolve_shard_for_key(
        pool: &str,
        partition_key: &str,
    ) -> Result<Principal, InternalError> {
        Self::lookup_partition_key(pool, partition_key)
            .ok_or_else(|| InternalError::public(crate::diagnostics::codes::POSITION_UNAVAILABLE))
    }

    #[must_use]
    pub fn registry() -> ShardingRegistryResponse {
        let data = ShardingRegistryOps::registry_data();

        let view = data
            .entries
            .into_iter()
            .filter(ShardingRegistryOps::entry_is_current)
            .map(|record| ShardingRegistryEntry {
                pid: record.pid,
                entry: ShardEntryMapper::record_to_view(&record.entry),
            })
            .collect();

        ShardingRegistryResponse(view)
    }

    #[must_use]
    pub fn partition_keys(pool: &str, shard: Principal) -> ShardingPartitionKeysResponse {
        let partition_keys = ShardingRegistryOps::partition_keys_in_shard(pool, shard)
            .into_iter()
            .filter(|key| Self::lookup_partition_key(pool, key) == Some(shard))
            .collect();
        ShardingPartitionKeysResponse(partition_keys)
    }
}
