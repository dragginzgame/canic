//! Placement-only drain and exact releases by the approved hub.
use super::{ChildId, Ledger, STATE, api};
use canic::sharding::{Key, ShardTarget};

impl Ledger {
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
pub async fn inspect_shard(
    shard: ic_platform_types::Principal,
) -> Result<canic::sharding::ShardStatus, String> {
    if !api::is_controller(&api::caller()) {
        return Err("controller required".into());
    }
    let principal = shard.to_text();
    let (status, local) = STATE.with_borrow(|s| {
        let pools = &s.as_ref().expect("open Root").pools;
        let (slot, ledger) = ChildId::SHARDS
            .into_iter()
            .find_map(|slot| {
                let ledger = pools.shard_at(slot)?;
                (!pools.archived(slot)
                    && ledger.principals[slot.index()].as_deref() == Some(&principal))
                .then_some((slot, ledger))
            })
            .ok_or("unknown shard")?;
        Ok::<_, String>((
            canic::sharding::ShardStatus {
                target: ShardTarget {
                    shard,
                    allocation: canic::sharding::Allocation {
                        slot: slot.index() as u32,
                        generation: ledger.generations[slot.index()]
                            .expect("installed shard allocation"),
                    },
                },
                draining: pools.draining.contains_key(&slot),
                live_placements: pools.remaining(slot, ledger),
            },
            hub_binding(pools, ledger),
        ))
    })?;
    if let Some((hub, pool)) = local {
        return hub_status(hub, pool, &status.target, false).await;
    }
    Ok(status)
}

impl super::Pools {
    pub(super) fn remaining(&self, slot: ChildId, ledger: &Ledger) -> u32 {
        if self.evacuated.contains(&slot) {
            0
        } else {
            ledger.counts[slot.index()]
        }
    }
}

fn hub_binding(
    pools: &super::Pools,
    ledger: &Ledger,
) -> Option<(ic_platform_types::Principal, String)> {
    pools.local_routes.contains(&ledger.approval.pool).then(|| {
        (
            ledger.approval.hub.parse().expect("approved hub"),
            ledger.approval.pool.clone(),
        )
    })
}

async fn hub_status(
    hub: ic_platform_types::Principal,
    pool: String,
    target: &ShardTarget,
    drain: bool,
) -> Result<canic::sharding::ShardStatus, String> {
    let status = canic::method::call::<canic::sharding::HubShardStatus>(
        hub,
        10,
        &(pool, target.clone(), drain),
    )
    .await
    .map_err(|_| "hub occupancy unavailable")??;
    if status.target != *target || (drain && !status.draining) {
        return Err("hub allocation differs".into());
    }
    Ok(status)
}

/// Fence future placements and return the current number of live keys.
/// # Errors
/// Requires the current controller and the exact installed allocation.
/// # Panics
/// Requires an opened Root.
pub async fn begin_shard_drain(target: &ShardTarget) -> Result<u32, String> {
    let controller = api::caller();
    if !api::is_controller(&controller) {
        return Err("controller required".into());
    }
    let (count, local) = STATE.with_borrow_mut(|s| {
        let state = s.as_mut().expect("open Root");
        let (ledger, slot) = state
            .pools
            .shards
            .values()
            .find_map(|ledger| Some((ledger, ledger.target_slot(target).ok()?)))
            .ok_or("assignment changed")?;
        let local = hub_binding(&state.pools, ledger);
        let count = ledger.counts[slot.index()];
        if let std::collections::btree_map::Entry::Vacant(entry) = state.pools.draining.entry(slot)
        {
            entry.insert(target.allocation.generation);
            state.pool.set(state.pools.encode());
        }
        Ok::<_, String>((count, local))
    })?;
    let Some((hub, pool)) = local else {
        return Ok(count);
    };
    let status = hub_status(hub, pool.clone(), target, true).await?;
    if !api::is_controller(&controller) {
        return Err("controller changed".into());
    }
    STATE.with_borrow_mut(|s| {
        let s = s.as_mut().expect("open Root");
        let ledger = s.pools.shards.get(&pool).ok_or("pool changed")?;
        let slot = ledger.target_slot(target)?;
        if ledger.approval.hub != hub.to_text()
            || s.pools.draining.get(&slot) != Some(&target.allocation.generation)
        {
            return Err("drain changed".into());
        }
        if status.live_placements == 0 && s.pools.evacuated.insert(slot) {
            s.pool.set(s.pools.encode());
        }
        Ok(status.live_placements)
    })
}

/// Ensure this key no longer belongs to the exact draining allocation.
/// # Errors
/// Requires the configured hub. Refuses a different allocation or an active shard.
/// # Panics
/// Requires a consistent opened placement ledger.
pub fn release_shard(key: &Key, target: &ShardTarget) -> Result<(), String> {
    let caller = api::caller().to_text();
    STATE.with_borrow_mut(|s| {
        let state = s.as_mut().expect("open Root");
        let (ledger, slot) = state
            .pools
            .shards
            .values_mut()
            .filter(|ledger| ledger.approval.hub == caller)
            .find_map(|ledger| {
                let slot = ledger.target_slot(target).ok()?;
                Some((ledger, slot))
            })
            .ok_or("assignment changed")?;
        if state.pools.local_routes.contains(&ledger.approval.pool) {
            return Err("account releases belong to the hub for this pool".into());
        }
        if state.pools.draining.get(&slot) != Some(&target.allocation.generation) {
            return Err("shard is not draining".into());
        }
        let key = (ledger.namespace, *key);
        match state.claims.get(&key) {
            None => Ok(()),
            Some(current) if usize::from(current) != slot.index() => {
                Err("key assigned elsewhere".into())
            }
            Some(_) => {
                state.claims.remove(&key);
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
    let principal = subject.to_text();
    pools.draining.iter().any(|(slot, expected)| {
        let Some(ledger) = pools.shard_at(*slot) else {
            return false;
        };
        expected == generation
            && pools.remaining(*slot, ledger) == 0
            && ledger.principals[slot.index()].as_deref() == Some(principal.as_str())
    })
}
