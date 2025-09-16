pub mod admin;

use crate::{
    Error, Log,
    config::Config,
    log,
    memory::{CanisterState, ShardRegistry, ShardRegistryView, canister::shard::PoolMetrics},
    ops::{OpsError, request::create_canister_request},
    types::CanisterType,
};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

//
// OPS / SHARD
//
// This module orchestrates shard lifecycle and policy decisions,
// sitting above the raw registry (`memory/shard`).
//
// Responsibilities:
//   * create new shard canisters (via create_canister_request),
//   * apply pool policies (capacity, growth thresholds),
//   * manage rebalancing and draining,
//   * provide planning endpoints (dry-run).
//
// The raw registry should never be called directly from endpoints;
// always go through this module.
//

///
/// ShardError
/// Error type for shard operations (policy/business logic).
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

/// Policy for managing shards in a pool.
#[derive(Clone, Copy, Debug)]
pub struct ShardPolicy {
    pub initial_capacity: u32,
    pub max_shards: u32,
    pub growth_threshold_pct: u32, // e.g., 80 = 80%
}

/// Dry-run planning output for assigning a tenant to a shard.
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

fn ensure_can_create(metrics: &PoolMetrics, policy: &ShardPolicy) -> Result<(), Error> {
    if metrics.active_count >= policy.max_shards {
        Err(OpsError::ShardError(ShardError::ShardCapReached))?;
    }

    if metrics.utilization_pct < policy.growth_threshold_pct && metrics.total_capacity > 0 {
        Err(OpsError::ShardError(ShardError::BelowGrowthThreshold))?;
    }

    Ok(())
}

fn get_pool_policy(pool: &str) -> Result<(CanisterType, ShardPolicy), Error> {
    let ty = CanisterState::try_get_type()?;

    let cfg = Config::try_get_canister(&ty)
        .ok()
        .and_then(|c| c.sharder)
        .ok_or(ShardError::ShardingDisabled)
        .map_err(OpsError::from)?;

    let pool_cfg = cfg
        .pools
        .get(pool)
        .ok_or(ShardError::PoolNotFound)
        .map_err(OpsError::from)?;

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
    ShardRegistry::get_tenant_shard(pool, tenant_pid)
}

#[must_use]
pub fn export_registry() -> ShardRegistryView {
    ShardRegistry::export()
}

/// Convenience: assign a tenant to a pool using this canister’s type and config-driven policy.
/// Creates a shard if policy allows, otherwise returns an error.
pub async fn assign_to_pool(pool: &str, tenant: Principal) -> Result<Principal, Error> {
    let hub_type = crate::memory::CanisterState::try_get_type()?;

    let cfg = Config::try_get_canister(&hub_type)?
        .sharder
        .ok_or(ShardError::ShardingDisabled)
        .map_err(OpsError::from)?;

    let pool_cfg = cfg
        .pools
        .get(pool)
        .ok_or(ShardError::PoolNotFound)
        .map_err(OpsError::from)?;

    assign_with_policy(
        &pool_cfg.canister_type,
        pool,
        tenant,
        ShardPolicy {
            initial_capacity: pool_cfg.policy.initial_capacity,
            max_shards: pool_cfg.policy.max_shards,
            growth_threshold_pct: pool_cfg.policy.growth_threshold_pct,
        },
        pool_cfg.policy.initial_capacity,
        None,
    )
    .await
}

/// Ensure a tenant is assigned to a shard; create a new shard canister if policy allows.
pub async fn assign_with_policy(
    canister_type: &CanisterType,
    pool: &str,
    tenant: Principal,
    policy: ShardPolicy,
    initial_capacity: u32,
    extra_arg: Option<Vec<u8>>,
) -> Result<Principal, Error> {
    if let Some(pid) = ShardRegistry::get_tenant_shard(pool, tenant) {
        log!(
            Log::Info,
            "📦 tenant={tenant} already shard={pid} pool={pool}"
        );
        return Ok(pid);
    }
    if let Some(pid) = ShardRegistry::assign_tenant_best_effort(pool, tenant, None) {
        log!(
            Log::Info,
            "📦 tenant={tenant} assigned shard={pid} pool={pool}"
        );
        return Ok(pid);
    }

    let metrics = ShardRegistry::metrics_for_pool(pool);
    ensure_can_create(&metrics, &policy)?;

    let response = create_canister_request::<Vec<u8>>(canister_type, extra_arg).await?;
    let pid = response.new_canister_pid;

    ShardRegistry::register(pid, pool, initial_capacity);
    log!(
        Log::Ok,
        "✨ created shard={pid} pool={pool} capacity={initial_capacity}"
    );

    let fallback = ShardRegistry::assign_tenant_best_effort(pool, tenant, None)
        .ok_or_else(|| Error::custom("no shard available after creation"))?;

    Ok(fallback)
}

/// Drain up to `limit` tenants from `shard_pid` into other shards in the same pool.
/// Creates a new shard if none are available.
pub async fn drain_shard(pool: &str, shard_pid: Principal, limit: u32) -> Result<u32, Error> {
    let (canister_type, policy) = get_pool_policy(pool)?;
    let tenants = ShardRegistry::tenants_for_shard(pool, shard_pid);
    let mut moved = 0u32;

    for tenant in tenants.into_iter().take(limit as usize) {
        if ShardRegistry::assign_tenant_best_effort(pool, tenant, Some(shard_pid)).is_some() {
            log!(Log::Info, "🚰 drained tenant={tenant} shard={shard_pid}");
            moved += 1;
            continue;
        }

        let response = create_canister_request::<Vec<u8>>(&canister_type, None).await?;
        let new_pid = response.new_canister_pid;
        ShardRegistry::register(new_pid, pool, policy.initial_capacity);
        ShardRegistry::assign_tenant_to_shard(pool, tenant, new_pid)?;
        log!(
            Log::Ok,
            "✨ created new shard={new_pid} draining={shard_pid}"
        );
        moved += 1;
    }

    Ok(moved)
}

