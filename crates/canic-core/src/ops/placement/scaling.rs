//! Policy layer for scaling worker pools.
//!
//! Scaling builds on top of [`ScalingRegistry`] and the configuration entries
//! under `[canisters.<type>.scaling]`. The helpers in this module apply policy
//! decisions, create new workers when necessary, and surface registry
//! snapshots for diagnostics.

pub use crate::ops::storage::scaling::ScalingRegistryView;

use crate::{
    Error, ThisError,
    cdk::utils::time::now_secs,
    config::schema::ScalePool,
    ops::{
        config::ConfigOps,
        rpc::{CreateCanisterParent, create_canister_request},
        storage::scaling::{ScalingWorkerRegistryStorageOps, WorkerEntry},
    },
};
use candid::Principal;

///
/// ScalingOpsError
/// Errors raised by scaling operations (policy / orchestration layer)
///

#[derive(Debug, ThisError)]
pub enum ScalingOpsError {
    #[error("scaling capability disabled for this canister")]
    ScalingDisabled,

    #[error("scaling pool '{0}' not found")]
    PoolNotFound(String),

    #[error("invalid scaling key: {0}")]
    InvalidKey(String),

    #[error("scaling plan rejected: {0}")]
    PlanRejected(String),
}

impl From<ScalingOpsError> for Error {
    fn from(err: ScalingOpsError) -> Self {
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
/// ScalingRegistryOps
///

pub struct ScalingRegistryOps;

impl ScalingRegistryOps {
    /// Evaluate scaling policy for a pool without side effects.
    #[allow(clippy::cast_possible_truncation)]
    pub fn plan_create_worker(pool: &str) -> Result<ScalingPlan, Error> {
        let pool_cfg = Self::get_scaling_pool_cfg(pool)?;
        let policy = pool_cfg.policy;
        let worker_count = ScalingWorkerRegistryStorageOps::find_by_pool(pool).len() as u32;

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

    /// Look up the config for a given pool on the *current canister*.
    fn get_scaling_pool_cfg(pool: &str) -> Result<ScalePool, Error> {
        let cfg = ConfigOps::current_canister()?;
        let scale_cfg = cfg.scaling.ok_or(ScalingOpsError::ScalingDisabled)?;

        let pool_cfg = scale_cfg
            .pools
            .get(pool)
            .ok_or_else(|| ScalingOpsError::PoolNotFound(pool.to_string()))?;

        Ok(pool_cfg.clone())
    }

    /// Export a snapshot of the current registry state.
    #[must_use]
    pub fn export() -> ScalingRegistryView {
        ScalingWorkerRegistryStorageOps::export()
    }

    /// Create a new worker canister in the given pool and register it.
    pub async fn create_worker(pool: &str) -> Result<Principal, Error> {
        // 1. Evaluate policy
        let plan = Self::plan_create_worker(pool)?;
        if !plan.should_spawn {
            return Err(ScalingOpsError::PlanRejected(plan.reason))?;
        }

        // 2. Look up pool config
        let pool_cfg = Self::get_scaling_pool_cfg(pool)?;
        let ty = pool_cfg.canister_type.clone();

        // 3. Create the canister
        let pid = create_canister_request::<()>(&ty, CreateCanisterParent::ThisCanister, None)
            .await?
            .new_canister_pid;

        // 4. Register in memory
        let entry =
            WorkerEntry::try_new(pool, ty, now_secs()).map_err(ScalingOpsError::InvalidKey)?;

        ScalingWorkerRegistryStorageOps::insert(pid, entry);

        Ok(pid)
    }

    /// Convenience: return only the decision flag for a pool.
    pub fn should_spawn_worker(pool: &str) -> Result<bool, Error> {
        Ok(Self::plan_create_worker(pool)?.should_spawn)
    }
}
