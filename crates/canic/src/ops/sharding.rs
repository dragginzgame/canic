//! Policy layer for sharding tenant workloads across pools.
//!
//! Sharding orchestrates maintained shard registries, enforcing config-driven
//! policies around capacity, growth thresholds, and tenant assignment. This
//! module wraps [`ShardingRegistry`] to provide admin commands, dry-run
//! planners, and helper flows for assigning, draining, or rebalancing shards.

use crate::{
    Error, Log, ThisError,
    config::Config,
    log,
    memory::{
        capability::sharding::{PoolMetrics, ShardingRegistry, ShardingRegistryView},
        state::CanisterState,
    },
    ops::{
        OpsError,
        request::{CreateCanisterParent, create_canister_request},
    },
    types::CanisterType,
};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};

///
/// ShardingError
/// Errors produced by sharding operations (policy / orchestration layer)
///

#[derive(Debug, ThisError)]
pub enum ShardingError {
    #[error("shard cap reached")]
    ShardCapReached,

    #[error("below growth threshold")]
    BelowGrowthThreshold,

    #[error("sharding disabled")]
    ShardingDisabled,

    #[error("shard pool not found")]
    PoolNotFound,

    #[error("tenant '{0}' not found")]
    TenantNotFound(Principal),
}

impl From<ShardingError> for Error {
    fn from(err: ShardingError) -> Self {
        OpsError::from(err).into()
    }
}

///
/// ShardingPolicy
/// Policy thresholds derived from configuration for managing a shard pool
///

#[derive(Clone, Copy, Debug)]
pub struct ShardingPolicy {
    pub initial_capacity: u32,
    pub max_shards: u32,
    pub growth_threshold_pct: u32,
}

///
/// ShardingPlan
/// Dry-run planning output for assigning a tenant to a shard
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct ShardingPlan {
    pub state: ShardingPlanState,
    pub utilization_pct: u32,
    pub active_count: u32,
    pub total_capacity: u64,
    pub total_used: u64,
}

///
/// ShardingPlanState
/// State of a dry-run shard assignment
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub enum ShardingPlanState {
    AlreadyAssigned { pid: Principal },
    UseExisting { pid: Principal },
    CreateAllowed,
    CreateBlocked { reason: String },
}

///
/// Administrative shard operations, grouped under a single endpoint.
///
#[derive(CandidType, Deserialize, Serialize, Debug, Clone)]
pub enum AdminCommand {
    Assign {
        pid: Principal,
        pool: String,
        shard_pid: Principal,
    },
    Drain {
        pool: String,
        shard_pid: Principal,
        max_moves: u32,
    },
    Rebalance {
        pool: String,
        max_moves: u32,
    },
    Decommission {
        shard_pid: Principal,
    },
}

///
/// AdminCommand
/// Result of executing an [`AdminCommand`].
///

#[derive(CandidType, Deserialize, Serialize, Debug, Clone)]
pub enum AdminResult {
    Ok,
    Moved(u32),
}

/// Run an administrative shard command.
pub async fn admin_command(cmd: AdminCommand) -> Result<AdminResult, Error> {
    match cmd {
        AdminCommand::Assign {
            pid,
            pool,
            shard_pid,
        } => {
            ShardingRegistry::assign(&pool, pid, shard_pid)?;
            Ok(AdminResult::Ok)
        }
        AdminCommand::Drain {
            pool,
            shard_pid,
            max_moves,
        } => {
            let moved = drain_shard(&pool, shard_pid, max_moves).await?;
            Ok(AdminResult::Moved(moved))
        }
        AdminCommand::Rebalance { pool, max_moves } => {
            let moved = rebalance_pool(&pool, max_moves)?;
            Ok(AdminResult::Moved(moved))
        }
        AdminCommand::Decommission { shard_pid } => {
            decommission_shard(shard_pid)?;
            Ok(AdminResult::Ok)
        }
    }
}

/// Check whether a pool can create a new shard under the given policy.
const fn ensure_can_create(
    metrics: &PoolMetrics,
    policy: &ShardingPolicy,
) -> Result<(), ShardingError> {
    if metrics.active_count >= policy.max_shards {
        return Err(ShardingError::ShardCapReached);
    }
    if metrics.utilization_pct < policy.growth_threshold_pct && metrics.total_capacity > 0 {
        return Err(ShardingError::BelowGrowthThreshold);
    }

    Ok(())
}

