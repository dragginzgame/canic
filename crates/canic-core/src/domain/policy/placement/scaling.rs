//! Policy layer for scaling worker pools.
//!
//! Scaling builds on top of the scaling registry and configuration entries
//! under `[canisters.<type>.scaling]`. This module is PURE policy:
//! - reads config
//! - reads registry
//! - computes decisions
//!
//! No IC calls. No async. No side effects.

use crate::{
    Error, ThisError,
    cdk::types::BoundedString64,
    config::schema::ScalePool,
    domain::policy::PolicyError,
    ids::CanisterRole,
    ops::{config::ConfigOps, storage::scaling::ScalingRegistryOps},
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
    pub created_at_secs: u64,
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
        created_at_secs: u64,
    ) -> Result<ScalingPlan, Error> {
        let pool_cfg = Self::get_scaling_pool_cfg(pool)?;
        let policy = pool_cfg.policy;
        let worker_count = ScalingRegistryOps::count_by_pool(pool);

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
                created_at_secs,
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

    fn get_scaling_pool_cfg(pool: &str) -> Result<ScalePool, Error> {
        let Some(scaling) = ConfigOps::current_scaling_config()? else {
            return Err(ScalingPolicyError::ScalingDisabled.into());
        };

        let Some(pool_cfg) = scaling.pools.get(pool) else {
            return Err(ScalingPolicyError::PoolNotFound(pool.to_string()).into());
        };

        Ok(pool_cfg.clone())
    }
}
