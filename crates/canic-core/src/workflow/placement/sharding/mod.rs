//! Module: workflow::placement::sharding
//!
//! Responsibility: orchestrate shard assignment, lifecycle admission, and pool lookup.
//! Does not own: storage schemas, placement policy decisions, or endpoint DTOs.
//! Boundary: coordinates sharding policy and storage ops for workflow callers.

mod allocation;
mod assignment;
mod bootstrap;
pub mod query;
mod registry;
mod release;

use crate::{
    InternalError, InternalErrorOrigin, config::schema::ShardPool,
    domain::policy::pure::placement::sharding::ShardingPolicyError,
    model::placement::sharding::CreateBlockedReason, ops::config::ConfigOps,
};
use thiserror::Error as ThisError;

///
/// ShardingWorkflowError
///
/// Workflow-level failures raised while coordinating sharding placement.
///
#[derive(Debug, ThisError)]
pub enum ShardingWorkflowError {
    #[error(transparent)]
    Policy(#[from] ShardingPolicyError),

    #[error("invariant violation: {0}")]
    Invariant(&'static str),
}

impl From<ShardingWorkflowError> for InternalError {
    fn from(err: ShardingWorkflowError) -> Self {
        match err {
            ShardingWorkflowError::Policy(err) => {
                Self::domain(InternalErrorOrigin::Domain, err.to_string())
            }
            ShardingWorkflowError::Invariant(msg) => {
                Self::invariant(InternalErrorOrigin::Workflow, msg)
            }
        }
    }
}

///
/// ShardingWorkflow
///
/// Entry point for sharding placement orchestration.
///
pub struct ShardingWorkflow;

impl ShardingWorkflow {
    pub(super) fn blocked(
        reason: CreateBlockedReason,
        pool: &str,
        partition_key: &str,
    ) -> InternalError {
        ShardingWorkflowError::Policy(ShardingPolicyError::ShardCreationBlocked {
            reason,
            partition_key: partition_key.to_string(),
            pool: pool.to_string(),
        })
        .into()
    }

    pub(super) fn get_shard_pool_cfg(pool: &str) -> Result<ShardPool, InternalError> {
        let sharding = ConfigOps::current_canister()?
            .sharding
            .ok_or(ShardingPolicyError::ShardingDisabled)?;
        let available = if sharding.pools.is_empty() {
            "<none>".to_string()
        } else {
            let mut names: Vec<_> = sharding.pools.keys().cloned().collect();
            names.sort_unstable();
            names.join(", ")
        };

        sharding
            .pools
            .get(pool)
            .cloned()
            .ok_or_else(|| ShardingPolicyError::PoolNotFound {
                requested: pool.to_string(),
                available,
            })
            .map_err(InternalError::from)
    }
}
