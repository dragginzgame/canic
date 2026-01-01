use crate::{PublicError, cdk::types::Principal, workflow::placement::scaling::ScalingWorkflow};

/// API wrapper that exposes worker creation by delegating to the scaling workflow.
pub async fn create_worker(pool: &str) -> Result<Principal, PublicError> {
    ScalingWorkflow::create_worker(pool)
        .await
        .map_err(PublicError::from)
}

/// API wrapper that exposes the scaling decision (dry-run) via the workflow.
pub fn plan_create_worker(pool: &str) -> Result<bool, PublicError> {
    ScalingWorkflow::plan_create_worker(pool).map_err(PublicError::from)
}
