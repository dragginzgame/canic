//! Demonstrates sharding lifecycle mechanics using the in-memory registry.
//!
//! This runs natively (not on IC) and exercises:
//! - registering shards in a pool
//! - assigning tenants and observing imbalance
//! - rebalancing within the pool
//! - draining a shard (simulated via exclusion helper)
//! - decommissioning an empty shard

use candid::Principal;
use icu::memory::canister::shard::ShardRegistry;
use icu::ops::shard::rebalance_pool;

fn p(id: u8) -> Principal {
    // Fake deterministic principal for demo
    Principal::self_authenticating(vec![id])
}

fn main() {
    let pool = "demo";
    let a = p(1);
    let b = p(2);

    // Register two shards with capacity 4 each
    ShardRegistry::register(a, pool, 4);
    ShardRegistry::register(b, pool, 4);

    // Heavily load shard B with tenants
    for i in 10..14u8 {
        let tenant = p(i);
        ShardRegistry::assign_tenant_to_shard(pool, tenant, b).unwrap();
    }

    // Snapshot before
    let before = ShardRegistry::export();
    println!("Before: {before:#?}");

    // Rebalance using least-loaded selection (no creation)
    let moved = rebalance_pool(pool, 10).unwrap();
    println!("Rebalanced moves: {moved}");

    // Drain shard B by moving remaining tenants off it using exclusion helper
    let tenants = ShardRegistry::tenants_for_shard(pool, b);
    for tenant in tenants {
        let _ = ShardRegistry::assign_tenant_best_effort(pool, tenant, Some(b))
            .expect("expected available capacity");
    }

    // Now B should be empty; decommission it
    ShardRegistry::remove_shard(b).unwrap();

    let after = ShardRegistry::export();
    println!("After: {after:?}");
}
