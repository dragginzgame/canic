//! Placement-only drain and exact releases by the approved hub.
use super::{ChildId, Ledger, SLOT_COUNT, STATE, api, hub};
use canic::sharding::{Key, ShardTarget};

impl Ledger<SLOT_COUNT> {
    fn target_slot(&self, target: &ShardTarget) -> Result<ChildId, String> {
        let slot = ChildId::SHARDS
            .into_iter()
            .find(|s| s.index() as u32 == target.allocation.slot)
            .ok_or("unknown shard slot")?;
        let index = slot.index();
        if self.generations[index] != Some(target.allocation.generation)
            || self.principals[index].as_deref() != Some(&target.shard.to_text())
        {
            return Err("assignment changed".into());
        }
        Ok(slot)
    }
}

/// Inspect one installed shard without changing admission or issuing effects.
/// # Errors
/// Requires the current controller and a shard principal recorded by this pool.
/// # Panics
/// Requires an opened Root with consistent allocation custody.
pub fn inspect_shard(
    shard: ic_platform_types::Principal,
) -> Result<canic::sharding::ShardStatus, String> {
    if !api::is_controller(&api::caller()) {
        return Err("controller required".into());
    }
    let principal = shard.to_text();
    STATE.with_borrow(|s| {
        let pools = &s.as_ref().expect("open Root").pools;
        let ledger = pools.shards.as_ref().ok_or("missing shard pool")?;
        let slot = ChildId::SHARDS
            .into_iter()
            .find(|s| ledger.principals[s.index()].as_deref() == Some(principal.as_str()))
            .ok_or("unknown shard")?;
        Ok(canic::sharding::ShardStatus {
            target: ShardTarget {
                shard,
                allocation: canic::sharding::Allocation {
                    slot: slot.index() as u32,
                    generation: ledger.generations[slot.index()]
                        .expect("installed shard allocation"),
                },
            },
            draining: pools.draining.contains_key(&slot),
            live_placements: ledger.counts[slot.index()],
        })
    })
}

/// Fence future placements and return the current number of live keys.
/// # Errors
/// Requires the current controller and the exact installed allocation.
/// # Panics
/// Requires an opened Root.
pub fn begin_shard_drain(target: &ShardTarget) -> Result<u16, String> {
    if !api::is_controller(&api::caller()) {
        return Err("controller required".into());
    }
    STATE.with_borrow_mut(|s| {
        let state = s.as_mut().expect("open Root");
        let ledger = state.pools.shards.as_ref().ok_or("missing shard pool")?;
        let slot = ledger.target_slot(target)?;
        let count = ledger.counts[slot.index()];
        if let std::collections::btree_map::Entry::Vacant(entry) = state.pools.draining.entry(slot)
        {
            entry.insert(target.allocation.generation);
            state.pool.set(state.pools.encode());
        }
        Ok(count)
    })
}

/// Ensure this key no longer belongs to the exact draining allocation.
/// # Errors
/// Requires the configured hub. Refuses a different allocation or an active shard.
/// # Panics
/// Requires a consistent opened placement ledger.
pub fn release_shard(key: &Key, target: &ShardTarget) -> Result<(), String> {
    hub()?;
    STATE.with_borrow_mut(|s| {
        let state = s.as_mut().expect("open Root");
        let ledger = state.pools.shards.as_mut().expect("approved hub");
        let slot = ledger.target_slot(target)?;
        if state.pools.draining.get(&slot) != Some(&target.allocation.generation) {
            return Err("shard is not draining".into());
        }
        match ledger.claims.get(key) {
            None => Ok(()),
            Some(current) if *current != slot => Err("key assigned elsewhere".into()),
            Some(_) => {
                ledger.claims.remove(key);
                ledger.counts[slot.index()] -= 1;
                state.pool.set(state.pools.encode());
                Ok(())
            }
        }
    })
}

#[cfg(feature = "role-issuer")]
pub(super) fn closed_assignment(
    pools: &super::Pools,
    subject: ic_platform_types::Principal,
    generation: &[u8; 32],
) -> bool {
    let Some(ledger) = &pools.shards else {
        return false;
    };
    let principal = subject.to_text();
    pools.draining.iter().any(|(slot, expected)| {
        expected == generation
            && ledger.counts[slot.index()] == 0
            && ledger.principals[slot.index()].as_deref() == Some(principal.as_str())
    })
}
