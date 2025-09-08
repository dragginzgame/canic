use crate::{
    Error,
    config::Config,
    memory::{CanisterShardRegistry, PoolName},
    ops::{prelude::*, request::create_canister_request},
};
use candid::CandidType;
use serde::{Deserialize, Serialize};

///
/// ShardPolicy
///

#[derive(Clone, Copy, Debug)]
pub struct ShardPolicy {
    pub initial_capacity: u32,
    pub max_shards: u32,
    pub growth_threshold_pct: u32, // e.g., 80 = 80%
}

///
/// ShardPlan
///

#[derive(CandidType, Clone, Debug, Deserialize, Serialize)]
pub struct ShardPlan {
    pub state: ShardPlanState,
    pub utilization_pct: u32,
    pub active_count: u32,
    pub total_capacity: u64,
    pub total_used: u64,
}

///
/// ShardPlanState
///

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

#[derive(Clone, Copy, Debug)]
struct Metrics {
    utilization_pct: u32,
    active_count: u32,
    total_capacity: u64,
    total_used: u64,
}

fn metrics_for_pool(pool: &PoolName) -> Metrics {
    let view = CanisterShardRegistry::export();
    let mut active_count = 0u32;
    let mut total_capacity = 0u64;
    let mut total_used = 0u64;
    for (_pid, e) in &view {
        if e.capacity > 0 && e.pool.as_ref() == Some(pool) {
            active_count += 1;
            total_capacity += u64::from(e.capacity);
            total_used += u64::from(e.count);
        }
    }

    let utilization_pct: u32 = if total_capacity == 0 {
        0
    } else {
        let pct = total_used.saturating_mul(100) / total_capacity;
        // pct is logically <= 100; convert safely and cap at 100 if ever exceeded.
        u32::try_from(pct).unwrap_or(100).min(100)
    };

    Metrics {
        utilization_pct,
        active_count,
        total_capacity,
        total_used,
    }
}

fn ensure_can_create(metrics: &Metrics, policy: &ShardPolicy) -> Result<(), Error> {
    if metrics.active_count >= policy.max_shards {
        crate::log!(
            crate::Log::Warn,
            "💎 shard: creation blocked (cap reached) util={util}% active={active} max={max} cap={cap} used={used}",
            util = metrics.utilization_pct,
            active = metrics.active_count,
            max = policy.max_shards,
            cap = metrics.total_capacity,
            used = metrics.total_used
        );
        return Err(Error::custom("shard cap reached"));
    }

    if metrics.utilization_pct < policy.growth_threshold_pct && metrics.total_capacity > 0 {
        crate::log!(
            crate::Log::Info,
            "💎 shard: below growth threshold util={util}% < {threshold}% (cap={cap}, used={used})",
            util = metrics.utilization_pct,
            threshold = policy.growth_threshold_pct,
            cap = metrics.total_capacity,
            used = metrics.total_used
        );
        return Err(Error::custom("below growth threshold"));
    }

    Ok(())
}

fn try_assign_existing(item: Principal, pool: &PoolName) -> Option<Principal> {
    if let Some(pid) = CanisterShardRegistry::get_item_partition(&item, pool) {
        crate::log!(
            crate::Log::Info,
            "💎 shard: already assigned item={item} -> {pid}"
        );
        return Some(pid);
    }

    if let Some(pid) = CanisterShardRegistry::assign_item_best_effort(item, pool) {
        crate::log!(
            crate::Log::Info,
            "💎 shard: assigned existing item={item} -> {pid}"
        );
        return Some(pid);
    }

    None
}

