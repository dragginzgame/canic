use crate::{
    PublicError,
    cdk::types::Principal,
    dto::placement::{ShardingPlanStateView, ShardingRegistryView, ShardingTenantsView},
    workflow::placement::{query, sharding::ShardingWorkflow},
};

pub async fn assign_to_pool(pool: &str, tenant: impl AsRef<str>) -> Result<Principal, PublicError> {
    ShardingWorkflow::assign_to_pool(pool, tenant)
        .await
        .map_err(PublicError::from)
}

pub fn plan_assign_to_pool(
    pool: &str,
    tenant: impl AsRef<str>,
) -> Result<ShardingPlanStateView, PublicError> {
    ShardingWorkflow::plan_assign_to_pool(pool, tenant).map_err(PublicError::from)
}

#[must_use]
pub fn sharding_registry() -> ShardingRegistryView {
    query::sharding_registry_view()
}

#[must_use]
pub fn sharding_tenants(pool: String, shard_pid: Principal) -> ShardingTenantsView {
    query::sharding_tenants_view(&pool, shard_pid)
}
