//! Keep paid refill outcomes outside the disposable canister.
use super::super::{MANAGEMENT_LIMITS, create_args, management, reserved_cycles};
use super::{STATE, api};
use canic::sharding::{
    Allocation,
    standby::{Asset, Refill},
};
use canic_management_protocol::{CREATION_CYCLES, children::ChildId, stored::MAX_SUBNET_CHILDREN};
use ic_platform_protocol::management as mgmt;
use ic_platform_types::Principal;

/// Observe retained approvals without changing the pool.
/// # Panics
/// Requires the Root controller and an opened ledger.
#[must_use]
pub fn inspect() -> Vec<(Refill, Asset)> {
    assert!(api::is_controller(&api::caller()));
    STATE.with_borrow(|s| s.as_ref().unwrap().pools.standby.clone())
}

/// Create a fresh empty canister under one expiring, replayable approval.
/// # Errors
/// Refuses changed or out-of-sequence authority, insufficient funding or exhausted capacity.
/// # Panics
/// Requires the Root controller and opened placement state.
pub async fn refill(input: Refill) -> Result<Asset, String> {
    assert!(api::is_controller(&api::caller()));
    if input.root != api::canister_self() || input.cycles != CREATION_CYCLES {
        return Err("wrong Root or creation debit".into());
    }
    if let Some((approval, asset)) = inspect().get(usize::from(input.sequence)) {
        return if approval == &input {
            Ok(asset.clone())
        } else {
            Err("refill approval differs".into())
        };
    }
    super::admit_pool_change(false).map_err(|e| format!("{e:?}"))?;
    let now = api::time();
    if input.expires_at_ns <= now || input.expires_at_ns - now > 600_000_000_000 {
        return Err("refill requires a deadline within ten minutes".into());
    }
    if ic_platform_runtime::raw::liquid_cycle_balance()
        < reserved_cycles() + u128::from(input.cycles)
    {
        return Err("insufficient Root reserve".into());
    }
    STATE.with_borrow_mut(|s| {
        let state = s.as_mut().unwrap();
        if state.pools.capacity.is_none() || state.pools.full() {
            return Err("no approved subnet capacity");
        }
        if state.pools.standby.len() != usize::from(input.sequence)
            || input.sequence >= MAX_SUBNET_CHILDREN
        {
            return Err("refill sequence outside retained capacity");
        }
        state.pools.standby.push((input.clone(), Asset::Issued));
        state.pool.set(state.pools.encode());
        Ok(())
    })?;
    let bytes = management("create_canister", create_args(), u128::from(input.cycles))
        .await
        .map_err(|e| format!("refill issued: {e:?}"))?;
    let id = mgmt::decode_created(&bytes, MANAGEMENT_LIMITS)
        .map_err(|_| "refill issued: unreadable principal")?
        .canister_id;
    STATE.with_borrow_mut(|s| {
        let state = s.as_mut().unwrap();
        let entry = &mut state.pools.standby[usize::from(input.sequence)];
        assert_eq!(entry, &(input, Asset::Issued));
        entry.1 = Asset::Empty(id);
        state.pool.set(state.pools.encode());
    });
    Ok(Asset::Empty(id))
}

pub(in crate::engine) fn claimed(slot: ChildId) -> Option<Principal> {
    STATE.with_borrow(|s| s.as_ref().unwrap().pools.standby_claim(slot))
}

pub(super) fn reserve(pools: &mut super::Pools, slot: ChildId, generation: [u8; 32]) {
    if let Some((_, asset)) = pools
        .standby
        .iter_mut()
        .find(|(_, asset)| matches!(asset, Asset::Empty(_)))
        && let Asset::Empty(id) = *asset
    {
        *asset = Asset::Claimed(
            id,
            Allocation {
                slot: slot.index() as u32,
                generation,
            },
        );
    }
}

/// Observe the original hub's receipt and withdraw a released spare from worker lookup.
/// # Errors
/// Requires an exact live claim and an affirmative reply from the recorded worker hub.
/// # Panics
/// Requires Root's controller and opened custody.
pub async fn confirm_release(target: canic::sharding::ShardTarget) -> Result<(), String> {
    assert!(api::is_controller(&api::caller()));
    let claimed = Asset::Claimed(target.shard, target.allocation.clone());
    let released = Asset::Released(target.shard, target.allocation.clone());
    let slot = ChildId::placement(target.allocation.slot as usize).ok_or("unknown worker slot")?;
    let (index, hub) = match STATE.with_borrow(|s| {
        let state = s.as_ref().unwrap();
        if state.pools.standby.iter().any(|(_, a)| a == &released) {
            return Ok(None);
        }
        let index = state
            .pools
            .standby
            .iter()
            .position(|(_, a)| a == &claimed)
            .ok_or("unknown standby assignment")?;
        let workers = state
            .pools
            .workers
            .as_ref()
            .ok_or("missing worker allowance")?;
        let actual = workers.claim_at(slot).ok_or("unknown worker")?;
        if actual.generation != target.allocation.generation
            || actual.principal.as_deref() != Some(&target.shard.to_text())
        {
            return Err("worker binding changed");
        }
        Ok(Some((
            index,
            Principal::from_text(&workers.grant.hub).expect("approved worker hub"),
        )))
    })? {
        Some(binding) => binding,
        None => return Ok(()),
    };
    if !canic::method::call::<canic::sharding::standby::WorkerReleased>(hub, 10, &target)
        .await
        .map_err(|e| format!("hub release unconfirmed: {e:?}"))?
    {
        return Err("hub still owns worker".into());
    }
    STATE.with_borrow_mut(|s| {
        let state = s.as_mut().unwrap();
        let asset = &mut state.pools.standby[index].1;
        if *asset == released {
            return Ok(());
        }
        if *asset != claimed {
            return Err("standby assignment changed".into());
        }
        *asset = released;
        state.pool.set(state.pools.encode());
        Ok(())
    })
}
