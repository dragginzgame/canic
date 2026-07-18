//! Module: workflow::placement::scaling
//!
//! Responsibility: create and bootstrap scaling workers from placement policy.
//! Does not own: scaling policy rules, registry schemas, or endpoint authorization.
//! Boundary: coordinates policy decisions, canister creation, and registry writes.

pub mod query;

use crate::{
    InternalError, InternalErrorOrigin,
    cdk::types::{BoundedString64, Principal},
    config::schema::{ScalePool, ScalingConfig},
    domain::policy::pure::placement::scaling::{
        ScalingPlan, ScalingPolicy, ScalingPolicyInput, ScalingPoolPolicyInput,
    },
    log::Topic,
    model::placement::{allocation::PlacementAllocationIdentity, scaling::ScalingWorkerEntry},
    ops::{
        config::ConfigOps,
        ic::IcOps,
        runtime::metrics::{
            recording::ScalingMetricEvent as MetricEvent,
            scaling::{
                ScalingMetricOperation as MetricOperation, ScalingMetricOutcome as MetricOutcome,
                ScalingMetricReason as MetricReason,
            },
        },
        storage::placement::scaling::ScalingRegistryOps,
    },
    workflow::placement::allocation::{PlacementAllocationRequest, PlacementAllocationWorkflow},
};

///
/// ScalingWorkflow
///
/// Entry point for scaling placement orchestration.
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
        let scaling_policy = scaling.as_ref().map(scaling_policy_input);
        let ScalingPlan {
            should_spawn,
            plan_reason,
            reason,
            worker_entry,
        } = match ScalingPolicy::plan_create_worker(pool, worker_count, scaling_policy.as_ref()) {
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
        let pool_cfg = scaling
            .as_ref()
            .and_then(|config| config.pools.get(pool))
            .ok_or_else(|| {
                InternalError::invariant(
                    InternalErrorOrigin::Workflow,
                    format!("scaling plan admitted missing pool '{pool}'"),
                )
            })?;
        let available_capacity =
            available_worker_capacity(pool_cfg.policy.max_workers, worker_count);
        Self::create_worker_from_plan(entry_plan, available_capacity).await
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
        let scaling_policy = scaling.as_ref().map(scaling_policy_input);
        let plan =
            match ScalingPolicy::plan_create_worker(pool, worker_count, scaling_policy.as_ref()) {
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
        pool_cfg: &ScalePool,
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

            let entry_plan = ScalingWorkerEntry {
                pool: BoundedString64::new(pool),
                canister_role: pool_cfg.canister_role.clone(),
            };
            let available_capacity = u64::from(target.saturating_sub(current));
            let pid = match Self::create_worker_from_plan(entry_plan, available_capacity).await {
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
        entry_plan: ScalingWorkerEntry,
        available_capacity: u64,
    ) -> Result<Principal, InternalError> {
        let role = entry_plan.canister_role.clone();
        let pool = entry_plan.pool.as_ref();
        let owner = IcOps::canister_self();
        let identity_probe = PlacementAllocationIdentity::scaling(owner, pool, 0, &role, None);
        let sequence = PlacementAllocationWorkflow::next_sequence(&identity_probe);
        let identity = PlacementAllocationIdentity::scaling(owner, pool, sequence, &role, None);
        let reservation_limit =
            PlacementAllocationWorkflow::reservation_limit_for_available_capacity(
                &identity,
                available_capacity,
            );

        MetricEvent::started(MetricOperation::CreateWorker);
        let (permit, pid) =
            match PlacementAllocationWorkflow::create_child(PlacementAllocationRequest {
                identity,
                canister_role: role,
                extra_arg: None,
                reservation_limit,
            })
            .await
            {
                Ok(result) => {
                    MetricEvent::completed(MetricOperation::CreateWorker, MetricReason::Ok);
                    result
                }
                Err(err) => {
                    MetricEvent::failed(MetricOperation::CreateWorker, &err);
                    return Err(err);
                }
            };
        crate::perf!("create_canister");

        MetricEvent::started(MetricOperation::RegisterWorker);
        let created_at_secs = IcOps::now_secs();
        ScalingRegistryOps::upsert(pid, entry_plan, created_at_secs);
        if let Err(err) = PlacementAllocationWorkflow::finish_registered_child(&permit, pid) {
            MetricEvent::failed(MetricOperation::RegisterWorker, &err);
            return Err(err);
        }
        MetricEvent::completed(MetricOperation::RegisterWorker, MetricReason::Ok);
        crate::perf!("register_worker");

        Ok(pid)
    }
}

fn available_worker_capacity(max_workers: u32, worker_count: u32) -> u64 {
    if max_workers == 0 {
        u64::MAX
    } else {
        u64::from(max_workers.saturating_sub(worker_count))
    }
}

fn scaling_policy_input(scaling: &ScalingConfig) -> ScalingPolicyInput {
    ScalingPolicyInput {
        pools: scaling
            .pools
            .iter()
            .map(|(pool, pool_cfg)| {
                (
                    pool.clone(),
                    ScalingPoolPolicyInput {
                        canister_role: pool_cfg.canister_role.clone(),
                        min_workers: pool_cfg.policy.min_workers,
                        max_workers: pool_cfg.policy.max_workers,
                    },
                )
            })
            .collect(),
    }
}
