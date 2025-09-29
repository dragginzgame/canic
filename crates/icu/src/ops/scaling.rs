use crate::{
    Error, ThisError,
    config::data::ScalePool,
    memory::scaling::{ScalingRegistry, ScalingRegistryView, WorkerEntry},
    ops::{
        OpsError, cfg_current_canister,
        request::{CreateCanisterParent, create_canister_request},
    },
    utils::time::now_secs,
};
use candid::Principal;

//
// OPS / SCALING
//
// Policy + orchestration layer on top of `ScalingRegistry`.
// Handles creation, draining, rebalancing, and dry-run planning.
//

// -----------------------------------------------------------------------------
// Errors
// -----------------------------------------------------------------------------

///
/// ScalingError
/// Errors for scaling operations (policy / orchestration layer).
///

#[derive(Debug, ThisError)]
pub enum ScalingError {
    #[error("scaling capability disabled for this canister")]
    ScalingDisabled,

    #[error("scaling pool '{0}' not found")]
    PoolNotFound(String),
}

// -----------------------------------------------------------------------------
// Planning
// -----------------------------------------------------------------------------

///
/// ScalingPlan
/// Result of a dry-run policy evaluation for scaling a pool.
///

#[derive(Clone, Debug)]
pub struct ScalingPlan {
    /// Whether a new worker should be spawned.
    pub should_spawn: bool,
    /// Explanation / debug string for the decision.
    pub reason: String,
}

// -----------------------------------------------------------------------------
// Internal helpers
// -----------------------------------------------------------------------------

/// Look up the config for a given pool on the *current canister*.
fn get_scaling_pool_cfg(pool: &str) -> Result<ScalePool, Error> {
    let cfg = cfg_current_canister()?;

    let scale_cfg = cfg
        .scaling
        .ok_or_else(|| OpsError::from(ScalingError::ScalingDisabled))?;

    let pool_cfg = scale_cfg
        .pools
        .get(pool)
        .ok_or_else(|| OpsError::from(ScalingError::PoolNotFound(pool.to_string())))?;

    Ok(pool_cfg.clone())
}

// -----------------------------------------------------------------------------
// Public API
// -----------------------------------------------------------------------------

/// Export a snapshot of the current registry state.
#[must_use]
pub fn export_registry() -> ScalingRegistryView {
    ScalingRegistry::export()
}

/// Create a new worker canister in the given pool and register it.
pub async fn create_worker(pool: &str) -> Result<Principal, Error> {
    // 1. Look up pool config
    let pool_cfg = get_scaling_pool_cfg(pool)?;
    let ty = pool_cfg.canister_type.clone();

    // 2. Create the canister
    let pid = create_canister_request::<()>(&ty, CreateCanisterParent::Caller, None)
        .await?
        .new_canister_pid;

    // 3. Register in memory
    let entry = WorkerEntry {
        pool: pool.to_string(),
        canister_type: ty,
        created_at_secs: now_secs(),
        // load_bps: 0 by default (no load yet)
    };

    ScalingRegistry::insert(pid, entry);

    Ok(pid)
}

/// Dry-run the scaling policy for a pool without creating a canister.
///
/// For now this is a stub that always recommends scaling up. Later, it should
/// evaluate thresholds from [`ScalePool.policy`] and current registry load.
pub fn plan_create_worker(pool: &str) -> Result<ScalingPlan, Error> {
    // Ensure pool exists + capability enabled (mirrors create_worker).
    let pool_cfg = get_scaling_pool_cfg(pool)?;

    // TODO: fold in policy thresholds + registry state
    let should_spawn = true;
    let reason = format!(
        "Elastic pool '{pool}' (type {}) requested scale-up (naive policy)",
        pool_cfg.canister_type
    );

    Ok(ScalingPlan {
        should_spawn,
        reason,
    })
}
