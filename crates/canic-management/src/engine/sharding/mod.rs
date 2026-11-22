//! Independent hub grants and shared-shard placement over Store-backed child effects.
mod capacity;
mod drain;
mod persistence;
pub mod standby;
mod workers;
pub use capacity::{configure_capacity, increase_pool};
pub use drain::{begin_shard_drain, inspect_shard, release_shard};
pub(super) use persistence::{BYTES, open};
pub use workers::{configure_workers, worker, worker_hub};

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

const SLOT_COUNT: usize = ChildId::SLOT_COUNT;

#[derive(Serialize, Deserialize)]
struct Ledger {
    namespace: u8,
    approval: ShardPool,
    increases: Vec<u8>,
    counts: [u32; SLOT_COUNT],
    generations: [Option<[u8; 32]>; SLOT_COUNT],
    principals: [Option<String>; SLOT_COUNT],
}

#[derive(Default, Serialize, Deserialize)]
pub(super) struct Pools {
    returns: Vec<standby::Returned>,
    pub(super) retirements: BTreeMap<ChildId, standby::Retirement>,
    reserve_policy: Option<stored::StandbyPolicy>,
    permanent: [bool; ChildId::ALL.len()],
    draining: BTreeMap<ChildId, [u8; 32]>,
    shards: BTreeMap<String, Ledger>,
    capacity: Option<stored::SubnetCapacity>,
    instance_increases: Vec<u8>,
    instances: Option<stored::InstancePool>,
    projects: Vec<ProjectClaim>,
    workers: BTreeMap<String, workers::Workers>,
    standby: standby::Inventory,
}

#[derive(Serialize, Deserialize)]
struct ProjectClaim {
    key: Key,
    generation: [u8; 32],
    principal: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Acquisition {
    Reserve { index: usize, principal: Principal },
    Create,
}

impl Acquisition {
    pub(super) fn attachment(self) -> u64 {
        match self {
            Self::Reserve { .. } => 0,
            Self::Create => CREATION_CYCLES,
        }
    }

    fn approve(self, remaining_budget: u64) -> Result<super::Origin, Placement> {
        if self.attachment() > remaining_budget {
            Err(Placement::GrowthRefused)
        } else {
            Ok(super::Origin::Placement(self))
        }
    }
}

impl Pools {
    pub(super) fn shard_pool_name(&self, slot: ChildId) -> Option<&str> {
        self.shard_at(slot)
            .map(|ledger| ledger.approval.pool.as_str())
    }

    fn shard_at(&self, slot: ChildId) -> Option<&Ledger> {
        self.shards
            .values()
            .find(|ledger| ledger.generations[slot.index()].is_some())
    }

    pub(super) fn acquisition(&self) -> Result<Acquisition, Placement> {
        if let Some((index, (_, _, principal))) = self
            .standby
            .iter()
            .enumerate()
            .filter_map(|(index, (_, asset))| Some((index, standby::available(asset)?)))
            .min_by_key(|(_, key)| *key)
        {
            Ok(Acquisition::Reserve { index, principal })
        } else if self.full() {
            Err(Placement::Full)
        } else {
            Ok(Acquisition::Create)
        }
    }

    fn pool_limit(&self) -> u8 {
        self.capacity.as_ref().map_or(2, |c| c.max_children)
    }

    fn shard_slots(&self, grant: &ShardPool) -> Vec<ChildId> {
        let ledger = &self.shards[&grant.pool];
        let limit = usize::from(ledger.increases.last().copied().unwrap_or(grant.max_shards));
        let mut slots: Vec<_> = ChildId::SHARDS
            .into_iter()
            .filter(|slot| {
                ledger
                    .generations
                    .get(slot.index())
                    .is_some_and(Option::is_some)
            })
            .collect();
        let free = limit.saturating_sub(slots.len());
        slots.extend(
            ChildId::SHARDS
                .into_iter()
                .filter(|slot| self.shard_at(*slot).is_none())
                .take(free),
        );
        slots
    }