/// Ensure an item is assigned to a shard; create a new shard canister on demand
/// respecting the provided policy.
async fn ensure_item_assignment_internal(
    canister_type: &CanisterType,
    pool: &PoolName,
    item: Principal,
    policy: ShardPolicy,
    initial_capacity: u32,
    extra_arg: Option<Vec<u8>>,
) -> Result<Principal, Error> {
    if let Some(pid) = try_assign_existing(item, pool) {
        return Ok(pid);
    }

    // Evaluate whether to create a new shard based on policy
    let metrics = metrics_for_pool(pool);
    ensure_can_create(&metrics, &policy)?;

    // Create and register a new shard canister
    crate::log!(
        crate::Log::Info,
        "💎 shard: creating new canister type={canister_type} util={util}% active={active} of max={max} (cap={cap}, used={used})",
        util = metrics.utilization_pct,
        active = metrics.active_count,
        max = policy.max_shards,
        cap = metrics.total_capacity,
        used = metrics.total_used
    );

    let response = create_canister_request::<Vec<u8>>(canister_type, extra_arg).await?;
    let pid = response.new_canister_pid;

    CanisterShardRegistry::register(pid, pool.clone(), initial_capacity);
    crate::log!(
        crate::Log::Info,
        "💎 shard: created {pid} with capacity={initial_capacity} (registered)",
    );

    // Try assignment again (prefer the newly created shard)
    if CanisterShardRegistry::assign_item_to_partition(item, pool, pid).is_err() {
        let pid2 = CanisterShardRegistry::assign_item_best_effort(item, pool)
            .ok_or_else(|| Error::custom("no shard available after creation"))?;

        crate::log!(
            crate::Log::Info,
            "💎 shard: assigned after creation via fallback item={item} -> {pid2}",
        );
        return Ok(pid2);
    }

    crate::log!(crate::Log::Ok, "💎 shard: assigned item={item} -> {pid}");
    Ok(pid)
}

/// Ensure an item is assigned, using explicit policy and capacity.
pub async fn ensure_item_assignment(
    canister_type: &CanisterType,
    pool: &PoolName,
    item: Principal,
    policy: ShardPolicy,
    initial_capacity: u32,
    extra_arg: Option<Vec<u8>>,
) -> Result<Principal, Error> {
    ensure_item_assignment_internal(
        canister_type,
        pool,
        item,
        policy,
        initial_capacity,
        extra_arg,
    )
    .await
}

/// Convenience: derive policy/capacity from icu.toml for the given hub and shard types.
pub async fn ensure_item_assignment_from_pool(
    hub_type: &CanisterType,
    canister_type: &CanisterType,
    item: Principal,
) -> Result<Principal, Error> {
    // Hub defines pools
    let cfg = Config::try_get_canister(hub_type)
        .ok()
        .and_then(|c| c.sharder)
        .ok_or_else(|| Error::custom("sharding disabled"))?;

    // Find the pool entry that targets the requested canister type
    let (pool_name, pool) = cfg
        .pools
        .iter()
        .find(|(_, p)| &p.canister_type == canister_type)
        .ok_or_else(|| Error::custom("shard pool not found"))?;

    let policy = ShardPolicy {
        initial_capacity: pool.policy.initial_capacity,
        max_shards: pool.policy.max_shards,
        growth_threshold_pct: pool.policy.growth_threshold_pct,
    };

    ensure_item_assignment_internal(
        &pool.canister_type,
        &PoolName::from(pool_name.as_str()),
        item,
        policy,
        pool.policy.initial_capacity,
        None,
    )
    .await
}

/// Short alias: assign using config.
pub async fn assign_in_pool(
    hub_type: &CanisterType,
    canister_type: &CanisterType,
    item: Principal,
) -> Result<Principal, Error> {
    ensure_item_assignment_from_pool(hub_type, canister_type, item).await
}

/// Short alias: assign with explicit policy/capacity.
pub async fn assign_with_policy(
    canister_type: &CanisterType,
    pool_name: &str,
    item: Principal,
    policy: ShardPolicy,
    initial_capacity: u32,
) -> Result<Principal, Error> {
    ensure_item_assignment_internal(
        canister_type,
        &PoolName::from(pool_name),
        item,
        policy,
        initial_capacity,
        None,
    )
    .await
}

