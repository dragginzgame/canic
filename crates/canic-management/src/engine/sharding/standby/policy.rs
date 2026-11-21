//! One controller approval owns initial fill, replenishment and external handoff.
use super::{Asset, Refill, STATE, Source, api, available, create_issued, reserved_cycles};
use crate::engine::{MANAGEMENT_LIMITS, sharding};
use canic_management_protocol::{CREATION_CYCLES, stored::StandbyPolicy};
use ic_platform_protocol::{management as mgmt, placement};
use ic_platform_runtime::call::{Call, Outcome, Wait};
use ic_platform_types::Principal;
use ic_timers::{Lifetime, Observation, Once, OnceDemand, OnceNext, OnceResult};
use std::cell::RefCell;

thread_local! {
    static TIMER: RefCell<Option<Once>> = const { RefCell::new(None) };
}

pub(in crate::engine) fn validate(p: &StandbyPolicy, cap: u8) -> Result<(), String> {
    if p.root != api::canister_self().to_text()
        || p.minimum_available > cap
        || p.maximum_creations > cap
        || p.imports.len() > usize::from(cap)
        || p.expires_at_ns == 0
    {
        return Err("reserve bounds differ from the addressed Root and subnet ceiling".into());
    }
    let mut ids = std::collections::BTreeSet::new();
    for candidate in &p.imports {
        let text = &candidate.principal;
        if let Some(previous) = &candidate.previous_root {
            let old: Principal = previous.parse().map_err(|_| "invalid former Root")?;
            if old.as_slice().last() != Some(&1)
                || old.to_text() != *previous
                || old == api::canister_self()
                || previous == text
            {
                return Err(format!("invalid former Root {previous} for {text}"));
            }
        }
        let id: Principal = text.parse().map_err(|_| "invalid import principal")?;
        if id.to_text() != *text
            || id == api::canister_self()
            || id.as_slice().last() != Some(&1)
            || !ids.insert(id)
        {
            return Err(format!("invalid or duplicate import {text}"));
        }
    }
    Ok(())
}

/// Reserve approved external slots and arm one demand-driven fill.
/// # Errors
/// Refuses changed terms, competing custody and capacity overflow.
/// # Panics
/// Requires an opened Root and its controller.
pub fn configure(policy: StandbyPolicy) -> Result<(), String> {
    assert!(api::is_controller(&api::caller()));
    sharding::admit_pool_change(false).map_err(|e| format!("{e:?}"))?;
    STATE.with_borrow_mut(|s| -> Result<(), String> {
        let s = s.as_mut().unwrap();
        let pools = &mut s.pools;
        let cap = pools
            .capacity
            .as_ref()
            .ok_or("configure subnet capacity first")?
            .max_children;
        validate(&policy, cap)?;
        // Appending named candidates cannot renew paid creations or change a
        // prior principal's release authority. An older approval is a replay.
        let added = if let Some(old) = &pools.reserve_policy {
            let same_allowance = policy.root == old.root
                && policy.minimum_available == old.minimum_available
                && policy.maximum_creations == old.maximum_creations
                && policy.expires_at_ns == old.expires_at_ns
                && policy.minimum_import_cycles == old.minimum_import_cycles;
            if !same_allowance {
                return Err("reserve approval differs".into());
            }
            if old.imports.starts_with(&policy.imports) {
                return Ok(());
            }
            policy
                .imports
                .strip_prefix(old.imports.as_slice())
                .ok_or("import list must retain original candidates")?
        } else {
            policy.imports.as_slice()
        };
        if pools.occupied_slots() + added.len() > usize::from(cap) {
            return Err("imports exceed remaining physical capacity".into());
        }
        if !added.is_empty()
            && (pools
                .standby
                .iter()
                .any(|(_, a)| matches!(a, Asset::Issued))
                || s.journal
                    .0
                    .values()
                    .any(|j| j.0.attempts[0] > 0 && j.0.worker.is_none()))
        {
            return Err("unresolved creation prevents proving import uniqueness".into());
        }
        for candidate in added {
            let text = &candidate.principal;
            if s.journal
                .0
                .values()
                .any(|j| j.0.worker.as_ref() == Some(text))
                || s.artifact
                    .0
                    .values()
                    .filter_map(|a| a.sources())
                    .any(|a| a.creation.reference.store == *text)
                || pools.standby.iter().any(|(_, a)| match a {
                    Asset::Issued => false,
                    Asset::Transfer(id, ..)
                    | Asset::AwaitingRelease(id)
                    | Asset::Empty(id)
                    | Asset::Quarantined(id, _)
                    | Asset::AwaitingHandoff(id)
                    | Asset::Available(id, _)
                    | Asset::Claimed(id, _)
                    | Asset::Released(id, _)
                    | Asset::Reset(id, ..) => id.to_text() == *text,
                })
            {
                return Err("import principal already belongs to this fleet".into());
            }
        }
        for candidate in added {
            let text = &candidate.principal;
            let id = text.parse().expect("validated principal");
            pools.standby.push((
                Source::Imported(id),
                if candidate.previous_root.is_some() {
                    Asset::AwaitingRelease(id)
                } else {
                    Asset::AwaitingHandoff(id)
                },
            ));
        }
        pools.reserve_policy = Some(policy);
        s.pool.set(pools.encode());
        Ok(())
    })?;
    wake();
    Ok(())
}

