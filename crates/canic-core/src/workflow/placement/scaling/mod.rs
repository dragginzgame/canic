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
    cdk::types::BoundedString64,
    domain::policy::placement::scaling::{ScalingPlan, ScalingPolicy, ScalingWorkerPlanEntry},
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
    /// Create configured startup workers for every scaling pool on this canister.
    pub(crate) async fn bootstrap_configured_initial_workers() -> Result<(), InternalError> {
        let Some(scaling) = ConfigOps::current_scaling_config()? else {
            return Ok(());
        };

        for (pool, pool_cfg) in scaling.pools {
            Self::bootstrap_initial_workers_for_pool(&pool, &pool_cfg).await?;
        }

        Ok(())
    }

    /// Create a new worker canister in the given pool, if policy allows.
    pub(crate) async fn create_worker(pool: &str) -> Result<Principal, InternalError> {
        // 0. Observe state (workflow responsibility)
        let worker_count = ScalingRegistryOps::count_by_pool(pool);
        let scaling = ConfigOps::current_scaling_config()?;
        crate::perf!("observe_state");

        // 1. Evaluate policy
        let ScalingPlan {
            should_spawn,
            reason,
            worker_entry,
        } = ScalingPolicy::plan_create_worker(pool, worker_count, scaling)?;
        crate::perf!("plan_spawn");

        if !should_spawn {
            return Err(InternalError::domain(InternalErrorOrigin::Workflow, reason));
        }

        let entry_plan = worker_entry.ok_or_else(|| {
            InternalError::invariant(
                InternalErrorOrigin::Workflow,
                "worker entry missing for spawn plan",
            )
        })?;

        Self::create_worker_from_plan(entry_plan).await
    }

    /// Plan whether a worker should be created according to policy.
    pub(crate) fn plan_create_worker(pool: &str) -> Result<bool, InternalError> {
        // 0. Observe state (workflow responsibility)
        let worker_count = ScalingRegistryOps::count_by_pool(pool);

        let scaling = ConfigOps::current_scaling_config()?;
        crate::perf!("observe_state");
        let plan = ScalingPolicy::plan_create_worker(pool, worker_count, scaling)?;
        crate::perf!("plan_spawn");

        Ok(plan.should_spawn)
    }

    // Create enough workers to satisfy one pool's startup warmup target.
    async fn bootstrap_initial_workers_for_pool(
        pool: &str,
        pool_cfg: &crate::config::schema::ScalePool,
    ) -> Result<(), InternalError> {
        let target = pool_cfg.policy.initial_workers;
        if target == 0 {
            return Ok(());
        }

        loop {
            let current = ScalingRegistryOps::count_by_pool(pool);
            if current >= target {
                return Ok(());
            }

            let entry_plan = ScalingWorkerPlanEntry {
                pool: BoundedString64::new(pool),
                canister_role: pool_cfg.canister_role.clone(),
            };
            let pid = Self::create_worker_from_plan(entry_plan).await?;

            crate::log!(
                Topic::Init,
                Ok,
                "scale.bootstrap: {pid} pool={pool} worker={}/{}",
                current.saturating_add(1),
                target
            );
        }
    }

    // Create and register a worker from a policy-approved or bootstrap-approved plan.
    async fn create_worker_from_plan(
        entry_plan: ScalingWorkerPlanEntry,
    ) -> Result<Principal, InternalError> {
        let role = entry_plan.canister_role.clone();

        let pid =
            RequestOps::create_canister::<()>(&role, CreateCanisterParent::ThisCanister, None)
                .await?
                .new_canister_pid;
        crate::perf!("create_canister");

        let created_at_secs = IcOps::now_secs();
        ScalingRegistryOps::upsert_from_plan(pid, entry_plan, created_at_secs);
        crate::perf!("register_worker");

        Ok(pid)
    }
}
