//! Placement-only drain and exact releases by the approved hub.
use super::{ChildId, Ledger, STATE, api};
use canic::sharding::{ShardError, ShardTarget};

impl Ledger {
    fn target_slot(&self, target: &ShardTarget) -> Result<ChildId, ShardError> {
        let slot = ChildId::SHARDS
            .into_iter()
            .find(|s| s.index() as u32 == target.allocation.slot)
            .ok_or(ShardError::UnknownShard)?;
        let index = slot.index();
        if self.generations[index] != Some(target.allocation.generation)
            || self.principals[index].as_deref() != Some(&target.shard.to_text())
        {
            return Err(ShardError::GenerationChanged);
        }
        Ok(slot)
    }
}

/// Record the parent’s completed release before a separate controller effect.
/// # Panics
/// Requires an opened Root with its placement ledger.
pub fn release_child_allocation(
    principal: ic_platform_types::Principal,
    allocation: canic::sharding::Allocation,
) -> Result<(), canic::sharding::ReleaseError> {
    use canic::sharding::ReleaseError;
    let target = ShardTarget {
        allocation,
        shard: principal,
    };
    STATE.with_borrow_mut(|state| {
        let state = state.as_mut().expect("open Root");
        let (owner, slot) = state
            .pools
            .shards
            .values()
            .find_map(|ledger| Some((&ledger.approval.hub, ledger.target_slot(&target).ok()?)))
            .ok_or(ReleaseError::UnknownAllocation)?;
        if owner != &api::caller().to_text() {
            return Err(ReleaseError::Unauthorized);
        }
        if state.pools.withdrawn(slot) || state.pools.evacuated.contains(&slot) {
            return if state.pools.draining.get(&slot) == Some(&target.allocation.generation) {
                Ok(())
            } else {
                Err(ReleaseError::UnknownAllocation)
            };
        }
        state
            .pools
            .draining
            .insert(slot, target.allocation.generation);
        state.pools.evacuated.insert(slot);
        state
            .pool
            .put(&super::Key::Drain(slot), &target.allocation.generation);
        Ok(())
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
