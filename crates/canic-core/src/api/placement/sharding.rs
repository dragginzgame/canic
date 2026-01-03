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

pub fn canic_sharding_registry() -> Result<ShardingRegistryView, PublicError> {
    Ok(query::sharding_registry_view())
}

pub fn canic_sharding_tenants(
    pool: String,
    shard_pid: Principal,
) -> Result<ShardingTenantsView, PublicError> {
    Ok(query::sharding_tenants_view(&pool, shard_pid))
}
