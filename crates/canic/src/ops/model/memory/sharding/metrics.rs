use crate::model::memory::sharding::ShardingRegistry;

///
/// PoolMetrics
/// Aggregated metrics for a pool (derived view, therefore ops).
///

#[derive(Clone, Copy, Debug)]
pub struct PoolMetrics {
    pub utilization_pct: u32,
    pub active_count: u32,
    pub total_capacity: u64,
    pub total_used: u64,
}

/// Compute pool-level metrics from the registry.
#[must_use]
pub fn pool_metrics(pool: &str) -> PoolMetrics {
    let view = ShardingRegistry::export();
    let mut active = 0;
    let mut cap = 0;
    let mut used = 0;

    for (_, e) in &view {
        if e.capacity > 0 && e.pool == pool {
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

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        model::memory::sharding::ShardKey,
        ops::model::memory::sharding::ShardingRegistryOps,
        types::{CanisterType, Principal},
    };

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    #[test]
    fn pool_metrics_computation() {
        ShardingRegistry::clear();
        ShardingRegistryOps::create(p(1), "poolA", 0, &CanisterType::new("alpha"), 10).unwrap();
        ShardingRegistryOps::create(p(2), "poolA", 1, &CanisterType::new("alpha"), 20).unwrap();

        ShardingRegistry::with_mut(|core| {
            core.insert_assignment(ShardKey::new("poolA", "t1"), p(1));
            core.insert_assignment(ShardKey::new("poolA", "t2"), p(1));
            core.insert_assignment(ShardKey::new("poolA", "t3"), p(2));
            core.insert_assignment(ShardKey::new("poolA", "t4"), p(2));
            core.insert_assignment(ShardKey::new("poolA", "t5"), p(2));

            if let Some(mut entry) = core.get_entry(&p(1)) {
                entry.count = 2;
                core.insert_entry(p(1), entry);
            }
            if let Some(mut entry) = core.get_entry(&p(2)) {
                entry.count = 3;
                core.insert_entry(p(2), entry);
            }
        });

        let m = pool_metrics("poolA");
        assert_eq!(m.active_count, 2);
        assert_eq!(m.total_capacity, 30);
        assert_eq!(m.total_used, 5);
        assert_eq!(m.utilization_pct, (5 * 100 / 30) as u32);
    }
}
