use crate::{
    Error,
    cdk::types::Principal,
    dto::placement::sharding::{ShardingPlanStateView, ShardingRegistryView, ShardingTenantsView},
    workflow::placement::sharding::{ShardingWorkflow, query::ShardingQuery},
};

///
/// ShardingApi
///
/// Public API façade for shard placement and inspection.
///
/// Responsibilities:
/// - Expose read-only sharding queries
/// - Expose sharding workflows (assignment / planning)
/// - Normalize internal `InternalError` into `Error`
///
/// Does not:
/// - Contain business logic
/// - Interpret policies
/// - Access storage directly
///
pub struct ShardingApi;

impl ShardingApi {
    // ───────────────────────── Queries ─────────────────────────

    /// Lookup the shard assigned to a tenant in a pool, if any.
    #[must_use]
    pub fn lookup_tenant(pool: &str, tenant: &str) -> Option<Principal> {
        ShardingQuery::lookup_tenant(pool, tenant)
    }

    /// Return the shard for a tenant, or an Error if unassigned.
    pub fn require_tenant_shard(pool: &str, tenant: impl AsRef<str>) -> Result<Principal, Error> {
        ShardingQuery::require_tenant_shard(pool, tenant.as_ref()).map_err(Error::from)
    }

    /// Return a view of the full sharding registry.
    #[must_use]
    pub fn registry_view() -> ShardingRegistryView {
        ShardingQuery::registry_view()
    }

    /// Return all tenants currently assigned to a shard.
    #[must_use]
    pub fn tenants_view(pool: &str, shard: Principal) -> ShardingTenantsView {
        ShardingQuery::tenants_view(pool, shard)
    }

    // ─────────────────────── Workflows ────────────────────────

    /// Assign a tenant to a shard in the given pool.
    ///
    /// This performs validation, selection, and persistence.
    pub async fn assign_to_pool(pool: &str, tenant: impl AsRef<str>) -> Result<Principal, Error> {
        ShardingWorkflow::assign_to_pool(pool, tenant)
            .await
            .map_err(Error::from)
    }

    /// Perform a dry-run shard assignment and return the resulting plan.
    pub fn plan_assign_to_pool(
        pool: &str,
        tenant: impl AsRef<str>,
    ) -> Result<ShardingPlanStateView, Error> {
        ShardingWorkflow::plan_assign_to_pool(pool, tenant).map_err(Error::from)
    }
}
