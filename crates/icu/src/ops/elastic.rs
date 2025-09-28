use crate::{
    Error, ThisError,
    config::data::ElasticPool,
    memory::elastic::{ElasticEntry, ElasticRegistry, ElasticRegistryView},
    ops::{
        OpsError, cfg_current_canister,
        request::{CreateCanisterParent, create_canister_request},
    },
    utils::time::now_secs,
};
use candid::Principal;

//
// OPS / ELASTIC
//
// Policy + orchestration layer on top of `ElasticRegistry`.
// Handles creation, draining, rebalancing, and dry-run planning.
//

// -----------------------------------------------------------------------------
// Errors
// -----------------------------------------------------------------------------

/// Errors for elastic operations (policy / orchestration layer).
#[derive(Debug, ThisError)]
pub enum ElasticError {
    /// This hub canister does not have elastic capability enabled.
    #[error("elastic capability disabled for this canister")]
    ElasticDisabled,

    /// A requested pool name does not exist in config.
    #[error("elastic pool '{0}' not found")]
    PoolNotFound(String),
}

// -----------------------------------------------------------------------------
// Planning
// -----------------------------------------------------------------------------

/// Result of a dry-run policy evaluation for scaling an elastic pool.
#[derive(Clone, Debug)]
pub struct ElasticPlan {
    /// Whether a new worker should be spawned.
    pub should_spawn: bool,
    /// Explanation / debug string for the decision.
    pub reason: String,
}

// -----------------------------------------------------------------------------
// Internal helpers
// -----------------------------------------------------------------------------

/// Look up the config for a given elastic pool on the *current canister*.
///
/// Returns a cloned [`ElasticPool`] on success, or a wrapped [`ElasticError`].
fn get_elastic_pool_cfg(pool: &str) -> Result<ElasticPool, Error> {
    let cfg = cfg_current_canister()?;

    let elastic_cfg = cfg
        .elastic
        .ok_or_else(|| OpsError::from(ElasticError::ElasticDisabled))?;

    let pool_cfg = elastic_cfg
        .pools
        .get(pool)
        .ok_or_else(|| OpsError::from(ElasticError::PoolNotFound(pool.to_string())))?;

    Ok(pool_cfg.clone())
}

// -----------------------------------------------------------------------------
// Public API
// -----------------------------------------------------------------------------

/// Export a snapshot of the current elastic registry state.
#[must_use]
pub fn export_registry() -> ElasticRegistryView {
    ElasticRegistry::export()
}

/// Create a new elastic worker canister in the given pool and register it.
///
/// This:
/// 1. Reads the [`ElasticPool`] config from the current canister.
/// 2. Creates a canister of the configured type.
/// 3. Inserts it into the [`ElasticRegistry`] with initial metadata.
pub async fn create_worker(pool: &str) -> Result<Principal, Error> {
    // 1. Look up pool config
    let pool_cfg = get_elastic_pool_cfg(pool)?;
    let ty = pool_cfg.canister_type.clone();

    // 2. Create the canister
    let pid = create_canister_request::<()>(&ty, CreateCanisterParent::Caller, None)
        .await?
        .new_canister_pid;

    // 3. Register in memory
    let entry = ElasticEntry {
        pool: pool.to_string(),
        canister_type: ty,
        created_at_secs: now_secs(),
        // load_bps: 0 by default (no load yet)
    };
    ElasticRegistry::insert(pid, entry);

    Ok(pid)
}

/// Dry-run the scaling policy for a pool without creating a canister.
///
/// For now this is a stub that always recommends scaling up. Later, it should
/// evaluate thresholds from [`ElasticPool.policy`] and current registry load.
pub fn plan_create_worker(pool: &str) -> Result<ElasticPlan, Error> {
    // Ensure pool exists + elastic capability enabled (mirrors create_worker).
    let pool_cfg = get_elastic_pool_cfg(pool)?;

    // TODO: fold in policy thresholds + registry state
    let should_spawn = true;
    let reason = format!(
        "Elastic pool '{pool}' (type {}) requested scale-up (naive policy)",
        pool_cfg.canister_type
    );

    Ok(ElasticPlan {
        should_spawn,
        reason,
    })
}