    fn instance_slots(&self, grant: &stored::InstancePool) -> &'static [ChildId] {
        let count = self
            .instance_increases
            .last()
            .copied()
            .unwrap_or(grant.max_instances);
        &ChildId::INSTANCES[..usize::from(count)]
    }

    fn occupied_slots(&self) -> usize {
        // A returned allocation remains in its original claim history.
        self.used() + self.held() - self.returns.len()
    }

    fn used(&self) -> usize {
        self.permanent.iter().filter(|&&used| used).count()
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
            .filter(|(_, asset)| {
                matches!(
                    asset,
                    canic::sharding::standby::Asset::Transfer(
                        _,
                        _,
                        _,
                        canic::sharding::standby::TransferPhase::Reserved
                            | canic::sharding::standby::TransferPhase::ControllersIssued
                    ) | canic::sharding::standby::Asset::Empty(_)
                        | canic::sharding::standby::Asset::Available(..)
                        | canic::sharding::standby::Asset::AwaitingRelease(_)
                        | canic::sharding::standby::Asset::AwaitingHandoff(_)
                        | canic::sharding::standby::Asset::Quarantined(..)
                        | canic::sharding::standby::Asset::Issued
                )
            })
            .count()
    }

    pub(super) fn standby_claim(&self, slot: ChildId) -> Option<Principal> {
        use canic::sharding::standby::{Asset, Source};
        for (origin, asset) in &self.standby {
            if let Asset::Claimed(id, allocation)
            | Asset::Released(id, allocation)
            | Asset::Reset(id, allocation, ..) = asset
                && allocation.slot == slot.index() as u32
            {
                let donated = matches!(origin, Source::Worker(t) | Source::Shard(t) if t.allocation == *allocation);
                return (!donated).then_some(*id);
            }
        }
        // The original shard journal survives return. Its paid creation stays charged
        // even after another allocation claims the principal.
        let retirement = self.retirements.get(&slot)?;
        if !self.archived(slot) {
            return None;
        }
        let target = retirement.target();
        let donated = self
            .standby
            .iter()
            .any(|(source, _)| matches!(source, Source::Shard(original) if original == &target));
        (!donated).then_some(target.shard)
    }

    pub(super) fn archived(&self, slot: ChildId) -> bool {
        self.retirements
            .get(&slot)
            .is_some_and(|r| r.phase == stored::RetirementPhase::Returned)
    }

    pub(super) fn withdrawn(&self, slot: ChildId) -> bool {
        self.retirements.contains_key(&slot)
            || self.standby.iter().any(|(_, asset)| {
                standby::released_binding(asset).is_some_and(|(_, a)| a.slot == slot.index() as u32)
            })
    }

    fn creation_debit(&self, slots: impl Iterator<Item = ChildId>) -> u64 {
        slots
            .filter(|slot| self.standby_claim(*slot).is_none())
            .count() as u64
            * CREATION_CYCLES
    }

    fn admit_growth(
        &self,
        slots: impl Iterator<Item = ChildId>,
        budget: u64,
    ) -> Result<super::Origin, Placement> {
        self.acquisition()?
            .approve(budget.saturating_sub(self.creation_debit(slots)))
    }

    pub(super) fn full(&self) -> bool {
        self.capacity
            .as_ref()
            .is_some_and(|c| self.occupied_slots() >= usize::from(c.max_children))
    }
}

fn valid_limits(a: &ShardPool) -> bool {
    (1..=usize::from(stored::MAX_SUBNET_CHILDREN)).contains(&usize::from(a.max_shards))
        && (1..=stored::MAX_KEYS_PER_SHARD).contains(&a.keys_per_shard)
        && !a.pool.is_empty()
        && a.pool.len() <= 64
}

// The approved parent and Root must agree through every recognized envelope.
fn check_shard_topology(grant: &ShardPool) -> Result<(), Refusal> {
    if let Some(install) = canic::topology::Install::approved(grant.arguments.as_slice())
        .map_err(|_| Refusal::Binding)?
        && (install.root.to_text() != grant.root || install.parent.to_text() != grant.hub)
    {
        return Err(Refusal::Binding);
    }
    Ok(())
}