/// Lookup the config for a shard pool on the current canister.
fn get_shard_pool_cfg(pool: &str) -> Result<(CanisterType, ShardingPolicy), Error> {
    let this_ty = CanisterState::try_get_canister()?.ty;

    let sharding_cfg = Config::try_get_canister(&this_ty)?
        .sharding
        .ok_or(OpsError::ShardingError(ShardingError::ShardingDisabled))?;

    let pool_cfg = sharding_cfg
        .pools
        .get(pool)
        .ok_or(OpsError::ShardingError(ShardingError::PoolNotFound))?;

    Ok((
        pool_cfg.canister_type.clone(),
        ShardingPolicy {
            initial_capacity: pool_cfg.policy.initial_capacity,
            max_shards: pool_cfg.policy.max_shards,
            growth_threshold_pct: pool_cfg.policy.growth_threshold_pct,
        },
    ))
}

/// Lookup the shard assigned to a tenant (if any).
#[must_use]
pub fn lookup_tenant(pool: &str, tenant_pid: Principal) -> Option<Principal> {
    ShardingRegistry::tenant_shard(pool, tenant_pid)
}

/// Lookup the shard assigned to a tenant, returning an error if none.
pub fn try_lookup_tenant(pool: &str, tenant_pid: Principal) -> Result<Principal, Error> {
    lookup_tenant(pool, tenant_pid).ok_or_else(|| ShardingError::TenantNotFound(tenant_pid).into())
}

/// Export the full shard registry view.
#[must_use]
pub fn export_registry() -> ShardingRegistryView {
    ShardingRegistry::export()
}

/// Assign a tenant to a pool using config-driven policy.
/// Creates a shard if policy allows.
pub async fn assign_to_pool(pool: &str, tenant: Principal) -> Result<Principal, Error> {
    let (canister_type, policy) = get_shard_pool_cfg(pool)?;
    assign_with_policy(&canister_type, pool, tenant, policy, None).await
}

/// Ensure a tenant is assigned to a shard; create if policy allows.
pub async fn assign_with_policy(
    canister_type: &CanisterType,
    pool: &str,
    tenant: Principal,
    policy: ShardingPolicy,
    extra_arg: Option<Vec<u8>>,
) -> Result<Principal, Error> {
    // Already assigned?
    if let Some(pid) = ShardingRegistry::tenant_shard(pool, tenant) {
        log!(
            Log::Info,
            "📦 tenant={tenant} already shard={pid} pool={pool}"
        );
        return Ok(pid);
    }

    // Try existing shards
    if let Some(pid) = ShardingRegistry::assign_best_effort(pool, tenant) {
        log!(
            Log::Info,
            "📦 tenant={tenant} assigned shard={pid} pool={pool}"
        );
        return Ok(pid);
    }

    // Maybe create new shard
    let metrics = ShardingRegistry::metrics(pool);
    ensure_can_create(&metrics, &policy).map_err(OpsError::ShardingError)?;

    let response =
        create_canister_request::<Vec<u8>>(canister_type, CreateCanisterParent::Caller, extra_arg)
            .await?;
    let pid = response.new_canister_pid;

    ShardingRegistry::create(pid, pool, canister_type, policy.initial_capacity);
    log!(Log::Ok, "✨ sharder.create: {pid} pool={pool}");

    // Assign again (should now succeed)
    let fallback = ShardingRegistry::assign_best_effort(pool, tenant)
        .ok_or_else(|| Error::custom("no shard available after creation"))?;

    Ok(fallback)
}

