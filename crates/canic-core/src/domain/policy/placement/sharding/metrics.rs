use crate::{cdk::candid::Principal, view::placement::sharding::ShardPlacement};

///
/// PoolMetrics
/// Aggregated metrics for a pool
///

#[derive(Clone, Copy, Debug)]
pub struct PoolMetrics {
    pub utilization_pct: u32,
    pub active_count: u32,
    pub total_capacity: u64,
    pub total_used: u64,
}

/// compute_pool_metrics
#[must_use]
pub fn compute_pool_metrics(pool: &str, entries: &[(Principal, ShardPlacement)]) -> PoolMetrics {
    let mut active = 0;
    let mut cap = 0;
    let mut used = 0;

    for (_, e) in entries {
        if e.capacity > 0 && e.pool.as_str() == pool {
            active += 1;
            cap += u64::from(e.capacity);
            used += u64::from(e.count);
        }
    }

    let utilization = if cap == 0 {
        0
    } else {
        ((used * 100) / cap).min(100) as u32
    };

    PoolMetrics {
        utilization_pct: utilization,
        active_count: active,
        total_capacity: cap,
        total_used: used,
    }
}
