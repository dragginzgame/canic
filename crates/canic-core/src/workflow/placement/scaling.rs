//! Scaling workflow.
//!
//! This module performs scaling side effects:
//! - evaluates scaling policy
//! - creates canisters
//! - mutates the scaling registry
//!
//! All async and IC interactions live here.

use crate::{
    Error, ThisError,
    cdk::utils::time::now_secs,
    dto::rpc::CreateCanisterParent,
    ops::{
        adapter::placement::worker_entry_from_view, rpc::create_canister_request,
        storage::scaling::ScalingRegistryOps,
    },
    policy::placement::scaling::{ScalingPlan, ScalingPolicy},
};
use candid::Principal;

///
/// ScalingWorkflowError
/// Errors raised during scaling execution
///

#[derive(Debug, ThisError)]
pub enum ScalingWorkflowError {
    #[error("scaling plan rejected: {0}")]
    PlanRejected(String),
}

impl From<ScalingWorkflowError> for Error {
    fn from(err: ScalingWorkflowError) -> Self {
        Self::WorkflowError(err.to_string())
    }
}

///
/// ScalingWorkflow
///
pub struct ScalingWorkflow;

impl ScalingWorkflow {
    /// Create a new worker canister in the given pool, if policy allows.
    pub async fn create_worker(pool: &str) -> Result<Principal, Error> {
        // 1. Evaluate policy
        let ScalingPlan {
            should_spawn,
            reason,
            worker_entry,
        } = ScalingPolicy::plan_create_worker(pool, now_secs())?;

        if !should_spawn {
            return Err(ScalingWorkflowError::PlanRejected(reason))?;
        }

        let entry_view = worker_entry.ok_or_else(|| {
            ScalingWorkflowError::PlanRejected("worker entry missing for spawn plan".to_string())
        })?;

        let role = entry_view.canister_role.clone();

        // 3. Create the canister
        let pid = create_canister_request::<()>(&role, CreateCanisterParent::ThisCanister, None)
            .await?
            .new_canister_pid;

        // 4. Register in memory
        let entry = worker_entry_from_view(entry_view);
        ScalingRegistryOps::upsert(pid, entry);

        Ok(pid)
    }
}
