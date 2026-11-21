//! Keep paid refill outcomes outside the disposable canister.
use super::super::{MANAGEMENT_LIMITS, create_args, management, reserved_cycles};
use super::{STATE, api};
use canic::sharding::{
    Allocation,
    standby::{Asset, Refill, Source},
};
use canic_management_protocol::{CREATION_CYCLES, children::ChildId, stored::MAX_SUBNET_CHILDREN};
use ic_platform_protocol::management as mgmt;
use ic_platform_types::Principal;

pub mod policy;
pub mod transfer;

pub(super) type Inventory = Vec<(Source, Asset)>;

#[derive(serde::Serialize, serde::Deserialize)]
pub(super) struct Returned {
    pub target: canic::sharding::ShardTarget,
    pub deadline: u64,
    pub balance: u128,
}

/// Can a new receiver grant be bound to this principal at this stage of its lifecycle?
/// # Panics
/// Requires opened Root custody.
#[must_use]
pub fn permits_enrollment(receiver: Principal) -> bool {
    STATE.with_borrow(|s| {
        let pools = &s.as_ref().unwrap().pools;
        !pools.retirements.values().any(|r| {
            r.target().shard == receiver
                && r.phase != canic_management_protocol::stored::RetirementPhase::Returned
        }) && !pools.standby.iter().any(|(_, asset)| match asset {
            Asset::Transfer(id, ..)
            | Asset::AwaitingRelease(id)
            | Asset::AwaitingHandoff(id)
            | Asset::Quarantined(id, _)
            | Asset::Empty(id)
            | Asset::Available(id, _)
            | Asset::Released(id, _)
            | Asset::Reset(id, ..) => *id == receiver,
            _ => false,
        })
    })
}

// Check the original operator again when a custody observation resumes.
fn authorize_change(controller: Principal) -> Result<(), String> {
    if !api::is_controller(&controller) {
        return Err("Root controller required".into());
    }
    super::admit_pool_change(false).map_err(|e| format!("{e:?}"))
}

