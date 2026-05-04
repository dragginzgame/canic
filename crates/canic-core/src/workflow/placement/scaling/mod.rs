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
        runtime::metrics::{
            recording::ScalingMetricEvent as MetricEvent,
            scaling::{
                ScalingMetricOperation as MetricOperation, ScalingMetricOutcome as MetricOutcome,
                ScalingMetricReason as MetricReason,
            },
        },
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
        let scaling = match ConfigOps::current_scaling_config() {
            Ok(Some(scaling)) => scaling,
            Ok(None) => {
                MetricEvent::skipped(
                    MetricOperation::BootstrapConfig,
                    MetricReason::ScalingDisabled,
                );
                return Ok(());
            }
            Err(err) => {
                MetricEvent::failed(MetricOperation::BootstrapConfig, &err);
                return Err(err);
            }
        };

        MetricEvent::started(MetricOperation::BootstrapConfig);
        for (pool, pool_cfg) in scaling.pools {
            if let Err(err) = Self::bootstrap_initial_workers_for_pool(&pool, &pool_cfg).await {
                MetricEvent::failed(MetricOperation::BootstrapConfig, &err);
                return Err(err);
            }
        }

        MetricEvent::completed(MetricOperation::BootstrapConfig, MetricReason::Ok);
        Ok(())
    }

    /// Create a new worker canister in the given pool, if policy allows.
    pub(crate) async fn create_worker(pool: &str) -> Result<Principal, InternalError> {
        MetricEvent::started(MetricOperation::PlanCreate);

        // 0. Observe state (workflow responsibility)
        let worker_count = ScalingRegistryOps::count_by_pool(pool);
        let scaling = match ConfigOps::current_scaling_config() {
            Ok(scaling) => scaling,
            Err(err) => {
                MetricEvent::failed(MetricOperation::PlanCreate, &err);
                return Err(err);
            }
        };
        crate::perf!("observe_state");

        // 1. Evaluate policy
        let ScalingPlan {
            should_spawn,
            plan_reason,
            reason,
            worker_entry,
        } = match ScalingPolicy::plan_create_worker(pool, worker_count, scaling) {
            Ok(plan) => plan,
            Err(err) => {
                MetricEvent::failed(MetricOperation::PlanCreate, &err);
                return Err(err);
            }
        };
        crate::perf!("plan_spawn");

        if !should_spawn {
            MetricEvent::skipped(
                MetricOperation::PlanCreate,
                MetricReason::from_plan_reason(plan_reason),
            );
            return Err(InternalError::domain(InternalErrorOrigin::Workflow, reason));
        }

        let entry_plan = worker_entry.ok_or_else(|| {
            MetricEvent::failed_reason(
                MetricOperation::PlanCreate,
                MetricReason::MissingWorkerEntry,
            );
            InternalError::invariant(
                InternalErrorOrigin::Workflow,
                "worker entry missing for spawn plan",
            )
        })?;

        MetricEvent::completed(
            MetricOperation::PlanCreate,
            MetricReason::from_plan_reason(plan_reason),
        );
        Self::create_worker_from_plan(entry_plan).await
    }

    /// Plan whether a worker should be created according to policy.
    pub(crate) fn plan_create_worker(pool: &str) -> Result<bool, InternalError> {
        MetricEvent::started(MetricOperation::PlanCreate);

        // 0. Observe state (workflow responsibility)
        let worker_count = ScalingRegistryOps::count_by_pool(pool);

        let scaling = match ConfigOps::current_scaling_config() {
            Ok(scaling) => scaling,
            Err(err) => {
                MetricEvent::failed(MetricOperation::PlanCreate, &err);
                return Err(err);
            }
        };
        crate::perf!("observe_state");
        let plan = match ScalingPolicy::plan_create_worker(pool, worker_count, scaling) {
            Ok(plan) => plan,
            Err(err) => {
                MetricEvent::failed(MetricOperation::PlanCreate, &err);
                return Err(err);
            }
        };
        crate::perf!("plan_spawn");

        MetricEvent::record(
            MetricOperation::PlanCreate,
            if plan.should_spawn {
                MetricOutcome::Completed
            } else {
                MetricOutcome::Skipped
            },
            MetricReason::from_plan_reason(plan.plan_reason),
        );

        Ok(plan.should_spawn)
    }

    // Create enough workers to satisfy one pool's startup warmup target.
    async fn bootstrap_initial_workers_for_pool(
        pool: &str,
        pool_cfg: &crate::config::schema::ScalePool,
    ) -> Result<(), InternalError> {
        MetricEvent::started(MetricOperation::BootstrapPool);
        let target = pool_cfg.policy.initial_workers;
        if target == 0 {
            MetricEvent::skipped(
                MetricOperation::BootstrapPool,
                MetricReason::NoInitialWorkers,
            );
            return Ok(());
        }

        let mut created = 0u32;
        loop {
            let current = ScalingRegistryOps::count_by_pool(pool);
            if current >= target {
                MetricEvent::record(
                    MetricOperation::BootstrapPool,
                    if created == 0 {
                        MetricOutcome::Skipped
                    } else {
                        MetricOutcome::Completed
                    },
                    if created == 0 {
                        MetricReason::TargetSatisfied
                    } else {
                        MetricReason::Ok
                    },
                );
                return Ok(());
            }

            let entry_plan = ScalingWorkerPlanEntry {
                pool: BoundedString64::new(pool),
                canister_role: pool_cfg.canister_role.clone(),
            };
            let pid = match Self::create_worker_from_plan(entry_plan).await {
                Ok(pid) => pid,
                Err(err) => {
                    MetricEvent::failed(MetricOperation::BootstrapPool, &err);
                    return Err(err);
                }
            };
            created = created.saturating_add(1);

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

        MetricEvent::started(MetricOperation::CreateWorker);
        let pid = match RequestOps::create_canister::<()>(
            &role,
            CreateCanisterParent::ThisCanister,
            None,
        )
        .await
        {
            Ok(response) => {
                MetricEvent::completed(MetricOperation::CreateWorker, MetricReason::Ok);
                response.new_canister_pid
            }
            Err(err) => {
                MetricEvent::failed(MetricOperation::CreateWorker, &err);
                return Err(err);
            }
        };
        crate::perf!("create_canister");

        MetricEvent::started(MetricOperation::RegisterWorker);
        let created_at_secs = IcOps::now_secs();
        ScalingRegistryOps::upsert_from_plan(pid, entry_plan, created_at_secs);
        MetricEvent::completed(MetricOperation::RegisterWorker, MetricReason::Ok);
        crate::perf!("register_worker");

        Ok(pid)
    }
}
