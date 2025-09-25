use crate::{
    Error, Log, ThisError,
    config::Config,
    log,
    memory::{
        shard::{PoolMetrics, ShardRegistry, ShardRegistryView},
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

//
// OPS / SHARD
//
// Policy + orchestration layer on top of `ShardRegistry`.
// Handles creation, draining, rebalancing, and dry-run planning.
//

///
/// ShardError
/// error type for shard operations (policy/business logic)
///

#[derive(Debug, ThisError)]
pub enum ShardError {
    #[error("shard cap reached")]
    ShardCapReached,

    #[error("below growth threshold")]
    BelowGrowthThreshold,

    #[error("sharding disabled")]
    ShardingDisabled,

    #[error("shard pool not found")]
    PoolNotFound,
}

///
/// AdminCommand
///
/// Administrative shard operations, combined under a single endpoint.
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
/// AdminResult
///

#[derive(CandidType, Deserialize, Serialize, Debug, Clone)]
pub enum AdminResult {
    Ok,
    Moved(u32),
}

/// admin
/// Run a shard admin command.
pub async fn admin_command(cmd: AdminCommand) -> Result<AdminResult, Error> {
    match cmd {
        AdminCommand::Assign {
            pid,
            pool,
            shard_pid,
        } => {
            ShardRegistry::assign(&pool, pid, shard_pid)?;

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

///
/// ShardPolicy
/// Policy for managing shards in a pool
///

#[derive(Clone, Copy, Debug)]
pub struct ShardPolicy {
    pub initial_capacity: u32,
    pub max_shards: u32,
    pub growth_threshold_pct: u32,
}

///
/// ShardPlan
/// Dry-run planning output for assigning a tenant to a shard.
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct ShardPlan {
    pub state: ShardPlanState,
    pub utilization_pct: u32,
    pub active_count: u32,
    pub total_capacity: u64,
    pub total_used: u64,
}

/// State of a planned shard assignment.
#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub enum ShardPlanState {
    AlreadyAssigned { pid: Principal },
    UseExisting { pid: Principal },
    CreateAllowed,
    CreateBlocked { reason: String },
}

//
// Internal helpers
//

const fn ensure_can_create(metrics: &PoolMetrics, policy: &ShardPolicy) -> Result<(), ShardError> {
    if metrics.active_count >= policy.max_shards {
        return Err(ShardError::ShardCapReached);
    }
    if metrics.utilization_pct < policy.growth_threshold_pct && metrics.total_capacity > 0 {
        return Err(ShardError::BelowGrowthThreshold);
    }

    Ok(())
}

fn get_pool_policy(pool: &str) -> Result<(CanisterType, ShardPolicy), Error> {
    let hub_type = CanisterState::try_get_view()?.ty;

    let cfg = Config::try_get_canister(&hub_type)
        .ok()
        .and_then(|c| c.sharder)
        .ok_or(OpsError::ShardError(ShardError::ShardingDisabled))?;

    let pool_cfg = cfg
        .pools
        .get(pool)
        .ok_or(OpsError::ShardError(ShardError::PoolNotFound))?;

    Ok((
        pool_cfg.canister_type.clone(),
        ShardPolicy {
            initial_capacity: pool_cfg.policy.initial_capacity,
            max_shards: pool_cfg.policy.max_shards,
            growth_threshold_pct: pool_cfg.policy.growth_threshold_pct,
        },
    ))
}

//
// Core API
//

#[must_use]
pub fn lookup_tenant(pool: &str, tenant_pid: Principal) -> Option<Principal> {
    ShardRegistry::tenant_shard(pool, tenant_pid)
}

#[must_use]
pub fn export_registry() -> ShardRegistryView {
    ShardRegistry::export()
}

/// Assign a tenant to a pool using config-driven policy.
/// Creates a shard if policy allows.
pub async fn assign_to_pool(pool: &str, tenant: Principal) -> Result<Principal, Error> {
    let (canister_type, policy) = get_pool_policy(pool)?;

    assign_with_policy(&canister_type, pool, tenant, policy, None).await
}

/// Ensure a tenant is assigned to a shard; create if policy allows.
pub async fn assign_with_policy(
    canister_type: &CanisterType,
    pool: &str,
    tenant: Principal,
    policy: ShardPolicy,
    extra_arg: Option<Vec<u8>>,
) -> Result<Principal, Error> {
    if let Some(pid) = ShardRegistry::tenant_shard(pool, tenant) {
        log!(
            Log::Info,
            "📦 tenant={tenant} already shard={pid} pool={pool}"
        );
        return Ok(pid);
    }

    if let Some(pid) = ShardRegistry::assign_best_effort(pool, tenant) {
        log!(
            Log::Info,
            "📦 tenant={tenant} assigned shard={pid} pool={pool}"
        );
        return Ok(pid);
    }

    let metrics = ShardRegistry::metrics(pool);
    ensure_can_create(&metrics, &policy).map_err(OpsError::ShardError)?;

    let response =
        create_canister_request::<Vec<u8>>(canister_type, CreateCanisterParent::Caller, extra_arg)
            .await?;
    let pid = response.new_canister_pid;

    ShardRegistry::create(pid, pool, canister_type, policy.initial_capacity);
    log!(Log::Ok, "✨ sharder.create: {pid} pool={pool}");

    let fallback = ShardRegistry::assign_best_effort(pool, tenant)
        .ok_or_else(|| Error::custom("no shard available after creation"))?;

    Ok(fallback)
}

/// Drain up to `limit` tenants from a shard into other shards.
/// Creates new shards if none available.
pub async fn drain_shard(pool: &str, shard_pid: Principal, limit: u32) -> Result<u32, Error> {
    let (canister_type, policy) = get_pool_policy(pool)?;
    let tenants = ShardRegistry::tenants_in_shard(pool, shard_pid);
    let mut moved = 0;

    for tenant in tenants.into_iter().take(limit as usize) {
        if let Some(new_pid) = ShardRegistry::assign_best_effort_excluding(pool, tenant, shard_pid)
        {
            log!(
                Log::Info,
                "🚰 drained tenant={tenant} donor={shard_pid} → shard={new_pid}"
            );
            moved += 1;
            continue;
        }

        // No shard available → create one if policy allows
        let metrics = ShardRegistry::metrics(pool);
        ensure_can_create(&metrics, &policy).map_err(OpsError::ShardError)?;

        let response =
            create_canister_request::<Vec<u8>>(&canister_type, CreateCanisterParent::Caller, None)
                .await?;
        let new_pid = response.new_canister_pid;

        ShardRegistry::create(new_pid, pool, &canister_type, policy.initial_capacity);
        ShardRegistry::assign_direct(pool, tenant, new_pid)?;

        log!(
            Log::Ok,
            "✨ sharder.create: {new_pid} draining donor={shard_pid}"
        );
        moved += 1;
    }

    Ok(moved)
}

/// Rebalance tenants across shards in a pool (no new shards).
pub fn rebalance_pool(pool: &str, limit: u32) -> Result<u32, Error> {
    let mut moved = 0;

    for _ in 0..limit {
        let view = ShardRegistry::export();
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

        if let Some(tenant) = ShardRegistry::tenants_in_shard(pool, donor_pid)
            .first()
            .copied()
            && ShardRegistry::assign_direct(pool, tenant, recv_pid).is_ok()
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

/// Decommission an empty shard.
pub fn decommission_shard(shard_pid: Principal) -> Result<(), Error> {
    ShardRegistry::remove(shard_pid)?;
    log!(Log::Ok, "🗑️ decommissioned shard={shard_pid}");

    Ok(())
}

/// Dry-run plan (never creates).
pub fn plan_assign_to_pool(pool: &str, tenant: Principal) -> Result<ShardPlan, Error> {
    let metrics = ShardRegistry::metrics(pool);

    if let Some(pid) = ShardRegistry::tenant_shard(pool, tenant) {
        return Ok(ShardPlan {
            state: ShardPlanState::AlreadyAssigned { pid },
            utilization_pct: metrics.utilization_pct,
            active_count: metrics.active_count,
            total_capacity: metrics.total_capacity,
            total_used: metrics.total_used,
        });
    }

    if let Some(pid) = ShardRegistry::peek_best_effort(pool) {
        return Ok(ShardPlan {
            state: ShardPlanState::UseExisting { pid },
            utilization_pct: metrics.utilization_pct,
            active_count: metrics.active_count,
            total_capacity: metrics.total_capacity,
            total_used: metrics.total_used,
        });
    }

    let (_, policy) = get_pool_policy(pool)?;
    match ensure_can_create(&metrics, &policy) {
        Ok(()) => Ok(ShardPlan {
            state: ShardPlanState::CreateAllowed,
            utilization_pct: metrics.utilization_pct,
            active_count: metrics.active_count,
            total_capacity: metrics.total_capacity,
            total_used: metrics.total_used,
        }),
        Err(e) => Ok(ShardPlan {
            state: ShardPlanState::CreateBlocked {
                reason: e.to_string(),
            },
            utilization_pct: metrics.utilization_pct,
            active_count: metrics.active_count,
            total_capacity: metrics.total_capacity,
            total_used: metrics.total_used,
        }),
    }
}
