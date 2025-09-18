use crate::{
    memory::shard::{ShardEntry, ShardRegistry, ShardRegistryView},
    types::CanisterType,
};

///
/// PoolMetrics
///

#[derive(Clone, Copy, Debug)]
pub struct PoolMetrics {
    pub utilization_pct: u32,
    pub active_count: u32,
    pub total_capacity: u64,
    pub total_used: u64,
}

///
/// ShardMetrics
///

#[derive(Clone, Copy, Debug)]
pub struct ShardMetrics {
    pub capacity: u32,
    pub count: u32,
    pub utilization_pct: u32,
}

impl ShardMetrics {
    #[must_use]
    pub fn from_entry(entry: &ShardEntry) -> Self {
        let utilization = if entry.capacity == 0 {
            0
        } else {
            ((u64::from(entry.count) * 100) / u64::from(entry.capacity)).min(100) as u32
        };

        Self {
            capacity: entry.capacity,
            count: entry.count,
            utilization_pct: utilization,
        }
    }

    #[must_use]
    pub const fn has_capacity(&self) -> bool {
        self.count < self.capacity
    }
}

impl ShardRegistry {
    #[must_use]
    pub fn metrics(pool: &str) -> PoolMetrics {
        let view: ShardRegistryView = Self::export();
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

    /// Compute metrics for a single shard
    #[must_use]
    pub fn shard_metrics(shard_pid: &candid::Principal) -> Option<ShardMetrics> {
        Self::with(|s| s.get_entry(shard_pid)).map(|e| ShardMetrics::from_entry(&e))
    }

    /// Find latest `created_at_secs` for all shards of given type
    #[must_use]
    pub fn last_created_at_for_type(ty: &CanisterType) -> u64 {
        Self::with(|s| {
            s.all_entries().iter().fold(0, |last, (_, e)| {
                if &e.canister_type == ty {
                    last.max(e.created_at_secs)
                } else {
                    last
                }
            })
        })
    }
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;
    use candid::Principal;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    #[test]
    fn shard_metrics_computation() {
        let entry = ShardEntry {
            canister_type: CanisterType::new("alpha"),
            capacity: 10,
            count: 3,
            created_at_secs: 123,
            pool: "poolX".into(),
        };
        let metrics = ShardMetrics::from_entry(&entry);

        assert_eq!(metrics.capacity, 10);
        assert_eq!(metrics.count, 3);
        assert_eq!(metrics.utilization_pct, 30);
        assert!(metrics.has_capacity());
    }

    #[test]
    fn pool_metrics_computation() {
        ShardRegistry::clear();
        ShardRegistry::create(p(1), "poolA", &CanisterType::new("alpha"), 10);
        ShardRegistry::create(p(2), "poolA", &CanisterType::new("alpha"), 20);

        // Simulate usage: assign 3 tenants to shard 1, 10 tenants to shard 2
        for i in 0..3 {
            ShardRegistry::assign("poolA", p(10 + i), p(1)).unwrap();
        }
        for i in 0..10 {
            ShardRegistry::assign("poolA", p(20 + i), p(2)).unwrap();
        }

        let m = ShardRegistry::metrics("poolA");

        assert_eq!(m.active_count, 2);
        assert_eq!(m.total_capacity, 30);
        assert_eq!(m.total_used, 13);
        assert_eq!(m.utilization_pct, (13 * 100 / 30) as u32);
    }

    #[test]
    fn last_created_at_for_type_picks_latest() {
        ShardRegistry::clear();
        let ty = CanisterType::new("alpha");

        ShardRegistry::create(p(1), "poolA", &ty, 5);
        std::thread::sleep(std::time::Duration::from_millis(5)); // ensure clock moves
        ShardRegistry::create(p(2), "poolA", &ty, 5);

        let t = ShardRegistry::last_created_at_for_type(&ty);
        assert!(t > 0);
    }

    #[test]
    fn shard_metrics_none_for_missing_shard() {
        ShardRegistry::clear();
        assert!(ShardRegistry::shard_metrics(&p(99)).is_none());
    }
}
