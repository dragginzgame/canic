use crate::{
    InternalError,
    dto::placement::sharding::{
        ShardingRegistryEntry, ShardingRegistryResponse, ShardingTenantsResponse,
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
    /// Lookup the shard assigned to a tenant in a pool, if any.
    #[must_use]
    pub fn lookup_tenant(pool: &str, tenant: &str) -> Option<Principal> {
        ShardingRegistryOps::tenant_shard(pool, tenant)
    }

    /// Return the shard assigned to a tenant in a pool, or an error if unassigned.
    pub fn require_tenant_shard(pool: &str, tenant: &str) -> Result<Principal, InternalError> {
        ShardingRegistryOps::tenant_shard_required(pool, tenant)
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

    /// Return all tenants currently assigned to a shard.
    #[must_use]
    pub fn tenants(pool: &str, shard: Principal) -> ShardingTenantsResponse {
        let tenants = ShardingRegistryOps::tenants_in_shard(pool, shard);

        ShardingTenantsResponse(tenants)
    }
}
