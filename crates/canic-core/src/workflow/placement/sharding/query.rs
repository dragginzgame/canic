use crate::{
    InternalError,
    dto::placement::sharding::{
        ShardingRegistryEntryView, ShardingRegistryView, ShardingTenantsView,
    },
    ops::storage::placement::sharding::ShardingRegistryOps,
    workflow::{placement::sharding::mapper::ShardingMapper, prelude::*},
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
    pub fn registry_view() -> ShardingRegistryView {
        let data = ShardingRegistryOps::export();

        let view = data
            .entries
            .into_iter()
            .map(|(pid, entry)| ShardingRegistryEntryView {
                pid,
                entry: ShardingMapper::shard_entry_to_view(&entry),
            })
            .collect();

        ShardingRegistryView(view)
    }

    /// Return all tenants currently assigned to a shard.
    #[must_use]
    pub fn tenants_view(pool: &str, shard: Principal) -> ShardingTenantsView {
        let tenants = ShardingRegistryOps::tenants_in_shard(pool, shard);

        ShardingTenantsView(tenants)
    }
}
