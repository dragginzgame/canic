//! Scaling workflow.
//!
//! This module performs scaling side effects:
//! - evaluates scaling policy
//! - creates canisters
//! - mutates the scaling registry
//!
//! All async and IC interactions live here.

pub mod mapper;
pub mod query;

use crate::{
    Error, ThisError,
    domain::policy::placement::scaling::{ScalingPlan, ScalingPolicy},
    dto::rpc::CreateCanisterParent,
    ops::{
        config::ConfigOps,
        ic::now_secs,
        rpc::request::RequestOps,
        storage::placement::scaling::{ScalingRegistryOps, WorkerEntry},
    },
    workflow::{placement::PlacementWorkflowError, prelude::*},
};

///
/// ScalingWorkflowError
///
/// Errors raised during scaling execution
///

#[derive(Debug, ThisError)]
pub enum ScalingWorkflowError {
    #[error("scaling plan rejected: {0}")]
    PlanRejected(String),
}

impl From<ScalingWorkflowError> for Error {
    fn from(err: ScalingWorkflowError) -> Self {
        PlacementWorkflowError::Scaling(err).into()
    }
}

///
/// ScalingWorkflow
///

pub struct ScalingWorkflow;

impl ScalingWorkflow {
    /// Create a new worker canister in the given pool, if policy allows.
    pub(crate) async fn create_worker(pool: &str) -> Result<Principal, Error> {
        // 0. Observe state (workflow responsibility)
        let worker_count = ScalingRegistryOps::count_by_pool(pool);
        let scaling = ConfigOps::current_scaling_config()?;

        // 1. Evaluate policy
        let ScalingPlan {
            should_spawn,
            reason,
            worker_entry,
        } = ScalingPolicy::plan_create_worker(pool, worker_count, scaling)?;

        if !should_spawn {
            return Err(ScalingWorkflowError::PlanRejected(reason))?;
        }

        let entry_plan = worker_entry.ok_or_else(|| {
            ScalingWorkflowError::PlanRejected("worker entry missing for spawn plan".to_string())
        })?;

        let role = entry_plan.canister_role.clone();

        // 3. Create the canister
        let pid =
            RequestOps::create_canister::<()>(&role, CreateCanisterParent::ThisCanister, None)
                .await?
                .new_canister_pid;

        // 4. Register in memory
        let entry = WorkerEntry {
            pool: entry_plan.pool,
            canister_role: entry_plan.canister_role,
            created_at_secs: now_secs(),
        };
        ScalingRegistryOps::upsert(pid, entry);

        Ok(pid)
    }

    /// Plan whether a worker should be created according to policy.
    pub(crate) fn plan_create_worker(pool: &str) -> Result<bool, Error> {
        // 0. Observe state (workflow responsibility)
        let worker_count = ScalingRegistryOps::count_by_pool(pool);

        let scaling = ConfigOps::current_scaling_config()?;
        let plan = ScalingPolicy::plan_create_worker(pool, worker_count, scaling)?;

        Ok(plan.should_spawn)
    }
}
