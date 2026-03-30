use crate::view::ShardPlacement;
use canic_core::cdk::candid::Principal;

///
/// PoolMetrics
/// Aggregated metrics for a pool
///

#[derive(Clone, Copy, Debug)]
pub struct PoolMetrics {
    pub active_count: u32,
}

#[must_use]
pub fn compute_pool_metrics(pool: &str, entries: &[(Principal, ShardPlacement)]) -> PoolMetrics {
    let mut active = 0;

    for (_, e) in entries {
        if e.capacity > 0 && e.pool.as_str() == pool {
            active += 1;
        }
    }

    PoolMetrics {
        active_count: active,
    }
}