/// Dry-run (plan) using config: never creates; returns current metrics and decision.
pub fn plan_pool(
    hub_type: &CanisterType,
    canister_type: &CanisterType,
    item: Principal,
) -> Result<ShardPlan, Error> {
    // Already assigned?
    // Resolve pool by canister type
    let cfg = Config::try_get_canister(hub_type)
        .ok()
        .and_then(|c| c.sharder)
        .ok_or_else(|| Error::custom("sharding disabled"))?;
    let (pool_name, pool_cfg) = cfg
        .pools
        .iter()
        .find(|(_, p)| &p.canister_type == canister_type)
        .ok_or_else(|| Error::custom("shard pool not found"))?;

    let pool = PoolName::from(pool_name.as_str());
    if let Some(pid) = CanisterShardRegistry::get_item_partition(&item, &pool) {
        return Ok(ShardPlan {
            state: ShardPlanState::AlreadyAssigned { pid },
            utilization_pct: 0,
            active_count: 0,
            total_capacity: 0,
            total_used: 0,
        });
    }

    // Existing candidate? (peek only; do not mutate state)
    if let Some(pid) = CanisterShardRegistry::peek_best_effort_for_pool(&pool) {
        return Ok(ShardPlan {
            state: ShardPlanState::UseExisting { pid },
            utilization_pct: 0,
            active_count: 0,
            total_capacity: 0,
            total_used: 0,
        });
    }

    // Policy from config; require Some to consider sharding enabled
    let max_shards = pool_cfg.policy.max_shards;
    let growth_threshold_pct = pool_cfg.policy.growth_threshold_pct;

    // Metrics and decision
    let metrics = metrics_for_pool(&pool);

    if metrics.active_count >= max_shards {
        return Ok(ShardPlan {
            state: ShardPlanState::CreateBlocked {
                reason: "shard cap reached".to_string(),
            },
            utilization_pct: metrics.utilization_pct,
            active_count: metrics.active_count,
            total_capacity: metrics.total_capacity,
            total_used: metrics.total_used,
        });
    }

    if metrics.utilization_pct < growth_threshold_pct && metrics.total_capacity > 0 {
        return Ok(ShardPlan {
            state: ShardPlanState::CreateBlocked {
                reason: "below growth threshold".to_string(),
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

/// auto_register_from_config
/// Auto-register this canister as a shard if config contains a shard block
pub fn auto_register_from_config() {
    // Only non-root canisters should auto-register
    if crate::memory::CanisterState::is_root() {
        return;
    }

    // Determine this canister's type
    let Some(_ty) = crate::memory::CanisterState::get_type() else {
        return;
    };

    // Auto-register cannot infer a pool; no-op.
}

#[cfg(test)]
mod tests {
    use super::*;
    use candid::Principal;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id])
    }

    #[test]
    fn cap_reached_blocks_creation() {
        let pool = PoolName::from("pool_cap");
        let pid = p(1);
        CanisterShardRegistry::register(pid, pool.clone(), 5);

        let policy = ShardPolicy {
            initial_capacity: 5,
            max_shards: 1,
            growth_threshold_pct: 80,
        };
        let m = metrics_for_pool(&pool);
        assert_eq!(m.active_count, 1);
        let err = ensure_can_create(&m, &policy).unwrap_err();
        assert!(err.to_string().contains("cap reached"));
    }

    #[test]
    fn below_threshold_blocks_when_capacity_exists() {
        let pool = PoolName::from("pool_thr");
        let pid = p(2);
        CanisterShardRegistry::register(pid, pool.clone(), 10);
        // increase used count by assigning an item to this shard
        let item = p(42);
        CanisterShardRegistry::assign_item_to_partition(item, &pool, pid).unwrap();

        let policy = ShardPolicy {
            initial_capacity: 10,
            max_shards: 64,
            growth_threshold_pct: 80,
        };
        let m = metrics_for_pool(&pool);
        assert_eq!(m.total_capacity, 10);
        assert_eq!(m.total_used, 1);
        let err = ensure_can_create(&m, &policy).unwrap_err();
        assert!(err.to_string().contains("below growth threshold"));
    }

    #[test]
    fn allows_creation_when_no_capacity_exists() {
        let pool = PoolName::from("pool_empty");
        // no registrations for this pool → total_capacity = 0
        let policy = ShardPolicy {
            initial_capacity: 10,
            max_shards: 64,
            growth_threshold_pct: 80,
        };
        let m = metrics_for_pool(&pool);
        assert_eq!(m.total_capacity, 0);
        assert!(ensure_can_create(&m, &policy).is_ok());
    }
}
