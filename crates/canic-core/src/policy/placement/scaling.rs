//! Policy layer for scaling worker pools.
//!
//! Scaling builds on top of the scaling registry and configuration entries
//! under `[canisters.<type>.scaling]`. This module is PURE policy:
//! - reads config
//! - reads registry
//! - computes decisions
//!
//! No IC calls. No async. No side effects.

pub use crate::model::memory::scaling::ScalingRegistryView;

use crate::{
    Error, ThisError,
    config::schema::ScalePool,
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
        Self::OpsError(err.to_string())
    }
}

///
/// ScalingPlan
/// Result of a dry-run evaluation for scaling decisions
///

#[derive(Clone, Debug)]
pub struct ScalingPlan {
    /// Whether a new worker should be spawned.
    pub should_spawn: bool,

    /// Explanation / debug string for the decision.
    pub reason: String,
}

///
/// ScalingPolicy
///

pub struct ScalingPolicy;

impl ScalingPolicy {
    /// Evaluate scaling policy for a pool without side effects.
    #[allow(clippy::cast_possible_truncation)]
    pub fn plan_create_worker(pool: &str) -> Result<ScalingPlan, Error> {
        let pool_cfg = Self::get_scaling_pool_cfg(pool)?;
        let policy = pool_cfg.policy;
        let worker_count = ScalingRegistryOps::find_by_pool(pool).len() as u32;

        if policy.max_workers > 0 && worker_count >= policy.max_workers {
            return Ok(ScalingPlan {
                should_spawn: false,
                reason: format!(
                    "pool '{pool}' at max_workers ({}/{})",
                    worker_count, policy.max_workers
                ),
            });
        }

        if worker_count < policy.min_workers {
            return Ok(ScalingPlan {
                should_spawn: true,
                reason: format!(
                    "pool '{pool}' below min_workers (current {worker_count}, min {})",
                    policy.min_workers
                ),
            });
        }

        Ok(ScalingPlan {
            should_spawn: false,
            reason: format!(
                "pool '{pool}' within policy bounds (current {worker_count}, min {}, max {})",
                policy.min_workers, policy.max_workers
            ),
        })
    }

    /// Convenience helper.
    pub fn should_spawn_worker(pool: &str) -> Result<bool, Error> {
        Ok(Self::plan_create_worker(pool)?.should_spawn)
    }

    /// Export a snapshot of the current registry state.
    #[must_use]
    pub fn export() -> ScalingRegistryView {
        ScalingRegistryOps::export()
    }

    /// Look up the config for a given pool on the *current canister*.
    fn get_scaling_pool_cfg(pool: &str) -> Result<ScalePool, Error> {
        let cfg = ConfigOps::current_canister();
        let scale_cfg = cfg.scaling.ok_or(ScalingPolicyError::ScalingDisabled)?;

        let pool_cfg = scale_cfg
            .pools
            .get(pool)
            .ok_or_else(|| ScalingPolicyError::PoolNotFound(pool.to_string()))?;

        Ok(pool_cfg.clone())
    }
}