pub(crate) fn occupied() -> bool {
    STATE.with_borrow(|s| {
        s.as_ref().is_some_and(|s| {
            s.pools.capacity.is_some() || !s.pools.shards.is_empty() || s.pools.instances.is_some()
        })
    })
}

/// Ordinary staging is available alongside placement only under an explicit backstop.
pub(crate) fn has_capacity() -> bool {
    STATE.with_borrow(|s| s.as_ref().is_some_and(|s| s.pools.capacity.is_some()))
}

pub(super) fn retain_child(
    slot: ChildId,
    generation: [u8; 32],
    origin: super::Origin,
    selected: Acquisition,
) {
    STATE.with_borrow_mut(|s| {
        let state = s.as_mut().expect("open Root");
        if state.pools.capacity.is_some() {
            if origin == super::Origin::Permanent {
                state.pools.permanent[slot.service_index().expect("permanent slot")] = true;
            }
            standby::reserve(&mut state.pools, slot, generation, selected);
            state.pool.set(state.pools.encode());
        }
    });
    standby::policy::wake();
}

pub(super) fn worker_generation(slot: ChildId) -> Option<[u8; 32]> {
    STATE.with_borrow(|s| {
        s.as_ref()?
            .pools
            .worker_at(slot)
            .map(|(_, claim)| claim.generation)
    })
}

