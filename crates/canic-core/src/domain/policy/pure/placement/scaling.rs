//! This module is PURE policy:
//! - reads policy input
//! - evaluates observed state
//! - computes decisions
//!
//! No IC calls. No async. No side effects.

use crate::{
    InternalError,
    domain::policy::pure::PolicyError,
    domain::value::BoundedString64,
    ids::CanisterRole,
    model::placement::scaling::{ScalingPlanReason, ScalingWorkerEntry},
};
use std::collections::BTreeMap;
use thiserror::Error as ThisError;

///
/// ScalingPolicyError
/// Errors raised during scaling policy evaluation
///

#[derive(Debug, ThisError)]
pub enum ScalingPolicyError {
    #[error("scaling capability disabled for this canister")]
    ScalingDisabled,

    #[error("scaling pool '{0}' not found")]
    PoolNotFound(String),
}

impl From<ScalingPolicyError> for InternalError {
    fn from(err: ScalingPolicyError) -> Self {
        PolicyError::from(err).into()
    }
}

///
/// ScalingPlan
///

#[derive(Clone, Debug)]
pub struct ScalingPlan {
    pub should_spawn: bool,
    pub plan_reason: ScalingPlanReason,
    pub reason: String,
    pub worker_entry: Option<ScalingWorkerEntry>,
}

///
/// ScalingPolicyInput
///

#[derive(Clone, Debug, Default)]
pub struct ScalingPolicyInput {
    pub pools: BTreeMap<String, ScalingPoolPolicyInput>,
}

///
/// ScalingPoolPolicyInput
///

#[derive(Clone, Debug)]
pub struct ScalingPoolPolicyInput {
    pub canister_role: CanisterRole,
    pub min_workers: u32,
    pub max_workers: u32,
}

///
/// ScalingPolicy
///

pub struct ScalingPolicy;

impl ScalingPolicy {
    pub(crate) fn plan_create_worker(
        pool: &str,
        worker_count: u32,
        scaling: Option<&ScalingPolicyInput>,
    ) -> Result<ScalingPlan, InternalError> {
        let pool_cfg = Self::get_scaling_pool_cfg(pool, scaling)?;

        // Max bound check
        if pool_cfg.max_workers > 0 && worker_count >= pool_cfg.max_workers {
            return Ok(ScalingPlan {
                should_spawn: false,
                plan_reason: ScalingPlanReason::AtMaxWorkers,
                reason: format!(
                    "pool '{pool}' at max_workers ({}/{})",
                    worker_count, pool_cfg.max_workers
                ),
                worker_entry: None,
            });
        }

        // Min bound check
        if worker_count < pool_cfg.min_workers {
            let entry = ScalingWorkerEntry {
                pool: BoundedString64::new(pool),
                canister_role: pool_cfg.canister_role.clone(),
            };

            return Ok(ScalingPlan {
                should_spawn: true,
                plan_reason: ScalingPlanReason::BelowMinWorkers,
                reason: format!(
                    "pool '{pool}' below min_workers (current {worker_count}, min {})",
                    pool_cfg.min_workers
                ),
                worker_entry: Some(entry),
            });
        }

        Ok(ScalingPlan {
            should_spawn: false,
            plan_reason: ScalingPlanReason::WithinBounds,
            reason: format!(
                "pool '{pool}' within policy bounds (current {worker_count}, min {}, max {})",
                pool_cfg.min_workers, pool_cfg.max_workers
            ),
            worker_entry: None,
        })
    }

    fn get_scaling_pool_cfg<'a>(
        pool: &str,
        scaling: Option<&'a ScalingPolicyInput>,
    ) -> Result<&'a ScalingPoolPolicyInput, InternalError> {
        let Some(scaling) = scaling else {
            return Err(ScalingPolicyError::ScalingDisabled.into());
        };

        let Some(pool_cfg) = scaling.pools.get(pool) else {
            return Err(ScalingPolicyError::PoolNotFound(pool.to_string()).into());
        };

        Ok(pool_cfg)
    }
}
