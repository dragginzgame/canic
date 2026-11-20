//! Subnet-wide admission and exact count-only changes for the two placement families.
use super::{STATE, admit_pool_change, api};
use canic_management_protocol::{Refusal, stored};

pub(super) fn valid_capacity(capacity: &stored::SubnetCapacity) -> bool {
    capacity.root == api::canister_self().to_text()
        && (1..=stored::MAX_SUBNET_CHILDREN).contains(&capacity.max_children)
}

/// Approve one shared lifetime ceiling for this subnet owner.
/// # Errors
/// Refuses another ceiling, existing conflicting management custody, or a limit below usage.
/// # Panics
/// Requires controller authority and valid Root state.
pub fn configure_capacity(capacity: stored::SubnetCapacity) -> Result<(), Refusal> {
    assert!(api::is_controller(&api::caller()));
    if !valid_capacity(&capacity) {
        return Err(Refusal::Binding);
    }
    admit_pool_change(true)?;
    STATE.with_borrow_mut(|s| {
        let state = s.as_mut().unwrap();
        if let Some(old) = &state.pools.capacity {
            return if old == &capacity {
                Ok(())
            } else {
                Err(Refusal::Occupied)
            };
        }
        let permanent = canic_management_protocol::children::ChildId::ALL.map(|slot| {
            state.journal.0[&slot].0.approval.is_some()
                && state
                    .pools
                    .shards
                    .as_ref()
                    .is_none_or(|l| l.generations[slot.index()].is_none())
        });
        if state.pools.used() + permanent.iter().filter(|&&used| used).count()
            > usize::from(capacity.max_children)
            || state
                .pools
                .shards
                .as_ref()
                .is_some_and(|l| l.approval.max_shards > capacity.max_children)
            || state
                .pools
                .instances
                .as_ref()
                .is_some_and(|p| p.max_instances > capacity.max_children)
        {
            return Err(Refusal::Occupied);
        }
        state.pools.permanent = permanent;
        state.pools.capacity = Some(capacity);
        state.pool.set(state.pools.encode());
        Ok(())
    })
}

/// Raise a pool count against its exact previous terms, retaining all existing liabilities.
/// # Errors
/// A stale grant, expired authority, or non-increase refuses.
/// # Panics
/// Requires the controller and an opened Root.
pub fn increase_pool(change: stored::IncreasePool) -> Result<(), Refusal> {
    assert!(api::is_controller(&api::caller()));
    STATE.with_borrow_mut(|s| {
        let state = s.as_mut().unwrap();
        let cap = state
            .pools
            .capacity
            .as_ref()
            .ok_or(Refusal::Binding)?
            .max_children;
        let (initial, previous, next, history, deadline) = match change {
            stored::IncreasePool::Shards(mut old, next) => {
                let initial = &state
                    .pools
                    .shards
                    .as_ref()
                    .ok_or(Refusal::Binding)?
                    .approval;
                let previous = std::mem::replace(&mut old.max_shards, initial.max_shards);
                if &old != initial {
                    return Err(Refusal::Binding);
                }
                (
                    initial.max_shards,
                    previous,
                    next,
                    &mut state.pools.shard_increases,
                    initial.expires_at_ns,
                )
            }
            stored::IncreasePool::Instances(mut old, next) => {
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
                )
            }
        };
        if next <= previous || next > cap {
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
        state.pool.set(state.pools.encode());
        Ok(())
    })
}
