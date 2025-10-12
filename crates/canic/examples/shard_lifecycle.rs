//! Demonstrates sharding lifecycle mechanics using the in-memory registry.
//!
//! This runs natively (not on IC) and exercises:
//! - registering shards in a pool
//! - assigning tenants and observing imbalance
//! - rebalancing within the pool
//! - draining a shard by reassigning tenants
//! - decommissioning an empty shard

use candid::Principal;
use canic::{
    memory::ext::sharding::ShardingRegistry, ops::ext::sharding::rebalance_pool,
    types::CanisterType,
};

const fn p(id: u8) -> Principal {
    // Deterministic test principal (29 identical bytes)
    Principal::from_slice(&[id; 29])
}

fn main() {
    canic::runtime::init_eager_tls();
    canic::memory::registry::init_memory();
    ShardingRegistry::clear();

    let pool = "demo";
    let shard_a = p(1);
    let shard_b = p(2);
    let shard_ty = CanisterType::new("demo_shard");

    // Register two shards with capacity 4 each
    ShardingRegistry::create(shard_a, pool, &shard_ty, 4);
    ShardingRegistry::create(shard_b, pool, &shard_ty, 4);

    // Heavily load shard B with tenants
    for i in 10..14u8 {
        let tenant = p(i);
        ShardingRegistry::assign(pool, tenant, shard_b).expect("assign tenant to shard");
    }

    // Snapshot before rebalance
    let before = ShardingRegistry::export();
    println!("Before: {before:#?}");

    // Rebalance using least-loaded selection (no creation)
    let moved = rebalance_pool(pool, 10).expect("rebalance pool");
    println!("Rebalanced moves: {moved}");

    // Drain shard B by moving remaining tenants off it using exclusion helper
    let tenants = ShardingRegistry::tenants_in_shard(pool, shard_b);
    for tenant in tenants {
        ShardingRegistry::assign_best_effort_excluding(pool, tenant, shard_b)
            .expect("expected available capacity");
    }

    // Now B should be empty; decommission it
    ShardingRegistry::remove(shard_b).expect("remove empty shard");

    let after = ShardingRegistry::export();
    println!("After: {after:#?}");
}
