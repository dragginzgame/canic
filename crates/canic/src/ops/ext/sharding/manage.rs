//! Management and maintenance layer for sharding.
//!
//! This module provides utilities for rebalancing tenants across shards,
//! decommissioning empty shards, and exporting registry state.
//!
//! It operates entirely on existing registry data and does not
//! create new shards or modify pool policies.

use crate::{
    Error, Log, log,
    memory::ext::sharding::{ShardingRegistry, ShardingRegistryView},
};
use candid::Principal;

/// High-level maintenance operations for sharding.
///
/// - [`rebalance_pool`] redistributes tenants among shards.
/// - [`decommission_shard`] safely removes an empty shard.
/// - [`export_registry`] returns a snapshot view of the registry.
pub struct ShardingManageOps;

impl ShardingManageOps {
    /// Export a read-only snapshot of all shard pools and tenants.
    #[must_use]
    pub fn export_registry() -> ShardingRegistryView {
        ShardingRegistry::export()
    }

    /// Decommission an empty shard by removing it from the registry.
    ///
    /// This operation **does not** call the management canister to delete the
    /// underlying canister—it only updates registry state.
    pub fn decommission_shard(shard_pid: Principal) -> Result<(), Error> {
        ShardingRegistry::remove(shard_pid)?;
        log!(Log::Ok, "🗑️ shard.decommission: {shard_pid}");
        Ok(())
    }

    /// Rebalance tenants across shards in a pool without creating new shards.
    ///
    /// Moves tenants from the most loaded shard(s) to the least loaded ones,
    /// up to the given `limit`.
    pub fn rebalance_pool(pool: &str, limit: u32) -> Result<u32, Error> {
        let mut moved = 0u32;

        for _ in 0..limit {
            let view = ShardingRegistry::export();

            // Gather all shards in the target pool
            let mut candidates: Vec<(Principal, u64, u32, u64)> = view
                .iter()
                .filter(|(_, e)| e.pool == pool)
                .map(|(pid, e)| {
                    (
                        *pid,
                        e.load_bps().unwrap_or(u64::MAX),
                        e.count,
                        e.created_at,
                    )
                })
                .collect();

            if candidates.len() < 2 {
                // Not enough shards to balance
                break;
            }

            // Sort by (load, count, created)
            candidates.sort_by_key(|(_, load, count, created)| (*load, *count, *created));

            let (recv_pid, recv_load, _, _) = candidates.first().copied().unwrap();
            let (donor_pid, donor_load, donor_count, _) = candidates.last().copied().unwrap();

            // No work to do if balanced or donor has no tenants
            if donor_pid == recv_pid || donor_count == 0 || donor_load <= recv_load {
                break;
            }

            if let Some(tenant) = ShardingRegistry::tenants_in_shard(pool, donor_pid)
                .first()
                .cloned()
                && ShardingRegistry::assign(pool, tenant.clone(), recv_pid).is_ok()
            {
                log!(
                    Log::Info,
                    "🔀 shard.rebalance: tenant={tenant} donor={donor_pid} → recv={recv_pid}"
                );
                moved += 1;
            }
        }

        Ok(moved)
    }
}