/// Rebalance a pool by moving up to `limit` tenants from the most loaded shard(s)
/// to the least loaded shard(s). Does not create new shards.
pub fn rebalance_pool(pool: &str, limit: u32) -> Result<u32, Error> {
    let mut moved = 0u32;

    for _ in 0..limit {
        let view = ShardRegistry::export();
        let mut candidates: Vec<(Principal, u64, u32, u64)> = view
            .iter()
            .filter(|(_, e)| e.pool == pool)
            .map(|(pid, e)| {
                (
                    *pid,
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

        if let Some(tenant) = ShardRegistry::tenants_for_shard(pool, donor_pid)
            .first()
            .copied()
        {
            if ShardRegistry::assign_tenant_to_shard(pool, tenant, recv_pid).is_ok() {
                log!(
                    Log::Info,
                    "🔀 moved tenant={tenant} donor={donor_pid} → recv={recv_pid}"
                );
                moved += 1;
            } else {
                log!(
                    Log::Warn,
                    "⚠️ failed to move tenant={tenant} donor={donor_pid} recv={recv_pid}"
                );
                break;
            }
        }
    }

    Ok(moved)
}

/// Decommission a shard (must be empty).
pub fn decommission_shard(shard_pid: Principal) -> Result<(), Error> {
    ShardRegistry::remove_shard(shard_pid)?;
    log!(Log::Ok, "🗑️ decommissioned shard={shard_pid}");
    Ok(())
}

/// Dry-run (plan) using config: never creates; returns current metrics and decision.
pub fn plan_assign_to_pool(pool: &str, tenant: Principal) -> Result<ShardPlan, Error> {
    let metrics = ShardRegistry::metrics_for_pool(pool);

    if let Some(pid) = ShardRegistry::get_tenant_shard(pool, tenant) {
        return Ok(ShardPlan {
            state: ShardPlanState::AlreadyAssigned { pid },
            utilization_pct: metrics.utilization_pct,
            active_count: metrics.active_count,
            total_capacity: metrics.total_capacity,
            total_used: metrics.total_used,
        });
    }
    if let Some(pid) = ShardRegistry::peek_best_effort_for_pool(pool) {
        return Ok(ShardPlan {
            state: ShardPlanState::UseExisting { pid },
            utilization_pct: metrics.utilization_pct,
            active_count: metrics.active_count,
            total_capacity: metrics.total_capacity,
            total_used: metrics.total_used,
        });
    }

    let (_, policy) = get_pool_policy(pool)?;

    if metrics.active_count >= policy.max_shards {
        return Ok(ShardPlan {
            state: ShardPlanState::CreateBlocked {
                reason: "shard cap reached".into(),
            },
            utilization_pct: metrics.utilization_pct,
            active_count: metrics.active_count,
            total_capacity: metrics.total_capacity,
            total_used: metrics.total_used,
        });
    }

    if metrics.utilization_pct < policy.growth_threshold_pct && metrics.total_capacity > 0 {
        return Ok(ShardPlan {
            state: ShardPlanState::CreateBlocked {
                reason: "below growth threshold".into(),
            },
            utilization_pct: metrics.utilization_pct,
            active_count: metrics.active_count,
            total_capacity: metrics.total_capacity,
            total_used: metrics.total_used,
        });
    }

    Ok(ShardPlan {
        state: ShardPlanState::CreateAllowed,
        utilization_pct: metrics.utilization_pct,
        active_count: metrics.active_count,
        total_capacity: metrics.total_capacity,
        total_used: metrics.total_used,
    })
}

//
// TESTS
//

#[cfg(test)]
mod tests {
    use super::*;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    #[test]
    fn rebalance_pool_shifts_load() {
        ShardRegistry::clear();
        let shard1 = p(1);
        let shard2 = p(2);

        ShardRegistry::register(shard1, "poolR", 4);
        ShardRegistry::register(shard2, "poolR", 4);

        for i in 0..4 {
            let tenant = p(50 + i);
            ShardRegistry::assign_tenant_to_shard("poolR", tenant, shard1).unwrap();
        }

        let before = ShardRegistry::tenants_for_shard("poolR", shard1).len();
        assert_eq!(before, 4);

        let moved = rebalance_pool("poolR", 2).unwrap();
        assert!(moved > 0);

        let after1 = ShardRegistry::tenants_for_shard("poolR", shard1).len();
        let after2 = ShardRegistry::tenants_for_shard("poolR", shard2).len();
        assert!(after1 < before);
        assert!(after2 > 0);
    }

    #[test]
    fn plan_assign_reports_correct_states() {
        ShardRegistry::clear();
        let shard1 = p(1);
        let tenant = p(42);

        ShardRegistry::register(shard1, "poolP", 2);

        let plan = plan_assign_to_pool("poolP", tenant).unwrap();
        matches!(plan.state, ShardPlanState::UseExisting { .. });

        ShardRegistry::assign_tenant_to_shard("poolP", tenant, shard1).unwrap();
        let plan2 = plan_assign_to_pool("poolP", tenant).unwrap();
        matches!(plan2.state, ShardPlanState::AlreadyAssigned { .. });
    }

    #[test]
    fn decommission_shard_removes_empty() {
        ShardRegistry::clear();
        let shard = p(99);

        ShardRegistry::register(shard, "poolD", 2);
        assert!(decommission_shard(shard).is_ok());

        let view = ShardRegistry::export();
        assert!(view.is_empty());
    }
}