/// Uncertified aggregate report; uses the same eligibility rules as the refill timer.
#[must_use]
pub fn spare_counts() -> Option<canic_management_protocol::stored::SpareCounts> {
    use canic_management_protocol::stored::{RefillStatus, SpareCounts};
    STATE.with_borrow(|s| {
        let pools = &s.as_ref()?.pools;
        let p = pools.reserve_policy.as_ref()?;
        let mut ready = 0;
        let mut issued = 0;
        let mut paid = 0;
        for (source, asset) in &pools.standby {
            ready += u8::from(available(asset).is_some());
            issued += u8::from(matches!(asset, Asset::Issued));
            paid += u8::from(matches!(source, Source::Automatic(_)));
        }
        let refill = if ready >= p.minimum_available {
            RefillStatus::AtTarget
        } else if ready + issued >= p.minimum_available {
            RefillStatus::AwaitingCreation
        } else if paid >= p.maximum_creations {
            RefillStatus::AllowanceSpent
        } else if pools.full() {
            RefillStatus::SubnetFull
        } else if api::time() >= p.expires_at_ns {
            RefillStatus::Expired
        } else if sharding::admit_pool_change(false).is_err() {
            RefillStatus::Occupied
        } else if ic_platform_runtime::raw::liquid_cycle_balance()
            < reserved_cycles() + u128::from(CREATION_CYCLES)
        {
            RefillStatus::Underfunded
        } else {
            RefillStatus::Due
        };
        Some(SpareCounts {
            ready,
            min_available_spares: p.minimum_available,
            creations_used: paid,
            maximum_creations: p.maximum_creations,
            refill,
        })
    })
}

fn demand() -> Option<Refill> {
    let counts = spare_counts()?;
    if counts.refill != canic_management_protocol::stored::RefillStatus::Due {
        return None;
    }
    STATE.with_borrow(|s| {
        Some(Refill {
            root: api::canister_self(),
            sequence: counts.creations_used,
            cycles: CREATION_CYCLES,
            expires_at_ns: s.as_ref()?.pools.reserve_policy.as_ref()?.expires_at_ns,
        })
    })
}

async fn step() -> OnceResult {
    let mut observation = Observation::NoWork;
    if let Some(input) = demand() {
        let source = Source::Automatic(input);
        let index = STATE.with_borrow_mut(|s| {
            let s = s.as_mut().unwrap();
            let index = s.pools.standby.len();
            s.pools.standby.push((source.clone(), Asset::Issued));
            s.pool.set(s.pools.encode());
            index
        });
        observation = if create_issued(index, source).await.is_ok() {
            Observation::Success
        } else {
            Observation::ExpectedFailure
        };
    }
    OnceResult {
        observation,
        next: demand().map_or(OnceNext::Stop, |_| OnceNext::At(api::time())),
    }
}

pub(in crate::engine) fn wake() {
    if demand().is_none() {
        return;
    }
    TIMER.with_borrow_mut(|t| {
        let timer = t.get_or_insert_with(|| {
            ic_timers::bind_once(["canic", "reserve", "fill"], Lifetime::Retained, step)
                .expect("reserve timer")
        });
        timer
            .set(OnceDemand::At(api::time()))
            .expect("schedule reserve demand");
    });
}

pub(super) async fn subnet(id: Principal) -> Result<Principal, String> {
    let registry = "rwlgt-iiaaa-aaaaa-aaaaa-cai".parse().expect("NNS Registry");
    let args =
        placement::encode_get_subnet_for_canister_args(id, 256).map_err(|_| "invalid lookup")?;
    let reply = Call::new(
        registry,
        "get_subnet_for_canister",
        Wait::Bounded { seconds: 10 },
    )
    .owned_args(args)
    .execute()
    .await
    .map_err(|_| "registry unavailable")?;
    if let Outcome::Reply(bytes) = reply.into_parts().0
        && let Ok(placement::PlacementResult::Ok(row)) =
            placement::decode_get_subnet_for_canister_response(&bytes, MANAGEMENT_LIMITS)
        && let Some(subnet) = row.subnet_id
    {
        return Ok(subnet);
    }
    Err("registry did not establish placement".into())
}