/// Drain up to `limit` tenants from a shard into other shards.
/// Creates new shards if none available.
pub async fn drain_shard(pool: &str, shard_pid: Principal, limit: u32) -> Result<u32, Error> {
    let (canister_type, policy) = get_shard_pool_cfg(pool)?;
    let tenants = ShardingRegistry::tenants_in_shard(pool, shard_pid);
    let mut moved = 0;

    for tenant in tenants.into_iter().take(limit as usize) {
        if let Some(new_pid) =
            ShardingRegistry::assign_best_effort_excluding(pool, tenant, shard_pid)
        {
            log!(
                Log::Info,
                "🚰 drained tenant={tenant} donor={shard_pid} → shard={new_pid}"
            );
            moved += 1;
            continue;
        }

        // No shard available → create one if policy allows
        let metrics = ShardingRegistry::metrics(pool);
        ensure_can_create(&metrics, &policy).map_err(OpsError::ShardingError)?;

        let response =
            create_canister_request::<Vec<u8>>(&canister_type, CreateCanisterParent::Caller, None)
                .await?;
        let new_pid = response.new_canister_pid;

        ShardingRegistry::create(new_pid, pool, &canister_type, policy.initial_capacity);
        ShardingRegistry::assign_direct(pool, tenant, new_pid)?;

        log!(
            Log::Ok,
            "✨ sharder.create: {new_pid} draining donor={shard_pid}"
        );
        moved += 1;
    }

    Ok(moved)
}

/// Rebalance tenants across shards in a pool (no new shards created).
pub fn rebalance_pool(pool: &str, limit: u32) -> Result<u32, Error> {
    let mut moved = 0;

    for _ in 0..limit {
        let view = ShardingRegistry::export();
        let mut candidates: Vec<(Principal, u64, u32, u64)> = view
            .into_iter()
            .filter(|(_, e)| e.pool == pool)
            .map(|(pid, e)| {
                (
                    pid,
                    e.load_bps().unwrap_or(u64::MAX),
                    e.count,
                    e.created_at_secs,
                )
            })
            .collect();

        if candidates.len() < 2 {
            break;
        }

        candidates.sort_by_key(|(_, load, count, created)| (*load, *count, *created));
        let (recv_pid, recv_load, _, _) = candidates.first().copied().unwrap();
        let (donor_pid, donor_load, donor_count, _) = candidates.last().copied().unwrap();

        if donor_pid == recv_pid || donor_count == 0 || donor_load <= recv_load {
            break;
        }

        if let Some(tenant) = ShardingRegistry::tenants_in_shard(pool, donor_pid)
            .first()
            .copied()
            && ShardingRegistry::assign_direct(pool, tenant, recv_pid).is_ok()
        {
            log!(
                Log::Info,
                "🔀 moved tenant={tenant} donor={donor_pid} → recv={recv_pid}"
            );
            moved += 1;
        }
    }

    Ok(moved)
}

/// Decommission an empty shard (remove from registry).
pub fn decommission_shard(shard_pid: Principal) -> Result<(), Error> {
    ShardingRegistry::remove(shard_pid)?;
    log!(Log::Ok, "🗑️ decommissioned shard={shard_pid}");
    Ok(())
}

/// Dry-run plan for assigning a tenant to a shard (never creates).
pub fn plan_assign_to_pool(pool: &str, tenant: Principal) -> Result<ShardingPlan, Error> {
    let metrics = ShardingRegistry::metrics(pool);

    if let Some(pid) = ShardingRegistry::tenant_shard(pool, tenant) {
        return Ok(ShardingPlan {
            state: ShardingPlanState::AlreadyAssigned { pid },
            utilization_pct: metrics.utilization_pct,
            active_count: metrics.active_count,
            total_capacity: metrics.total_capacity,
            total_used: metrics.total_used,
        });
    }

    if let Some(pid) = ShardingRegistry::peek_best_effort(pool) {
        return Ok(ShardingPlan {
            state: ShardingPlanState::UseExisting { pid },
            utilization_pct: metrics.utilization_pct,
            active_count: metrics.active_count,
            total_capacity: metrics.total_capacity,
            total_used: metrics.total_used,
        });
    }

    let (_, policy) = get_shard_pool_cfg(pool)?;
    match ensure_can_create(&metrics, &policy) {
        Ok(()) => Ok(ShardingPlan {
            state: ShardingPlanState::CreateAllowed,
            utilization_pct: metrics.utilization_pct,
            active_count: metrics.active_count,
            total_capacity: metrics.total_capacity,
            total_used: metrics.total_used,
        }),
        Err(e) => Ok(ShardingPlan {
            state: ShardingPlanState::CreateBlocked {
                reason: e.to_string(),
            },
            utilization_pct: metrics.utilization_pct,
            active_count: metrics.active_count,
            total_capacity: metrics.total_capacity,
            total_used: metrics.total_used,
        }),
    }
}
