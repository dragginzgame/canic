//! One controller-approved pool over the existing Store-backed child effects.
use super::{STATE, api, delivery, snapshot};
use crate::{
    placement::{self, score, select},
    storage::Artifact,
};
use canic::sharding::{Allocation, Key, Placement};
use canic_management_protocol::{
    Approval, CREATION_CYCLES, Phase, Refusal,
    children::ChildId,
    stored::{self, ShardPool},
};
use ic_platform_types::Principal;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub(crate) const BYTES: usize = 64 * 1024;
const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: BYTES,
    allocation: BYTES * 16,
    nodes: BYTES,
    depth: 24,
};

#[derive(Serialize, Deserialize)]
pub(super) struct Ledger {
    approval: ShardPool,
    claims: BTreeMap<Key, ChildId>,
    counts: [u16; 2],
    generations: [Option<[u8; 32]>; 2],
    principals: [Option<String>; 2],
}

pub(super) fn encode(value: &Option<Ledger>) -> Vec<u8> {
    ic_wire::encode(value, ic_wire::Encoding::Payload, LIMITS).expect("bounded shard ledger")
}

pub(super) fn open(bytes: &[u8]) -> Option<Ledger> {
    let ledger: Option<Ledger> =
        ic_wire::decode(bytes, ic_wire::Encoding::Payload, LIMITS).expect("shard ledger");
    if let Some(l) = &ledger {
        assert!(valid_limits(&l.approval));
        let mut counts = [0_u16; 2];
        for slot in l.claims.values() {
            assert!(slot.index() < usize::from(l.approval.max_shards));
            counts[slot.index()] += 1;
            assert!(counts[slot.index()] <= l.approval.keys_per_shard);
        }
        assert_eq!(counts, l.counts);
        assert!(
            l.generations.iter().flatten().count() as u64 * CREATION_CYCLES
                <= l.approval.creation_budget
        );
        for slot in ChildId::ALL {
            assert!(l.principals[slot.index()].is_none() || counts[slot.index()] > 0);
            assert_eq!(
                counts[slot.index()] > 0,
                l.generations[slot.index()].is_some()
            );
        }
    }
    ledger
}

fn valid_limits(a: &ShardPool) -> bool {
    (1..=ChildId::ALL.len()).contains(&usize::from(a.max_shards))
        && (1..=stored::MAX_KEYS_PER_SHARD).contains(&a.keys_per_shard)
        && !a.pool.is_empty()
        && a.pool.len() <= 64
}

pub(crate) fn occupied() -> bool {
    STATE.with_borrow(|s| s.as_ref().is_some_and(|s| s.ledger.is_some()))
}

/// Ingress guard before accepting an account key.
/// # Errors
/// Refuses any principal other than the approved hub.
pub fn hub() -> Result<(), &'static str> {
    STATE.with_borrow(|s| {
        s.as_ref()
            .and_then(|s| s.ledger.as_ref())
            .filter(|l| l.approval.hub == api::caller().to_text())
            .map(|_| ())
            .ok_or("configured shard hub required")
    })
}

/// Approve the exact hub, retained artifact and lifetime ceilings once.
/// # Errors
/// An occupied owner or invalid approval cannot reserve a pool.
/// # Panics
/// Requires controller authority and a successfully opened Root.
pub fn configure(a: ShardPool) -> Result<(), Refusal> {
    assert!(api::is_controller(&api::caller()));
    if occupied() {
        return STATE.with_borrow(|s| {
            if s.as_ref().unwrap().ledger.as_ref().unwrap().approval == a {
                Ok(())
            } else {
                Err(Refusal::Occupied)
            }
        });
    }
    #[cfg(feature = "backup")]
    if crate::backup::occupied() {
        return Err(Refusal::Occupied);
    }
    if ChildId::ALL
        .into_iter()
        .any(|s| snapshot(s).phase != Phase::Empty)
        || super::store_maintenance::observe().is_some()
    {
        return Err(Refusal::Occupied);
    }
    let hub = Principal::from_text(&a.hub).map_err(|_| Refusal::Binding)?;
    if !valid_limits(&a)
        || a.root != api::canister_self().to_text()
        || hub.to_text() != a.hub
        || hub.as_slice().last() != Some(&1)
        || hub == api::canister_self()
        || a.hub == a.reference.store
    {
        return Err(Refusal::Binding);
    }
    if a.expires_at_ns <= api::time() {
        return Err(Refusal::Time);
    }
    delivery::admit(&a.reference, &a.reference.manifest.hash)?;
    STATE.with_borrow_mut(|s| {
        let state = s.as_mut().unwrap();
        state.ledger = Some(Ledger {
            approval: a,
            claims: BTreeMap::new(),
            counts: [0; 2],
            generations: [None; 2],
            principals: [None, None],
        });
        state.pool.set(encode(&state.ledger));
    });
    Ok(())
}