/// Restrict import requests to the controller and declared former Roots.
/// # Errors
/// Refuses callers without a handoff role.
pub fn import_caller() -> Result<(), &'static str> {
    let caller = api::caller();
    if api::is_controller(&caller)
        || STATE.with_borrow(|s| {
            s.as_ref()
                .and_then(|s| s.pools.reserve_policy.as_ref())
                .is_some_and(|p| {
                    p.imports
                        .iter()
                        .any(|i| i.previous_root.as_deref() == Some(&caller.to_text()))
                })
        })
    {
        Ok(())
    } else {
        Err("controller or former Root required")
    }
}

/// Read the candidate-specific release record without advancing the handoff.
/// # Errors
/// Only the former Root declared for this candidate may observe it.
pub fn handoff_status(id: Principal) -> Result<Asset, String> {
    STATE.with_borrow(|s| {
        let pools = &s.as_ref().ok_or("Root not opened")?.pools;
        if !pools.reserve_policy.as_ref().is_some_and(|policy| {
            policy.imports.iter().any(|i| {
                i.principal == id.to_text()
                    && i.previous_root.as_deref() == Some(&api::caller().to_text())
            })
        }) {
            return Err("no reservation for this source Root and spare".into());
        }
        pools
            .standby
            .iter()
            .find(|(source, _)| source == &Source::Imported(id))
            .map(|(_, asset)| asset.clone())
            .ok_or_else(|| "missing import custody".into())
    })
}

/// Observe sole empty custody, retain a two-hour cutoff, and admit after a second observation.
/// # Errors
/// Requires a named surplus principal on Root's subnet with a sufficient balance.
/// # Panics
/// Requires an opened Root and its controller.
pub async fn import(id: Principal) -> Result<Asset, String> {
    let caller = api::caller();
    let (index, before, floor, former) = STATE.with_borrow(|s| {
        let pools = &s.as_ref().unwrap().pools;
        let policy = pools
            .reserve_policy
            .as_ref()
            .ok_or("reserve is not configured")?;
        let candidate = policy
            .imports
            .iter()
            .find(|i| i.principal == id.to_text())
            .ok_or("principal is not approved for import")?;
        let (index, (_, asset)) = pools
            .standby
            .iter()
            .enumerate()
            .find(|(_, (source, _))| source == &Source::Imported(id))
            .expect("approved candidate");
        Ok::<_, String>((
            index,
            asset.clone(),
            policy.minimum_import_cycles,
            candidate.previous_root.as_deref() == Some(&caller.to_text()),
        ))
    })?;
    if former && matches!(before, Asset::AwaitingRelease(_)) {
        sharding::admit_pool_change(false).map_err(|e| format!("{e:?}"))?;
        return STATE.with_borrow_mut(|s| {
            let s = s.as_mut().unwrap();
            s.pools.standby[index].1 = Asset::AwaitingHandoff(id);
            s.pool.set(s.pools.encode());
            Ok(Asset::AwaitingHandoff(id))
        });
    }
    if !api::is_controller(&caller) {
        return if former {
            Ok(before)
        } else {
            Err("wrong former Root".into())
        };
    }
    match before {
        Asset::AwaitingHandoff(_) => {}
        Asset::Quarantined(_, deadline) if api::time() >= deadline => {}
        _ => return Ok(before),
    }
    let root_subnet = subnet(api::canister_self()).await?;
    let candidate_subnet = subnet(id).await?;
    if root_subnet != candidate_subnet {
        return Err(format!(
            "cannot import {id}: registry reports subnet {candidate_subnet}, but Root requires {root_subnet}"
        ));
    }
    let args = mgmt::CanisterArgs { canister_id: id }
        .encode(256)
        .expect("principal");
    let (status, balance) = super::reset::observe_target(&args).await?;
    if !api::is_controller(&caller) {
        return Err("import caller no longer controls Root".into());
    }
    if status.module_hash.is_some() {
        return Err(format!(
            "cannot import {id}: candidate still has installed code"
        ));
    }
    if status.status != ic_platform_protocol::management_snapshots::Status::Running {
        return Err(format!(
            "cannot import {id}: candidate must be running, observed {:?}",
            status.status
        ));
    }
    if balance < u128::from(floor.get()) {
        return Err(format!(
            "cannot import {id}: balance {balance} cycles is below the approved minimum {}",
            floor.get()
        ));
    }
    sharding::admit_pool_change(false).map_err(|e| format!("{e:?}"))?;
    STATE.with_borrow_mut(|s| {
        let s = s.as_mut().unwrap();
        let entry = &mut s.pools.standby[index].1;
        if *entry == before {
            *entry = if matches!(before, Asset::AwaitingHandoff(_)) {
                Asset::Quarantined(
                    id,
                    api::time().saturating_add(canic::topology::attestation::LIFETIME_NS),
                )
            } else {
                Asset::Available(id, balance)
            };
            let result = entry.clone();
            if available(&result).is_some() {
                super::log_surplus(&s.pools);
            }
            s.pool.set(s.pools.encode());
            Ok(result)
        } else {
            Ok(entry.clone())
        }
    })
}