/// Observe retained approvals without changing the pool.
/// # Panics
/// Requires the Root controller and an opened ledger.
#[must_use]
pub fn inspect() -> Vec<(Source, Asset)> {
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
    let original = STATE.with_borrow(|s| {
        s.as_ref()
            .unwrap()
            .pools
            .standby
            .iter()
            .find_map(|(source, asset)| {
                let Source::Refill(approval) = source else {
                    return None;
                };
                (approval.sequence == input.sequence).then(|| {
                    if approval == &input {
                        Ok(asset.clone())
                    } else {
                        Err("refill approval differs".into())
                    }
                })
            })
    });
    if let Some(result) = original {
        return result;
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
    let index = STATE.with_borrow_mut(|s| {
        let state = s.as_mut().unwrap();
        if state.pools.capacity.is_none() || state.pools.full() {
            return Err("no approved subnet capacity");
        }
        if state
            .pools
            .standby
            .iter()
            .filter(|(source, _)| matches!(source, Source::Refill(_)))
            .count()
            != usize::from(input.sequence)
            || input.sequence >= MAX_SUBNET_CHILDREN
        {
            return Err("refill sequence outside retained capacity");
        }
        let index = state.pools.standby.len();
        state
            .pools
            .standby
            .push((Source::Refill(input.clone()), Asset::Issued));
        state.pool.set(state.pools.encode());
        Ok(index)
    })?;
    create_issued(index, Source::Refill(input)).await
}

// Dispatch only after the exact debit and inventory slot have been retained.
async fn create_issued(index: usize, source: Source) -> Result<Asset, String> {
    let (Source::Refill(input) | Source::Automatic(input)) = &source else {
        unreachable!()
    };
    let bytes = management("create_canister", create_args(), u128::from(input.cycles))
        .await
        .map_err(|e| format!("refill issued: {e:?}"))?;
    let id = mgmt::decode_created(&bytes, MANAGEMENT_LIMITS)
        .map_err(|_| "refill issued: unreadable principal")?
        .canister_id;
    STATE.with_borrow_mut(|s| {
        let state = s.as_mut().unwrap();
        let entry = &mut state.pools.standby[index];
        assert_eq!(entry, &(source, Asset::Issued));
        entry.1 = Asset::Empty(id);
        log_surplus(&state.pools);
        state.pool.set(state.pools.encode());
    });
    Ok(Asset::Empty(id))
}

pub(in crate::engine) fn claimed(slot: ChildId) -> Option<Principal> {
    STATE.with_borrow(|s| s.as_ref().unwrap().pools.standby_claim(slot))
}

pub(super) fn available(asset: &Asset) -> Option<(bool, u128, Principal)> {
    match asset {
        Asset::Empty(id) => Some((true, 0, *id)),
        Asset::Available(id, balance) => Some((false, *balance, *id)),
        _ => None,
    }
}

pub(in crate::engine::sharding) fn reserve(
    pools: &mut super::Pools,
    slot: ChildId,
    generation: [u8; 32],
) {
    if let Some(((_, _, id), asset)) = pools
        .standby
        .iter_mut()
        .filter_map(|(_, asset)| Some((available(asset)?, asset)))
        .min_by_key(|(key, _)| *key)
    {
        *asset = Asset::Claimed(
            id,
            Allocation {
                slot: slot.index() as u32,
                generation,
            },
        );
    } else if pools.reserve_policy.is_some() {
        canic::logging::record(
            canic::logging::Level::Warn,
            "reserve",
            &format!(
                "pool empty; child {} scheduled for approved creation",
                slot.index()
            ),
        );
    }
}

// Called on admission to the ready inventory, never by an observation or retry.
fn log_surplus(pools: &super::Pools) {
    let Some(policy) = &pools.reserve_policy else {
        return;
    };
    let ready = pools
        .standby
        .iter()
        .filter(|(_, asset)| available(asset).is_some())
        .count();
    if ready > usize::from(policy.minimum_available) {
        canic::logging::record(
            canic::logging::Level::Info,
            "reserve",
            &format!(
                "{ready} ready canisters exceed target {}; surplus retained for operator action",
                policy.minimum_available
            ),
        );
    }
}

pub(super) fn released_binding(asset: &Asset) -> Option<(Principal, &Allocation)> {
    match asset {
        Asset::Released(id, allocation) | Asset::Reset(id, allocation, ..) => {
            Some((*id, allocation))
        }
        _ => None,
    }
}

/// Observe the original hub's receipt and withdraw the released allocation from worker lookup.
/// # Errors
/// Requires an exact live claim and an affirmative reply from the recorded worker hub.
/// # Panics
/// Requires Root's controller and opened custody.
pub async fn confirm_release(target: canic::sharding::ShardTarget) -> Result<(), String> {
    let controller = api::caller();
    assert!(api::is_controller(&controller));
    let claimed = Asset::Claimed(target.shard, target.allocation.clone());
    let released = Asset::Released(target.shard, target.allocation.clone());
    let slot = ChildId::placement(target.allocation.slot as usize).ok_or("unknown worker slot")?;
    if !ChildId::INSTANCES.contains(&slot) {
        return Err("released worker allocation required".into());
    }
    let already_released = |pools: &super::Pools| {
        pools.returns.iter().any(|r| r.target == target)
            || pools
                .standby
                .iter()
                .any(|(_, a)| released_binding(a) == Some((target.shard, &target.allocation)))
    };
    let hub = match STATE.with_borrow(|s| {
        let state = s.as_ref().unwrap();
        if already_released(&state.pools) {
            return Ok(None);
        }
        let (workers, actual) = state
            .pools
            .worker_at(slot)
            .ok_or("missing worker allowance")?;
        workers.minimum_reuse_cycles()?;
        if actual.generation != target.allocation.generation
            || actual.principal.as_deref() != Some(&target.shard.to_text())
        {
            return Err("worker binding changed");
        }
        Ok(Some(
            Principal::from_text(&workers.grant.hub).expect("approved worker hub"),
        ))
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
    authorize_change(controller)?;
    STATE.with_borrow_mut(|s| {
        let state = s.as_mut().unwrap();
        if already_released(&state.pools) {
            return Ok(());
        }
        if let Some((_, asset)) = state.pools.standby.iter_mut().find(|(_, a)| a == &claimed) {
            *asset = released;
        } else {
            let status = &state
                .journal
                .0
                .get(&slot)
                .ok_or("missing worker journal")?
                .0;
            if status.phase != canic_management_protocol::Phase::Ready
                || status.attempts[0] != 1
                || status.worker.as_deref() != Some(target.shard.to_text().as_str())
            {
                return Err("missing original paid worker creation".into());
            }
            state
                .pools
                .standby
                .push((Source::Worker(target.clone()), released));
        }
        state.pool.set(state.pools.encode());
        Ok(())
    })
}

mod reset;
pub use reset::reset;

mod shard;
pub(in crate::engine) use shard::Retirement;
#[cfg(feature = "role-issuer")]
pub use shard::retire;