fn resolve(slot: ChildId, generation: [u8; 32]) -> Placement {
    let allocation = Allocation {
        slot: u32::try_from(slot.index()).unwrap(),
        generation,
    };
    STATE.with_borrow(|s| {
        let state = s.as_ref().unwrap();
        let principal = state.ledger.as_ref().unwrap().principals[slot.index()].as_deref();
        let generation = state.artifact.0[&slot]
            .sources()
            .map(|s| s.creation.operation);
        placement::resolve(allocation, principal, generation, &state.journal.0[&slot].0)
    })
}

/// Record the first verified worker in the same callback that commits readiness.
pub(super) fn bind_ready(slot: ChildId, pid: &str) {
    STATE.with_borrow_mut(|s| {
        let state = s.as_mut().unwrap();
        let Some(ledger) = state.ledger.as_mut() else {
            return;
        };
        let original = &mut ledger.principals[slot.index()];
        if let Some(original) = original {
            assert_eq!(original, pid, "shard identity changed");
        } else {
            *original = Some(pid.to_owned());
            state.pool.set(encode(&state.ledger));
        }
    });
}

/// Observe an existing claim, including pending claims, without issuing effects.
/// # Panics
/// Requires the configured hub.
#[must_use]
pub fn lookup(key: &Key) -> Placement {
    hub().expect("hub required");
    STATE.with_borrow(|s| {
        let l = s.as_ref().unwrap().ledger.as_ref().unwrap();
        l.claims.get(key).map_or(Placement::Unknown, |slot| {
            resolve(*slot, l.generations[slot.index()].unwrap())
        })
    })
}

/// Commit a capacity claim and, when necessary, the original Store-backed creation.
/// # Panics
/// Requires the approved hub. All durable writes share one IC message commit.
#[must_use]
pub fn assign(key: Key) -> Placement {
    let retained = lookup(&key);
    if retained != Placement::Unknown {
        return retained;
    }
    let (approval, counts, generations) = STATE.with_borrow(|s| {
        let ledger = s.as_ref().unwrap().ledger.as_ref().unwrap();
        (ledger.approval.clone(), ledger.counts, ledger.generations)
    });
    if counts.iter().sum::<u16>() == u16::from(approval.max_shards) * approval.keys_per_shard {
        return Placement::Full;
    }
    let mut ready = None;
    let mut free = None;
    let mut blocked = false;
    for &slot in &ChildId::ALL[..usize::from(approval.max_shards)] {
        if let Some(generation) = generations[slot.index()] {
            if counts[slot.index()] < approval.keys_per_shard {
                match resolve(slot, generation) {
                    Placement::Assigned(_, pid) => select(
                        &mut ready,
                        score(&[&key, Principal::from_text(pid).unwrap().as_slice()]),
                        slot,
                    ),
                    _ => blocked = true,
                }
            } else if !matches!(resolve(slot, generation), Placement::Assigned(..)) {
                blocked = true;
            }
        } else {
            select(
                &mut free,
                score(&[
                    approval.pool.as_bytes(),
                    &[0xff],
                    &key,
                    &u32::try_from(slot.index()).unwrap().to_le_bytes(),
                ]),
                slot,
            );
        }
    }
    let (slot, generation) = if let Some((_, slot)) = ready {
        (slot, generations[slot.index()].unwrap())
    } else {
        if blocked {
            return Placement::Busy;
        }
        let Some((_, slot)) = free else {
            return Placement::Full;
        };
        let issued = generations.iter().flatten().count() as u64;
        let now = api::time();
        if (issued + 1) * CREATION_CYCLES > approval.creation_budget
            || now >= approval.expires_at_ns
        {
            return Placement::GrowthRefused;
        }
        let input = stored::Stage {
            approval: Approval {
                manager: api::canister_self().to_text(),
                module_hash: approval.reference.manifest.hash,
                creation_cycles: CREATION_CYCLES,
                execute_at_ns: now,
                expires_at_ns: approval
                    .expires_at_ns
                    .min(now.saturating_add(600_000_000_000)),
            },
            reference: approval.reference,
        };
        assert_eq!(snapshot(slot).phase, Phase::Empty);
        let admitted = if approval.arguments.as_slice().is_empty() {
            let creation = delivery::retained(slot, stored::STAGE, &input);
            super::admit(
                slot,
                input.approval,
                Artifact::Stored(stored::Sources {
                    creation,
                    upgrades: vec![],
                }),
            )
        } else {
            super::application::admit_creation(
                slot,
                canic_management_protocol::application::Stage {
                    source: input,
                    arguments: approval.arguments,
                },
            )
        };
        if admitted.is_err() {
            return Placement::GrowthRefused;
        }
        (
            slot,
            delivery::source(slot, false)
                .expect("admitted shard source")
                .operation,
        )
    };
    STATE.with_borrow_mut(|s| {
        let state = s.as_mut().unwrap();
        let ledger = state.ledger.as_mut().unwrap();
        assert!(ledger.claims.insert(key, slot).is_none());
        ledger.counts[slot.index()] += 1;
        ledger.generations[slot.index()] = Some(generation);
        state.pool.set(encode(&state.ledger));
    });
    resolve(slot, generation)
}
