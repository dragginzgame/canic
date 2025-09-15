//! Demonstrates sharding lifecycle mechanics using the in-memory registry.
//!
//! This runs natively (not on IC) and exercises:
//! - registering shards in a pool
//! - assigning items and observing imbalance
//! - rebalancing within the pool
//! - draining a shard (simulated via exclusion helper)
//! - decommissioning an empty shard
use candid::Principal;
use icu::memory::canister::shard::CanisterShardRegistry;
use icu::memory::canister::shard::PoolName;

const fn p(id: u8) -> Principal {
    Principal::from_slice(&[id])
}

fn main() {
    let pool = PoolName::from("demo");
    let a = p(1);
    let b = p(2);

    // Register two shards with capacity 4 each
    CanisterShardRegistry::register(a, pool.clone(), 4);
    CanisterShardRegistry::register(b, pool.clone(), 4);

    // Heavily load shard B
    for i in 10..14u8 {
        let item = p(i);
        CanisterShardRegistry::assign_item_to_partition(item, &pool, b).unwrap();
    }

    // Snapshot before
    let before = CanisterShardRegistry::export();
    println!("Before: {before:?}");

    // Rebalance using least-loaded selection (no creation)
    let moved = icu::ops::shard::rebalance_pool("demo", 10).unwrap();
    println!("Rebalanced moves: {moved}");

    // Drain shard B by moving remaining items off it using exclusion helper
    let items = CanisterShardRegistry::items_for_shard(&pool, b);
    for item in items {
        let _ = CanisterShardRegistry::assign_item_best_effort_excluding(item, &pool, b)
            .expect("expected available capacity");
    }

    // Now B should be empty; decommission it
    icu::memory::canister::shard::CanisterShardRegistry::remove_shard(b).unwrap();

    let after = CanisterShardRegistry::export();
    println!("After: {after:?}");
}
