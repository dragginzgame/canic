use crate::{cdk::candid::Principal, view::placement::sharding::ShardPlacement};

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

    for (_, entry) in entries {
        if entry.capacity > 0 && entry.pool.as_str() == pool {
            active += 1;
        }
    }

    PoolMetrics {
        active_count: active,
    }
}
