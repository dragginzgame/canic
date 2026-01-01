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
    domain::policy::placement::scaling::{ScalingPlan, ScalingPolicy, ScalingWorkerPlanEntry},
    dto::{placement::WorkerEntryView, rpc::CreateCanisterParent},
    ops::{rpc::request::create_canister_request, storage::placement::scaling::ScalingRegistryOps},
    workflow::placement::{PlacementError, adapter::worker_entry_from_view},
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
        PlacementError::Scaling(err).into()
    }
}

///
/// ScalingWorkflow
///

pub struct ScalingWorkflow;

impl ScalingWorkflow {
    /// Create a new worker canister in the given pool, if policy allows.
    pub(crate) async fn create_worker(pool: &str) -> Result<Principal, Error> {
        // 1. Evaluate policy
        let ScalingPlan {
            should_spawn,
            reason,
            worker_entry,
        } = ScalingPolicy::plan_create_worker(pool, now_secs())?;

        if !should_spawn {
            return Err(ScalingWorkflowError::PlanRejected(reason))?;
        }

        let entry_plan = worker_entry.ok_or_else(|| {
            ScalingWorkflowError::PlanRejected("worker entry missing for spawn plan".to_string())
        })?;

        let role = entry_plan.canister_role.clone();

        // 3. Create the canister
        let pid = create_canister_request::<()>(&role, CreateCanisterParent::ThisCanister, None)
            .await?
            .new_canister_pid;

        // 4. Register in memory
        let entry = worker_entry_from_view(plan_entry_to_view(entry_plan));
        ScalingRegistryOps::upsert(pid, entry);

        Ok(pid)
    }

    /// Plan whether a worker should be created according to policy.
    pub(crate) fn plan_create_worker(pool: &str) -> Result<bool, Error> {
        let plan = ScalingPolicy::plan_create_worker(pool, now_secs())?;

        Ok(plan.should_spawn)
    }
}

fn plan_entry_to_view(entry: ScalingWorkerPlanEntry) -> WorkerEntryView {
    WorkerEntryView {
        pool: entry.pool,
        canister_role: entry.canister_role,
        created_at_secs: entry.created_at_secs,
    }
}