/// Ingress guard before accepting an account key.
/// # Errors
/// Refuses any principal other than the approved hub.
pub fn hub() -> Result<(), &'static str> {
    STATE.with_borrow(|s| {
        s.as_ref()
            .filter(|s| {
                s.pools
                    .shards
                    .values()
                    .any(|l| l.approval.hub == api::caller().to_text())
            })
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
    if let Some(result) = STATE.with_borrow(|s| {
        let pools = &s.as_ref().expect("open Root").pools;
        if a.max_shards > pools.pool_limit()
            || pools.workers.values().any(|w| !compatible(&a, &w.grant))
        {
            return Some(Err(Refusal::Binding));
        }
        if let Some(l) = pools.shards.get(&a.pool) {
            return Some(if l.approval == a {
                Ok(())
            } else {
                Err(Refusal::Occupied)
            });
        }
        if (!pools.shards.is_empty() && pools.capacity.is_none())
            || pools.shards.len() >= usize::from(pools.pool_limit())
            || pools
                .shards
                .values()
                .any(|other| other.approval.reference.store != a.reference.store)
        {
            return Some(Err(Refusal::Binding));
        }
        pools
            .instances
            .as_ref()
            .filter(|i| a.hub == i.hub || !compatible(&a, i))
            .map(|_| Err(Refusal::Binding))
    }) {
        return result;
    }
    admit_pool_change(false)?;
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
    check_shard_topology(&a)?;
    if a.expires_at_ns <= api::time() {
        return Err(Refusal::Time);
    }
    delivery::admit(&a.reference, &a.reference.manifest.hash)?;
    STATE.with_borrow_mut(|s| {
        let state = s.as_mut().unwrap();
        let namespace = u8::try_from(state.pools.shards.len()).expect("bounded shard grants");
        state.pools.shards.insert(
            a.pool.clone(),
            Ledger {
                namespace,
                increases: Vec::new(),
                approval: a,
                counts: [0; SLOT_COUNT],
                generations: [None; SLOT_COUNT],
                principals: Default::default(),
            },
        );
        state.pool.set(state.pools.encode());
    });
    Ok(())
}

// An explicit backstop accounts for existing ordinary reservations. Historical
// grants without that backstop retain their placement-only layout.
fn admit_pool_change(admitting_capacity: bool) -> Result<(), Refusal> {
    #[cfg(feature = "backup")]
    if crate::backup::occupied() {
        return Err(Refusal::Occupied);
    }
    if super::store_maintenance::observe().is_some()
        || (!admitting_capacity
            && !occupied()
            && ChildId::ALL
                .into_iter()
                .any(|s| snapshot(s).phase != Phase::Empty))
    {
        return Err(Refusal::Occupied);
    }
    Ok(())
}

fn compatible(shared: &ShardPool, instances: &stored::InstancePool) -> bool {
    shared.root == instances.root
        && shared.reference.store == instances.reference.store
        && shared.pool != instances.component
}

fn validate_instance(a: &stored::InstancePool) -> Result<(), Refusal> {
    let install = canic::topology::Install::approved(a.arguments.as_slice())
        .map_err(|_| Refusal::Binding)?
        .ok_or(Refusal::Binding)?;
    validate_pool_install(a, &install)
}

fn validate_pool_install(
    a: &stored::InstancePool,
    install: &canic::topology::Install,
) -> Result<(), Refusal> {
    let hub = Principal::from_text(&a.hub).map_err(|_| Refusal::Binding)?;
    if a.root != api::canister_self().to_text()
        || hub.to_text() != a.hub
        || hub.as_slice().last() != Some(&1)
        || a.hub == a.root
        || a.hub == a.reference.store
        || a.component.is_empty()
        || a.component.len() > 64
        || !(1..=stored::MAX_INSTANCES).contains(&a.max_instances)
    {
        return Err(Refusal::Binding);
    }
    if install.root != api::canister_self() || install.parent != hub {
        return Err(Refusal::Binding);
    }
    delivery::admit(&a.reference, &a.reference.manifest.hash)
}

/// Retain the controller's exact dedicated-instance grant without issuing effects.
/// # Errors
/// Refuses conflicting authority, invalid bindings, or a new expired grant.
/// # Panics
/// Requires the Root controller and an opened placement ledger.
pub fn configure_instances(a: stored::InstancePool) -> Result<(), Refusal> {
    assert!(api::is_controller(&api::caller()));
    let prior = STATE.with_borrow(|s| {
        let pools = &s.as_ref().expect("open Root").pools;
        if a.max_instances > pools.pool_limit()
            || pools
                .workers
                .values()
                .any(|w| !workers::separate(&a, &w.grant))
        {
            return Some(Err(Refusal::Binding));
        }
        if let Some(previous) = &pools.instances {
            return Some(if previous == &a {
                Ok(())
            } else {
                Err(Refusal::Occupied)
            });
        }
        pools
            .shards
            .values()
            .find(|l| l.approval.hub == a.hub || !compatible(&l.approval, &a))
            .map(|_| Err(Refusal::Binding))
    });
    if let Some(result) = prior {
        return result;
    }
    admit_pool_change(false)?;
    validate_instance(&a)?;
    if a.expires_at_ns <= api::time() {
        return Err(Refusal::Time);
    }
    STATE.with_borrow_mut(|s| {
        let state = s.as_mut().expect("open Root");
        state.pools.instances = Some(a);
        state.pool.set(state.pools.encode());
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
        let principal = state.pools.shard_at(slot).unwrap().principals[slot.index()].as_deref();
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
        let (generation, original) =
            if let Some(index) = ChildId::INSTANCES.iter().position(|id| *id == slot) {
                let pools = &mut state.pools;
                if let Some(claim) = pools.projects.get_mut(index) {
                    (claim.generation, &mut claim.principal)
                } else {
                    let claim = pools
                        .workers
                        .values_mut()
                        .flat_map(|w| &mut w.claims)
                        .find(|claim| claim.slot == slot && !claim.returned)
                        .expect("claimed worker");
                    (claim.generation, &mut claim.principal)
                }
            } else {
                let Some(ledger) = state
                    .pools
                    .shards
                    .values_mut()
                    .find(|l| l.generations.get(slot.index()).is_some_and(Option::is_some))
                else {
                    return;
                };
                let Some(generation) = ledger.generations[slot.index()] else {
                    return;
                };
                (generation, &mut ledger.principals[slot.index()])
            };
        let source = state.artifact.0[&slot]
            .sources()
            .expect("claimed child source");
        assert_eq!(
            source.creation.operation, generation,
            "child generation changed"
        );
        if let Some(original) = original {
            assert_eq!(original, pid, "child identity changed");
        } else {
            *original = Some(pid.to_owned());
            state.pool.set(state.pools.encode());
        }
    });
}

/// Observe an existing claim, including pending claims, without issuing effects.
/// # Panics
/// Requires the configured hub.
#[must_use]
pub fn lookup(pool: &str, key: &Key) -> Placement {
    hub().expect("hub required");
    STATE.with_borrow(|s| {
        let state = s.as_ref().unwrap();
        let l = state
            .pools
            .shards
            .get(pool)
            .filter(|l| l.approval.hub == api::caller().to_text())
            .expect("calling hub must own requested shard pool");
        state
            .claims
            .get(&(l.namespace, *key))
            .map_or(Placement::Unknown, |id| {
                let slot = ChildId::placement(usize::from(id)).expect("retained shard slot");
                resolve(
                    slot,
                    l.generations[slot.index()].expect("retained shard generation"),
                )
            })
    })
}

/// Commit a capacity claim and, when necessary, the original Store-backed creation.
/// # Panics
/// Requires the approved hub. All durable writes share one IC message commit.
#[must_use]
pub fn assign(pool: &str, key: Key) -> Placement {
    let retained = lookup(pool, &key);
    if retained != Placement::Unknown {
        return retained;
    }
    let (approval, slots, counts, generations) = STATE.with_borrow(|s| {
        let pools = &s.as_ref().unwrap().pools;
        let ledger = &pools.shards[pool];
        (
            ledger.approval.clone(),
            pools
                .shard_slots(&ledger.approval)
                .into_iter()
                .filter(|slot| !pools.draining.contains_key(slot))
                .collect::<Vec<_>>(),
            ledger.counts,
            ledger.generations,
        )
    });
    if slots
        .iter()
        .all(|s| counts[s.index()] == approval.keys_per_shard)
    {
        return Placement::Full;
    }
    let mut ready = None;
    let mut free = None;
    let mut blocked = false;
    for slot in slots {
        if let Some(generation) = generations[slot.index()] {
            if counts[slot.index()] < approval.keys_per_shard {
                match resolve(slot, generation) {
                    Placement::Assigned(_, pid) => {
                        let subject = Principal::from_text(pid).expect("retained shard principal");
                        select(&mut ready, score(&[&key, subject.as_slice()]), slot);
                    }
                    _ => blocked = true,
                }
            } else if !matches!(resolve(slot, generation), Placement::Assigned(..)) {
                blocked = true;
            }
        } else {
            let candidate = match slot {
                id if id == ChildId::SHARDS[0] => 0,
                id if id == ChildId::SHARDS[1] => 1,
                id => id.index() as u32,
            };
            select(
                &mut free,
                score(&[
                    approval.pool.as_bytes(),
                    &[0xff],
                    &key,
                    &candidate.to_le_bytes(),
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
        let now = api::time();
        let origin = match STATE.with_borrow(|s| {
            let pools = &s.as_ref().unwrap().pools;
            pools.admit_growth(
                pools
                    .shard_slots(&approval)
                    .into_iter()
                    .filter(|slot| generations[slot.index()].is_some()),
                approval.creation_budget,
            )
        }) {
            Ok(origin) => origin,
            Err(outcome) => return outcome,
        };
        if now >= approval.expires_at_ns {
            return Placement::GrowthRefused;
        }
        let Some(generation) = create(
            origin,
            slot,
            approval.reference,
            approval.arguments,
            now,
            approval.expires_at_ns,
        ) else {
            return Placement::GrowthRefused;
        };
        (slot, generation)
    };
    STATE.with_borrow_mut(|s| {
        let state = s.as_mut().unwrap();
        let ledger = state.pools.shards.get_mut(pool).unwrap();
        assert!(
            state
                .claims
                .insert((ledger.namespace, key), u8::try_from(slot.index()).unwrap())
                .is_none()
        );
        ledger.counts[slot.index()] += 1;
        ledger.generations[slot.index()] = Some(generation);
        state.pool.set(state.pools.encode());
    });
    resolve(slot, generation)
}

fn create(
    origin: super::Origin,
    slot: ChildId,
    reference: stored::Reference,
    arguments: canic_management_protocol::application::Arguments,
    now: u64,
    deadline: u64,
) -> Option<[u8; 32]> {
    let input = stored::Stage {
        approval: Approval {
            manager: api::canister_self().to_text(),
            module_hash: reference.manifest.hash,
            creation_cycles: CREATION_CYCLES,
            execute_at_ns: now,
            expires_at_ns: deadline.min(now.saturating_add(600_000_000_000)),
        },
        reference,
    };
    assert!(STATE.with_borrow(|s| {
        s.as_ref()
            .unwrap()
            .journal
            .0
            .get(&slot)
            .is_none_or(|j| j.0.phase == Phase::Empty)
    }));
    let admitted = if arguments.as_slice().is_empty() {
        let creation = delivery::retained(slot, stored::STAGE, &input);
        super::admit(
            origin,
            slot,
            input.approval,
            Artifact::Stored(stored::Sources {
                creation,
                upgrades: vec![],
            }),
        )
    } else {
        super::application::admit_creation(
            origin,
            slot,
            canic_management_protocol::application::Stage {
                source: input,
                arguments,
            },
        )
    };
    if admitted.is_err() {
        return None;
    }
    let generation = delivery::source(slot, false)
        .expect("admitted creation")
        .operation;
    STATE.with_borrow(|s| {
        assert!(
            !s.as_ref().unwrap().worker_history.contains_key(&generation),
            "allocation generation already returned"
        )
    });
    Some(generation)
}

pub(super) fn validate_claims(state: &super::State) {
    #[cfg(feature = "role-issuer")]
    super::role_issuer::validate_closures(state, |p, g| {
        drain::closed_assignment(&state.pools, p, g)
    });
    let claims = &state.pools.projects;
    let pools = &state.pools;
    for (_, asset) in &pools.standby {
        if let canic::sharding::standby::Asset::Claimed(id, allocation) = asset {
            let slot = ChildId::placement(allocation.slot as usize).expect("claimed slot");
            let journal = &state.journal.0[&slot].0;
            let approval = journal.approval.as_ref().expect("claimed admission");
            assert_eq!(
                state.artifact.0[&slot]
                    .sources()
                    .map_or(approval.module_hash, |s| s.creation.operation),
                allocation.generation
            );
            assert!(journal.worker.as_ref().is_none_or(|p| p == &id.to_text()));
            assert_eq!(journal.attempts[0], 0);
        }
    }
    let mut archived_workers = 0u64;
    for claim in pools
        .workers
        .values()
        .flat_map(|pool| &pool.claims)
        .filter(|claim| claim.returned)
    {
        assert!(
            state.worker_history.contains_key(&claim.generation),
            "missing returned worker custody"
        );
        archived_workers += 1;
    }
    assert_eq!(archived_workers, state.worker_history.len());
    if pools.capacity.is_some() {
        for (slot, reserved) in ChildId::ALL.into_iter().zip(pools.permanent) {
            assert_eq!(
                reserved,
                state.journal.0[&slot].0.approval.is_some() && pools.shard_at(slot).is_none(),
                "permanent service reservation"
            );
        }
        assert_eq!(
            pools.used()
                - usize::try_from(state.worker_history.len()).expect("bounded worker history"),
            state
                .journal
                .0
                .values()
                .filter(|entry| entry.0.approval.is_some())
                .count(),
            "subnet reservation custody"
        );
    } else {
        assert_eq!(
            state
                .journal
                .0
                .keys()
                .any(|s| ChildId::INSTANCES.contains(s)),
            !claims.is_empty()
        );
    }
    for slot in ChildId::SHARDS {
        if let Some((ledger, generation)) = pools.shard_at(slot).and_then(|ledger| {
            ledger.generations[slot.index()].map(|generation| (ledger, generation))
        }) {
            validate_allocation(
                state,
                slot,
                generation,
                ledger.principals[slot.index()].as_deref(),
                &ledger.approval.reference,
                &ledger.approval.arguments,
            );
        } else {
            assert!(
                state
                    .artifact
                    .0
                    .get(&slot)
                    .and_then(Artifact::sources)
                    .is_none(),
                "unclaimed shard source"
            );
        }
    }
    for (index, slot) in ChildId::INSTANCES.into_iter().enumerate() {
        let Some(journal) = state.journal.0.get(&slot) else {
            assert!(claims.get(index).is_none() && pools.worker_at(slot).is_none());
            continue;
        };
        let source = state.artifact.0[&slot].sources();
        let (grant, generation, principal) = if let Some(claim) = claims.get(index) {
            assert!(claims[..index].iter().all(|other| other.key != claim.key));
            (
                pools.instances.as_ref().expect("project grant"),
                claim.generation,
                claim.principal.as_deref(),
            )
        } else if let Some((workers, claim)) = pools.worker_at(slot) {
            (&workers.grant, claim.generation, claim.principal.as_deref())
        } else {
            assert!(source.is_none() && journal.0.phase == Phase::Empty);
            continue;
        };
        validate_allocation(
            state,
            slot,
            generation,
            principal,
            &grant.reference,
            &grant.arguments,
        );
    }
}

// A role can only be derived from the exact bytes Root installed. Check both
// pool families against the same retained installation and readiness evidence.
fn validate_allocation(
    state: &super::State,
    slot: ChildId,
    generation: [u8; 32],
    principal: Option<&str>,
    reference: &stored::Reference,
    arguments: &canic_management_protocol::application::Arguments,
) {
    let (sources, installed_args) = match &state.artifact.0[&slot] {
        Artifact::Stored(sources) => (sources, &[][..]),
        Artifact::Application(sources, app) => (sources, app.creation.as_slice()),
        Artifact::Inline(_) => panic!("placement requires a Store source"),
    };
    assert_eq!(
        installed_args,
        arguments.as_slice(),
        "placement install arguments"
    );
    assert_eq!(&sources.creation.reference, reference, "placement release");
    assert_eq!(
        sources.creation.operation, generation,
        "placement generation"
    );
    let journal = &state.journal.0[&slot].0;
    assert!(journal.approval.is_some());
    assert_eq!(
        principal.is_some(),
        journal.phase == Phase::Ready,
        "placement readiness"
    );
    if principal.is_some() {
        assert_eq!(principal, journal.worker.as_deref(), "placement principal");
    }
}

/// Require the actual caller bound to the dedicated pool.
/// # Errors
/// Refuses other callers, including the shared hub.
pub fn project_hub() -> Result<(), &'static str> {
    STATE.with_borrow(|s| {
        s.as_ref()
            .and_then(|s| s.pools.instances.as_ref())
            .filter(|a| a.hub == api::caller().to_text())
            .map(|_| ())
            .ok_or("configured project hub required")
    })
}

/// Observe the original project creation without issuing an effect.
/// # Panics
/// Requires the approved project hub.
#[must_use]
pub fn lookup_instance(key: &Key) -> Placement {
    project_hub().expect("project hub required");
    STATE.with_borrow(|s| {
        let state = s.as_ref().unwrap();
        let Some((i, claim)) = state
            .pools
            .projects
            .iter()
            .enumerate()
            .find(|(_, c)| &c.key == key)
        else {
            return Placement::Unknown;
        };
        let slot = ChildId::INSTANCES[i];
        placement::resolve(
            Allocation {
                slot: slot.index() as u32,
                generation: claim.generation,
            },
            claim.principal.as_deref(),
            state.artifact.0[&slot]
                .sources()
                .map(|s| s.creation.operation),
            &state.journal.0[&slot].0,
        )
    })
}

/// Reserve one dedicated child and retain its original creation in the same message.
/// # Panics
/// Requires the approved project hub and valid retained custody.
#[must_use]
pub fn assign_instance(key: Key) -> Placement {
    let previous = lookup_instance(&key);
    if previous != Placement::Unknown {
        return previous;
    }
    let selection = STATE.with_borrow(|s| {
        let pools = &s.as_ref().unwrap().pools;
        let grant = pools.instances.as_ref().unwrap();
        let count = pools.projects.len();
        let slots = pools.instance_slots(grant);
        let slot = *slots
            .get(count)
            .filter(|slot| pools.worker_at(**slot).is_none())
            .ok_or(Placement::Full)?;
        let origin = pools.admit_growth(slots[..count].iter().copied(), grant.creation_budget)?;
        Ok((grant.clone(), slot, origin))
    });
    let (grant, slot, origin) = match selection {
        Ok(selected) => selected,
        Err(outcome) => return outcome,
    };
    let now = api::time();
    if now >= grant.expires_at_ns {
        return Placement::GrowthRefused;
    }
    let Some(generation) = create(
        origin,
        slot,
        grant.reference,
        grant.arguments,
        now,
        grant.expires_at_ns,
    ) else {
        return Placement::GrowthRefused;
    };
    STATE.with_borrow_mut(|s| {
        let state = s.as_mut().unwrap();
        state.pools.projects.push(ProjectClaim {
            key,
            generation,
            principal: None,
        });
        state.pool.set(state.pools.encode());
    });
    lookup_instance(&key)
}

#[cfg(feature = "role-issuer")]
pub(super) struct AssignedRole {
    pub role: String,
    pub generation: [u8; 32],
    /// Live placements and drain state belong only to a shared shard.
    pub shard: Option<(u32, bool)>,
}

#[cfg(feature = "role-issuer")]
pub(super) fn assigned_role(subject: Principal) -> Result<AssignedRole, &'static str> {
    let caller = subject.to_text();
    STATE.with_borrow(|s| {
        let state = s.as_ref().expect("open Root");
        let (&slot, journal) = state
            .journal
            .0
            .iter()
            .find(|(slot, journal)| {
                !state.pools.withdrawn(**slot)
                    && journal.0.worker.as_deref() == Some(caller.as_str())
            })
            .ok_or("unassigned caller")?;
        if !journal.ready_for_role() {
            return Err("assignment unavailable");
        }
        let Artifact::Application(sources, app) = &state.artifact.0[&slot] else {
            return Err("unassigned caller");
        };
        let generation = sources.creation.operation;
        if ChildId::ALL.contains(&slot)
            && let Some(named) = super::directory::named_service(slot)
        {
            return Ok(AssignedRole {
                role: named.role,
                generation,
                shard: None,
            });
        }
        let (original, principal, args, reference, hub, shard) =
            if let Some(index) = ChildId::INSTANCES.iter().position(|id| *id == slot) {
                let claim = state.pools.projects.get(index).ok_or("unassigned caller")?;
                let grant = state
                    .pools
                    .instances
                    .as_ref()
                    .ok_or("missing instance grant")?;
                (
                    Some(claim.generation),
                    &claim.principal,
                    &grant.arguments,
                    &grant.reference,
                    &grant.hub,
                    None,
                )
            } else {
                let ledger = state.pools.shard_at(slot).ok_or("unassigned caller")?;
                let index = slot.index();
                (
                    ledger.generations[index],
                    &ledger.principals[index],
                    &ledger.approval.arguments,
                    &ledger.approval.reference,
                    &ledger.approval.hub,
                    Some((
                        ledger.counts[index],
                        state.pools.draining.contains_key(&slot),
                    )),
                )
            };
        if original != Some(generation)
            || principal.as_deref() != Some(&caller)
            || app.creation != *args
            || sources.creation.reference != *reference
        {
            return Err("assignment changed");
        }
        let install = canic::topology::Install::approved(args.as_slice())?
            .ok_or("missing placement topology")?;
        if install.root != api::canister_self() || install.parent.to_text() != *hub {
            return Err("placement topology mismatch");
        }
        Ok(AssignedRole {
            role: install.role,
            generation,
            shard,
        })
    })
}
