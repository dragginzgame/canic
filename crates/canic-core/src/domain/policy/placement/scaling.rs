//! This module is PURE policy:
//! - reads config
//! - evaluates observed state
//! - computes decisions
//!
//! No IC calls. No async. No side effects.

use crate::{
    Error, ThisError,
    cdk::types::BoundedString64,
    config::schema::{ScalePool, ScalingConfig},
    domain::policy::PolicyError,
    ids::CanisterRole,
};

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

impl From<ScalingPolicyError> for Error {
    fn from(err: ScalingPolicyError) -> Self {
        PolicyError::from(err).into()
    }
}

///
/// ScalingWorkerPlanEntry
///

#[derive(Clone, Debug)]
pub struct ScalingWorkerPlanEntry {
    pub pool: BoundedString64,
    pub canister_role: CanisterRole,
}

///
/// ScalingPlan
///

#[derive(Clone, Debug)]
pub struct ScalingPlan {
    pub should_spawn: bool,
    pub reason: String,
    pub worker_entry: Option<ScalingWorkerPlanEntry>,
}

///
/// ScalingPolicy
///

pub struct ScalingPolicy;

impl ScalingPolicy {
    #[allow(clippy::cast_possible_truncation)]
    pub(crate) fn plan_create_worker(
        pool: &str,
        worker_count: u32,
        scaling: Option<ScalingConfig>,
    ) -> Result<ScalingPlan, Error> {
        let pool_cfg = Self::get_scaling_pool_cfg(pool, scaling)?;
        let policy = pool_cfg.policy;

        // Max bound check
        if policy.max_workers > 0 && worker_count >= policy.max_workers {
            return Ok(ScalingPlan {
                should_spawn: false,
                reason: format!(
                    "pool '{pool}' at max_workers ({}/{})",
                    worker_count, policy.max_workers
                ),
                worker_entry: None,
            });
        }

        // Min bound check
        if worker_count < policy.min_workers {
            let entry = ScalingWorkerPlanEntry {
                pool: BoundedString64::new(pool),
                canister_role: pool_cfg.canister_role,
            };

            return Ok(ScalingPlan {
                should_spawn: true,
                reason: format!(
                    "pool '{pool}' below min_workers (current {worker_count}, min {})",
                    policy.min_workers
                ),
                worker_entry: Some(entry),
            });
        }

        Ok(ScalingPlan {
            should_spawn: false,
            reason: format!(
                "pool '{pool}' within policy bounds (current {worker_count}, min {}, max {})",
                policy.min_workers, policy.max_workers
            ),
            worker_entry: None,
        })
    }

    fn get_scaling_pool_cfg(
        pool: &str,
        scaling: Option<ScalingConfig>,
    ) -> Result<ScalePool, Error> {
        let Some(scaling) = scaling else {
            return Err(ScalingPolicyError::ScalingDisabled.into());
        };

        let Some(pool_cfg) = scaling.pools.get(pool) else {
            return Err(ScalingPolicyError::PoolNotFound(pool.to_string()).into());
        };

        Ok(pool_cfg.clone())
    }
}
