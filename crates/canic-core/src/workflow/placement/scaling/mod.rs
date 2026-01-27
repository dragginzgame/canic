//! Scaling workflow.
//!
//! This module performs scaling side effects:
//! - evaluates scaling policy
//! - creates canisters
//! - mutates the scaling registry
//!
//! All async and IC interactions live here.

pub mod query;

use crate::{
    InternalError, InternalErrorOrigin,
    domain::policy::placement::scaling::{ScalingPlan, ScalingPolicy},
    ops::{
        config::ConfigOps,
        ic::IcOps,
        rpc::request::{CreateCanisterParent, RequestOps},
        storage::placement::scaling::ScalingRegistryOps,
    },
    workflow::prelude::*,
};

///
/// ScalingWorkflow
///

pub struct ScalingWorkflow;

impl ScalingWorkflow {
    /// Create a new worker canister in the given pool, if policy allows.
    pub(crate) async fn create_worker(pool: &str) -> Result<Principal, InternalError> {
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
            return Err(InternalError::domain(InternalErrorOrigin::Workflow, reason));
        }

        let entry_plan = worker_entry.ok_or_else(|| {
            InternalError::invariant(
                InternalErrorOrigin::Workflow,
                "worker entry missing for spawn plan",
            )
        })?;

        let role = entry_plan.canister_role.clone();

        // 3. Create the canister
        let pid =
            RequestOps::create_canister::<()>(&role, CreateCanisterParent::ThisCanister, None)
                .await?
                .new_canister_pid;

        // 4. Register in memory
        let created_at_secs = IcOps::now_secs();
        ScalingRegistryOps::upsert_from_plan(pid, entry_plan, created_at_secs);

        Ok(pid)
    }

    /// Plan whether a worker should be created according to policy.
    pub(crate) fn plan_create_worker(pool: &str) -> Result<bool, InternalError> {
        // 0. Observe state (workflow responsibility)
        let worker_count = ScalingRegistryOps::count_by_pool(pool);

        let scaling = ConfigOps::current_scaling_config()?;
        let plan = ScalingPolicy::plan_create_worker(pool, worker_count, scaling)?;

        Ok(plan.should_spawn)
    }
}
