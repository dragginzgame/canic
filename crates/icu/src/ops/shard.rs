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
    let spec = get_pool_spec_for_child(hub_type, canister_type)?;

    ensure_item_assignment_internal(
        &spec.canister_type,
        &spec.pool_name,
        item,
        spec.policy,
        spec.policy.initial_capacity,
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

// ---- Lifecycle helpers: drain, rebalance, decommission ----

struct PoolSpec {
    pool_name: PoolName,
    canister_type: CanisterType,
    policy: ShardPolicy,
}

fn get_pool_spec(hub_type: &CanisterType, pool_name: &str) -> Result<PoolSpec, Error> {
    let cfg = Config::try_get_canister(hub_type)
        .ok()
        .and_then(|c| c.sharder)
        .ok_or_else(|| Error::custom("sharding disabled"))?;

    let pool_cfg = cfg
        .pools
        .get(pool_name)
        .ok_or_else(|| Error::custom("shard pool not found"))?;

    Ok(PoolSpec {
        pool_name: PoolName::from(pool_name),
        canister_type: pool_cfg.canister_type.clone(),
        policy: ShardPolicy {
            initial_capacity: pool_cfg.policy.initial_capacity,
            max_shards: pool_cfg.policy.max_shards,
            growth_threshold_pct: pool_cfg.policy.growth_threshold_pct,
        },
    })
}

fn get_pool_spec_for_child(
    hub_type: &CanisterType,
    child_type: &CanisterType,
) -> Result<PoolSpec, Error> {
    let cfg = Config::try_get_canister(hub_type)
        .ok()
        .and_then(|c| c.sharder)
        .ok_or_else(|| Error::custom("sharding disabled"))?;

    let (pool_name, pool_cfg) = cfg
        .pools
        .iter()
        .find(|(_, p)| &p.canister_type == child_type)
        .ok_or_else(|| Error::custom("shard pool not found"))?;

    Ok(PoolSpec {
        pool_name: PoolName::from(pool_name.as_str()),
        canister_type: pool_cfg.canister_type.clone(),
        policy: ShardPolicy {
            initial_capacity: pool_cfg.policy.initial_capacity,
            max_shards: pool_cfg.policy.max_shards,
            growth_threshold_pct: pool_cfg.policy.growth_threshold_pct,
        },
    })
}

/// Drain up to `limit` items from `shard_pid` to other shards in the same pool,
/// creating a new shard if none are available to receive.
///
/// Note: this updates only the assignment registry; it does not migrate
/// application data/state between shards. Coordinate data moves separately.
pub async fn drain_shard(
    hub_type: &CanisterType,
    pool_name: &str,
    shard_pid: Principal,
    limit: u32,
) -> Result<u32, Error> {
    let spec = get_pool_spec(hub_type, pool_name)?;
    let items = CanisterShardRegistry::items_for_shard(&spec.pool_name, shard_pid);
    let mut moved = 0u32;

    for item in items.into_iter().take(limit as usize) {
        // Try to assign to any shard except the source
        if CanisterShardRegistry::assign_item_best_effort_excluding(
            item,
            &spec.pool_name,
            shard_pid,
        )
        .is_some()
        {
            moved = moved.saturating_add(1);
            continue;
        }

        // No existing capacity: create a new shard canister and assign to it
        crate::log!(
            crate::Log::Info,
            "💎 shard: drain creating receiver for item={item} from={from}",
            from = shard_pid
        );

        let response = create_canister_request::<Vec<u8>>(&spec.canister_type, None).await?;
        let new_pid = response.new_canister_pid;
        CanisterShardRegistry::register(
            new_pid,
            spec.pool_name.clone(),
            spec.policy.initial_capacity,
        );

        CanisterShardRegistry::assign_item_to_partition(item, &spec.pool_name, new_pid)?;
        moved = moved.saturating_add(1);
    }

    Ok(moved)
}

/// Rebalance a pool by moving up to `limit` items from the most loaded shard(s)
/// to the least loaded ones; does not create new shards.
///
/// Note: this changes assignments only; migrate application data separately.
pub fn rebalance_pool(pool_name: &str, limit: u32) -> Result<u32, Error> {
    let pool = PoolName::from(pool_name);
    let mut moved = 0u32;

    for _ in 0..limit {
        // Snapshot registry for this iteration
        let view = CanisterShardRegistry::export();
        let mut candidates: Vec<(Principal, u64, u32)> = view
            .iter()
            .filter(|(_, e)| e.pool.as_ref() == Some(&pool))
            .map(|(pid, e)| (*pid, e.load_bps(), e.count))
            .collect();

        if candidates.len() < 2 {
            break; // nothing to balance
        }

        candidates.sort_by_key(|(_, load, count)| (*load, *count));
        let receiver = candidates.first().copied();
        let donor = candidates.last().copied();

        let (recv_pid, recv_load, _recv_count, donor_pid, donor_load, _donor_count) =
            match (receiver, donor) {
                (Some(r), Some(d)) if r.0 != d.0 && d.2 > 0 => (r.0, r.1, r.2, d.0, d.1, d.2),
                _ => break,
            };

        // Stop when loads are already balanced or inverted; prevents oscillation.
        if donor_load <= recv_load {
            break;
        }

        // Pick one item from donor and move to receiver
        let items = CanisterShardRegistry::items_for_shard(&pool, donor_pid);
        let Some(item) = items.first().copied() else {
            break;
        };
        if let Err(e) = CanisterShardRegistry::assign_item_to_partition(item, &pool, recv_pid) {
            crate::log!(
                crate::Log::Warn,
                "💎 shard: rebalance failed for item={item}: {e}"
            );
            break;
        }
        moved = moved.saturating_add(1);
    }

    Ok(moved)
}

/// Decommission a shard from a pool (must be empty).
///
/// Note: this does not delete the canister; it removes the shard from
/// the registry after verifying it holds no assignments.
pub fn decommission_shard(shard_pid: Principal) -> Result<(), Error> {
    CanisterShardRegistry::remove_shard(shard_pid)
}

/// Dry-run (plan) using config: never creates; returns current metrics and decision.
pub fn plan_pool(
    hub_type: &CanisterType,
    canister_type: &CanisterType,
    item: Principal,
) -> Result<ShardPlan, Error> {
    // Already assigned? Resolve pool by child type via shared helper
    let spec = get_pool_spec_for_child(hub_type, canister_type)?;
    let pool = spec.pool_name.clone();
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

    // Policy from config
    let max_shards = spec.policy.max_shards;
    let growth_threshold_pct = spec.policy.growth_threshold_pct;

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

///
/// TESTS
///

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

    #[test]
    fn rebalance_moves_from_heavy_to_light() {
        // Setup: two shards in same pool, one heavy, one empty
        let pool = PoolName::from("pool_rebalance");
        let a = Principal::from_slice(&[201]);
        let b = Principal::from_slice(&[202]);
        crate::memory::canister::shard::CanisterShardRegistry::register(a, pool.clone(), 4);
        crate::memory::canister::shard::CanisterShardRegistry::register(b, pool.clone(), 4);

        // Put 3 items on A
        for i in 1..=3u8 {
            let it = Principal::from_slice(&[50 + i]);
            crate::memory::canister::shard::CanisterShardRegistry::assign_item_to_partition(
                it, &pool, a,
            )
            .unwrap();
        }

        // Rebalance one move
        let moved = rebalance_pool("pool_rebalance", 1).unwrap();
        assert_eq!(moved, 1);

        // Verify counts closer
        let view = crate::memory::canister::shard::CanisterShardRegistry::export();
        let mut ca = 0u32;
        let mut cb = 0u32;
        for (pid, e) in view {
            if e.pool.as_ref() == Some(&pool) {
                if pid == a {
                    ca = e.count;
                }
                if pid == b {
                    cb = e.count;
                }
            }
        }
        assert_eq!(ca, 2);
        assert_eq!(cb, 1);
    }
}
