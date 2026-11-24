//! Placement-only drain and exact releases by the approved hub.
use super::{ChildId, Ledger, STATE, api};
use canic::sharding::ShardTarget;

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
    let (target, hub, pool) = STATE.with_borrow(|s| {
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
            ShardTarget {
                shard,
                allocation: canic::sharding::Allocation {
                    slot: slot.index() as u32,
                    generation: ledger.generations[slot.index()]
                        .expect("installed shard allocation"),
                },
            },
            ledger.approval.hub.parse().expect("approved hub"),
            ledger.approval.pool.clone(),
        ))
    })?;
    hub_status(hub, pool, &target, false).await
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
    let (hub, pool) = STATE.with_borrow_mut(|s| {
        let state = s.as_mut().expect("open Root");
        let (ledger, slot) = state
            .pools
            .shards
            .values()
            .find_map(|ledger| Some((ledger, ledger.target_slot(target).ok()?)))
            .ok_or("assignment changed")?;
        let binding = (
            ledger.approval.hub.parse().expect("approved hub"),
            ledger.approval.pool.clone(),
        );
        if let std::collections::btree_map::Entry::Vacant(entry) = state.pools.draining.entry(slot)
        {
            entry.insert(target.allocation.generation);
            state.pool.set(state.pools.encode());
        }
        Ok::<_, String>(binding)
    })?;
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
            && pools.evacuated.contains(slot)
            && ledger.principals[slot.index()].as_deref() == Some(principal.as_str())
    })
}
