use crate::{
    PublicError, cdk::types::Principal, dto::placement::sharding::ShardingPlanStateView,
    workflow::placement::sharding::ShardingWorkflow,
};

// Workflow Query Re-export
pub use crate::workflow::placement::sharding::query::ShardingQuery;

///
/// ShardingApi
///

pub struct ShardingApi;

impl ShardingApi {
    pub async fn assign_to_pool(
        pool: &str,
        tenant: impl AsRef<str>,
    ) -> Result<Principal, PublicError> {
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
}
