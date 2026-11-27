//! Subnet-wide admission and exact count-only changes for the two placement families.
use super::{Pools, STATE, admit_pool_change, api};
use canic::sharding::standby::{Asset, TransferPhase};
use canic_management_protocol::{Refusal, stored};

impl Pools {
    pub(super) fn occupied_slots(&self) -> usize {
        // A returned allocation remains in its original claim history.
        self.used() + self.held() - self.returns.len()
    }

    pub(super) fn used(&self) -> usize {
        self.permanent.len()
            + self.projects.len()
            + self.workers.values().map(|w| w.claims.len()).sum::<usize>()
            + self
                .shards
                .values()
                .map(|l| l.generations.iter().flatten().count())
                .sum::<usize>()
    }

    fn held(&self) -> usize {
        self.standby
            .iter()
            .filter(|(_, asset)| match asset {
                Asset::Transfer(
                    _,
                    _,
                    _,
                    TransferPhase::Reserved | TransferPhase::ControllersIssued,
                )
                | Asset::Empty(_)
                | Asset::Available(..)
                | Asset::AwaitingRelease(_)
                | Asset::AwaitingHandoff(_)
                | Asset::Quarantined(..)
                | Asset::Issued => true,
                Asset::Transfer(_, _, _, TransferPhase::Confirmed)
                | Asset::Claimed(..)
                | Asset::Released(..)
                | Asset::Reset(..) => false,
            })
            .count()
    }

    pub(in crate::engine) fn capacity_approved(&self) -> bool {
        self.capacity.is_some()
    }

    pub(super) fn full(&self) -> bool {
        self.occupied_slots() >= self.subnet_limit() as usize
    }
}

pub(super) fn valid_capacity(capacity: &stored::SubnetCapacity) -> bool {
    capacity.root == api::canister_self().to_text()
        && stored::SubnetCapacity::permits_limit(capacity.max_children, capacity.allow_large_subnet)
}

/// Approve one shared lifetime ceiling for this subnet owner.
/// # Errors
/// Refuses conflicting grants, except a controller-approved increase above the default backstop.
/// # Panics
/// Requires controller authority and valid Root state.
pub fn configure_subnet_capacity(capacity: stored::SubnetCapacity) -> Result<(), Refusal> {
    assert!(api::is_controller(&api::caller()));
    if !valid_capacity(&capacity) {
        return Err(Refusal::Binding);
    }
    admit_pool_change(true)?;
    STATE.with_borrow_mut(|s| {
        let state = s.as_mut().unwrap();
        if let Some(old) = &state.pools.capacity {
            if old == &capacity {
                return Ok(());
            }
            if !capacity.allow_large_subnet
                || capacity.max_children <= stored::DEFAULT_SUBNET_CHILD_LIMIT
                || capacity.max_children <= old.max_children
            {
                return Err(Refusal::Occupied);
            }
            state.pools.capacity = Some(capacity);
            state.pool.put(
                &super::Key::Capacity,
                state.pools.capacity.as_ref().unwrap(),
            );
            return Ok(());
        }
        if let Some(initial) = state.metadata.bootstrap
            && capacity.max_children > initial.max_children
            && !(capacity.allow_large_subnet
                && capacity.max_children > stored::DEFAULT_SUBNET_CHILD_LIMIT)
        {
            return Err(Refusal::Occupied);
        }
        let permanent: std::collections::BTreeSet<_> = state
            .journal
            .rows()
            .keys()
            .copied()
            .filter(|slot| {
                slot.service_index().is_some()
                    && state.journal.rows()[slot].0.approval.is_some()
                    && state.pools.shard_at(*slot).is_none()
            })
            .collect();
        if state.pools.used() + permanent.len() > capacity.max_children as usize
            || state
                .pools
                .shards
                .values()
                .any(|l| u32::from(l.approval.max_shards) > capacity.max_children)
            || state
                .pools
                .instances
                .as_ref()
                .is_some_and(|p| u32::from(p.max_instances) > capacity.max_children)
        {
            return Err(Refusal::Occupied);
        }
        state.pools.capacity = Some(capacity);
        for slot in permanent {
            super::retain_permanent(state, slot);
        }
        state.pool.put(
            &super::Key::Capacity,
            state.pools.capacity.as_ref().unwrap(),
        );
        Ok(())
    })
}

/// Raise a pool count against its exact previous terms, retaining all existing liabilities.
/// # Errors
/// A stale grant, expired authority, or non-increase refuses.
/// # Panics
/// Requires the controller and an opened Root.
pub fn increase_pool_limit(change: stored::IncreasePoolLimit) -> Result<(), Refusal> {
    assert!(api::is_controller(&api::caller()));
    STATE.with_borrow_mut(|s| {
        let state = s.as_mut().unwrap();
        let cap = state
            .pools
            .capacity
            .as_ref()
            .ok_or(Refusal::Binding)?
            .max_children;
        let (initial, previous, next, history, deadline, family_limit, key) = match change {
            stored::IncreasePoolLimit::Shards(mut old, next) => {
                let ledger = state
                    .pools
                    .shards
                    .get_mut(&old.pool)
                    .ok_or(Refusal::Binding)?;
                let initial = &ledger.approval;
                let previous = std::mem::replace(&mut old.max_shards, initial.max_shards);
                if &old != initial {
                    return Err(Refusal::Binding);
                }
                (
                    initial.max_shards,
                    previous,
                    next,
                    &mut ledger.increases,
                    initial.expires_at_ns,
                    stored::MAX_SHARDS,
                    super::Key::Shard(old.pool),
                )
            }
            stored::IncreasePoolLimit::Instances(mut old, next) => {
                let initial = state.pools.instances.as_ref().ok_or(Refusal::Binding)?;
                let previous = std::mem::replace(&mut old.max_instances, initial.max_instances);
                if &old != initial {
                    return Err(Refusal::Binding);
                }
                (
                    initial.max_instances,
                    previous,
                    next,
                    &mut state.pools.instance_increases,
                    initial.expires_at_ns,
                    stored::MAX_INSTANCES,
                    super::Key::InstanceIncreases,
                )
            }
        };
        if next <= previous || u32::from(next) > cap || next > family_limit {
            return Err(Refusal::Binding);
        }
        let mut current = initial;
        for &applied in history.iter() {
            if (current, applied) == (previous, next) {
                return Ok(());
            }
            current = applied;
        }
        if current != previous {
            return Err(Refusal::Occupied);
        }
        if api::time() >= deadline {
            return Err(Refusal::Time);
        }
        history.push(next);
        match key {
            super::Key::Shard(ref name) => state
                .pool
                .put(&super::Key::Shard(name.clone()), &state.pools.shards[name]),
            _ => state.pool.put(&key, &state.pools.instance_increases),
        }
        Ok(())
    })
}
