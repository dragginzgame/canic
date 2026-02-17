use crate::{
    cdk::types::Principal,
    dto::{
        error::Error,
        placement::sharding::{
            ShardingPartitionKeysResponse, ShardingPlanStateResponse, ShardingRegistryResponse,
        },
    },
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

    /// Lookup the shard assigned to a partition_key in a pool, if any.
    #[must_use]
    pub fn lookup_partition_key(pool: &str, partition_key: &str) -> Option<Principal> {
        ShardingQuery::lookup_partition_key(pool, partition_key)
    }

    /// Return the shard for a partition_key, or an Error if unassigned.
    pub fn resolve_shard_for_key(
        pool: &str,
        partition_key: impl AsRef<str>,
    ) -> Result<Principal, Error> {
        ShardingQuery::resolve_shard_for_key(pool, partition_key.as_ref()).map_err(Error::from)
    }

    /// Return a view of the full sharding registry.
    #[must_use]
    pub fn registry() -> ShardingRegistryResponse {
        ShardingQuery::registry()
    }

    /// Return all partition_keys currently assigned to a shard.
    #[must_use]
    pub fn partition_keys(pool: &str, shard: Principal) -> ShardingPartitionKeysResponse {
        ShardingQuery::partition_keys(pool, shard)
    }

    // ─────────────────────── Workflows ────────────────────────

    /// Assign a partition_key to a shard in the given pool.
    ///
    /// This performs validation, selection, and persistence.
    pub async fn assign_to_pool(
        pool: &str,
        partition_key: impl AsRef<str>,
    ) -> Result<Principal, Error> {
        ShardingWorkflow::assign_to_pool(pool, partition_key)
            .await
            .map_err(Error::from)
    }

    /// Perform a dry-run shard assignment and return the resulting plan.
    pub fn plan_assign_to_pool(
        pool: &str,
        partition_key: impl AsRef<str>,
    ) -> Result<ShardingPlanStateResponse, Error> {
        ShardingWorkflow::plan_assign_to_pool(pool, partition_key).map_err(Error::from)
    }
}
