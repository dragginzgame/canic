//! Independent hub grants and shared-shard placement over Store-backed child effects.
mod capacity;
mod drain;
pub mod funding;
pub mod payment;
mod persistence;
mod singleton;
pub use singleton::{configure_singleton, request_singleton};
pub mod standby;
#[cfg(test)]
mod tests;
mod workers;
pub use capacity::{configure_capacity, increase_pool};
pub use drain::{begin_shard_drain, inspect_shard};
pub(super) use persistence::{BYTES, open};
pub use workers::{configure_workers, worker};

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
    ordinals: Vec<ChildId>,
    approval: ShardPool,
    increases: Vec<u8>,
    generations: [Option<[u8; 32]>; SLOT_COUNT],
    principals: [Option<String>; SLOT_COUNT],
}

impl Ledger {
    fn allocated_slots(&self) -> impl Iterator<Item = ChildId> {
        ChildId::SHARDS
            .into_iter()
            .filter(|slot| self.generations[slot.index()].is_some())
    }
}

#[derive(Default, Serialize, Deserialize)]
pub(super) struct Pools {
    evacuated: std::collections::BTreeSet<ChildId>,
    funding: BTreeMap<ChildId, funding::Grant>,
    quotes: BTreeMap<payment::Owner, payment::Reservation>,
    payments: BTreeMap<[u8; 32], payment::Receipt>,
    returns: Vec<standby::Returned>,
    pub(super) retirements: BTreeMap<ChildId, standby::Retirement>,
    reserve_policy: Option<stored::StandbyPolicy>,
    permanent: std::collections::BTreeSet<ChildId>,
    draining: BTreeMap<ChildId, [u8; 32]>,
    shards: BTreeMap<String, Ledger>,
    capacity: Option<stored::SubnetCapacity>,
    instance_increases: Vec<u8>,
    instances: Option<stored::InstancePool>,
    projects: Vec<ProjectClaim>,
    singleton: Option<singleton::Registry>,
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
        self.shards.values().find(|ledger| {
            ledger
                .generations
                .get(slot.index())
                .is_some_and(Option::is_some)
        })
    }

    pub(super) fn acquisition(&self) -> Result<Acquisition, Placement> {
        if let Some((index, (principal, _))) = self
            .standby
            .iter()
            .enumerate()
            .filter(|(index, _)| !self.reserved_for_payment(*index))
            .filter_map(|(index, (_, asset))| Some((index, standby::available(asset)?)))
            .min_by_key(|(_, (id, balance))| (balance.is_none(), *balance, *id))
        {
            Ok(Acquisition::Reserve { index, principal })
        } else if self.full() {
            Err(Placement::Full)
        } else {
            Ok(Acquisition::Create)
        }
    }

    fn reserved_for_payment(&self, index: usize) -> bool {
        self.quotes
            .values()
            .any(|q| q.index == Some(index) && api::time() < q.deadline)
    }

    fn source(
        &self,
        payer: stored::CreationPayer,
        paid: Option<payment::Settlement>,
        budget: u64,
    ) -> Result<super::Origin, Placement> {
        match (payer, paid) {
            (stored::CreationPayer::Root, None) => self.acquisition()?.approve(budget),
            (stored::CreationPayer::Requester, Some(settlement))
                if settlement.source.attachment() <= budget =>
            {
                Ok(super::Origin::Paid(settlement.source, settlement.charge))
            }
            _ => Err(Placement::GrowthRefused),
        }
    }

    fn pool_limit(&self) -> u8 {
        u8::try_from(self.subnet_limit()).unwrap_or(u8::MAX)
    }

    fn subnet_limit(&self) -> u32 {
        self.capacity
            .as_ref()
            .map_or(stored::DEFAULT_SUBNET_CHILD_LIMIT, |c| c.max_children)
    }

    fn shard_slots(&self, ledger: &Ledger) -> Vec<ChildId> {
        let limit = usize::from(
            ledger
                .increases
                .last()
                .copied()
                .unwrap_or(ledger.approval.max_shards),
        );
        let mut slots: Vec<_> = ledger.allocated_slots().collect();
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
        payer: stored::CreationPayer,
        paid: Option<payment::Settlement>,
    ) -> Result<super::Origin, Placement> {
        self.source(
            payer,
            paid,
            budget.saturating_sub(self.creation_debit(slots)),
        )
    }

    pub(super) fn capacity_approved(&self) -> bool {
        self.capacity.is_some()
    }

    pub(super) fn full(&self) -> bool {
        self.occupied_slots() >= self.subnet_limit() as usize
    }
}

fn valid_limits(a: &ShardPool) -> bool {
    (1..=stored::MAX_SHARDS).contains(&a.max_shards)
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

/// Identify the placement mounts needed to recover existing approvals after an upgrade.
/// # Panics
/// Requires an opened Root.
#[must_use]
pub fn retained_mounts() -> [(&'static str, bool); 5] {
    use canic::sharding::standby::{Asset, Source};
    STATE.with_borrow(|state| {
        let pools = &state.as_ref().expect("open Root").pools;
        [
            ("shard-pool", !pools.shards.is_empty()),
            (
                "registry",
                pools.instances.is_some() || pools.singleton.is_some(),
            ),
            ("worker-pool", !pools.workers.is_empty()),
            ("child-funding", !pools.funding.is_empty()),
            (
                "reserve-pool",
                pools.reserve_policy.is_some()
                    || pools.standby.iter().any(|(source, asset)| {
                        matches!(
                            source,
                            Source::Imported(_) | Source::Automatic(_) | Source::Refill(_)
                        ) || matches!(asset, Asset::Transfer(..))
                    }),
            ),
        ]
    })
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
                state.pools.permanent.insert(slot);
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

impl Pools {
    fn caller_pool(&self, pool: &str) -> &Ledger {
        self.shards
            .get(pool)
            .filter(|ledger| ledger.approval.hub == api::caller().to_text())
            .expect("calling hub must own requested shard pool")
    }
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
            || pools.shards.len() >= usize::from(pools.pool_limit().min(stored::MAX_SHARDS))
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
        state.pools.shards.insert(
            a.pool.clone(),
            Ledger {
                ordinals: Vec::new(),
                increases: Vec::new(),
                approval: a,
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
            && super::service_slots()
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

fn resolve(
    state: &super::State,
    ledger: &Ledger,
    slot: ChildId,
    generation: [u8; 32],
) -> Placement {
    let allocation = Allocation {
        slot: u32::try_from(slot.index()).unwrap(),
        generation,
    };
    let principal = ledger.principals[slot.index()].as_deref();
    if state.pools.retirements.contains_key(&slot) {
        return Placement::Unavailable(allocation, principal.map(str::to_owned));
    }
    let generation = state.artifact.0[&slot]
        .sources()
        .map(|s| s.creation.operation);
    placement::resolve(allocation, principal, generation, &state.journal.0[&slot].0)
}

/// Record the first verified worker in the same callback that commits readiness.
pub(super) fn bind_ready(slot: ChildId, pid: &str) {
    STATE.with_borrow_mut(|s| {
        let state = s.as_mut().unwrap();
        let (generation, original) = if let Some(index) =
            ChildId::INSTANCES.iter().position(|id| *id == slot)
        {
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
            let Some(generation) = ledger.generations.get(slot.index()).copied().flatten() else {
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

/// Request the next whole shard, retaining one generation per ordinal.
/// # Panics
/// Refuses calls outside the named hub grant.
#[must_use]
pub fn grow_shard(pool: &str, ordinal: u8) -> (Placement, u32) {
    let placed = allocate_shard(pool, ordinal, None);
    let limit = STATE.with_borrow(|s| {
        s.as_ref().unwrap().pools.shards[pool]
            .approval
            .keys_per_shard
    });
    (placed, limit)
}

fn allocate_shard(pool: &str, ordinal: u8, paid: Option<payment::Settlement>) -> Placement {
    let candidate = STATE.with_borrow(|s| {
        let state = s.as_ref().expect("open Root");
        let pools = &state.pools;
        let ledger = pools.caller_pool(pool);
        if let Some(slot) = ledger.ordinals.get(usize::from(ordinal)) {
            return Err(resolve(
                state,
                ledger,
                *slot,
                ledger.generations[slot.index()].expect("retained allocation"),
            ));
        }
        if usize::from(ordinal) != ledger.ordinals.len() {
            return Err(Placement::GrowthRefused);
        }
        let mut key = [0; 32];
        key[0] = ordinal;
        let mut free = None;
        for slot in pools.shard_slots(ledger) {
            if let Some(generation) = ledger.generations[slot.index()] {
                if !pools.draining.contains_key(&slot)
                    && !matches!(
                        resolve(state, ledger, slot, generation),
                        Placement::Assigned(..)
                    )
                {
                    return Err(Placement::Busy);
                }
            } else {
                let candidate = if slot == ChildId::SHARDS[0] {
                    0
                } else if slot == ChildId::SHARDS[1] {
                    1
                } else {
                    slot.index() as u32
                };
                select(
                    &mut free,
                    score(&[pool.as_bytes(), &[0xff], &key, &candidate.to_le_bytes()]),
                    slot,
                );
            }
        }
        free.map(|(_, slot)| slot).ok_or(Placement::Full)
    });
    let slot = match candidate {
        Ok(slot) => slot,
        Err(outcome) => return outcome,
    };
    let now = api::time();
    let growth = STATE.with_borrow(|s| {
        let pools = &s.as_ref().unwrap().pools;
        let ledger = &pools.shards[pool];
        let approval = &ledger.approval;
        let origin = pools.admit_growth(
            ledger.allocated_slots(),
            approval.creation_budget,
            approval.creation_payer,
            paid,
        )?;
        if now >= approval.expires_at_ns {
            return Err(Placement::GrowthRefused);
        }
        Ok((
            origin,
            approval.reference.clone(),
            approval.arguments.clone(),
            approval.expires_at_ns,
        ))
    });
    let (origin, reference, arguments, expires) = match growth {
        Ok(terms) => terms,
        Err(outcome) => return outcome,
    };
    let Some(generation) = create(origin, slot, reference, arguments, now, expires) else {
        return Placement::GrowthRefused;
    };
    STATE.with_borrow_mut(|s| {
        let state = s.as_mut().unwrap();
        let ledger = state.pools.shards.get_mut(pool).unwrap();
        ledger.ordinals.push(slot);
        ledger.generations[slot.index()] = Some(generation);
        state.pool.set(state.pools.encode());
        resolve(state, &state.pools.shards[pool], slot, generation)
    })
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
    singleton::validate(state);
    let claims = &state.pools.projects;
    assert_eq!(
        state.project_index.len(),
        claims.len() as u64,
        "project claim index count"
    );
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
        for &slot in state
            .journal
            .0
            .keys()
            .filter(|s| s.service_index().is_some())
        {
            assert_eq!(
                pools.permanent.contains(&slot),
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
            ledger
                .generations
                .get(slot.index())
                .copied()
                .flatten()
                .map(|generation| (ledger, generation))
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
            assert_eq!(
                state.project_index.get(&claim.key),
                Some(index as u32),
                "project claim index"
            );
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

/// Observe the original project creation without issuing an effect.
/// # Panics
/// Requires the approved project hub.
#[must_use]
pub fn lookup_instance(key: &Key) -> Placement {
    STATE.with_borrow(|s| {
        let state = s.as_ref().unwrap();
        let caller = api::caller().to_text();
        assert!(
            state
                .pools
                .instances
                .as_ref()
                .is_some_and(|grant| grant.hub == caller),
            "configured instance grant required"
        );
        let Some(index) = state.project_index.get(key) else {
            return Placement::Unknown;
        };
        let claim = &state.pools.projects[index as usize];
        let slot = ChildId::INSTANCES[index as usize];
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
    instance(key, None)
}

fn instance(key: Key, paid: Option<payment::Settlement>) -> Placement {
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
        let origin = pools.admit_growth(
            slots[..count].iter().copied(),
            grant.creation_budget,
            grant.creation_payer,
            paid,
        )?;
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
        assert!(
            state
                .project_index
                .insert(key, state.pools.projects.len() as u32)
                .is_none()
        );
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
    /// A shard can close issuance after its hub acknowledges zero occupancy during drain.
    pub evacuated_shard: Option<bool>,
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
        if slot.service_index().is_some()
            && let Some(named) = super::directory::named_service(slot)
        {
            return Ok(AssignedRole {
                role: named.role,
                generation,
                evacuated_shard: None,
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
                    Some(state.pools.evacuated.contains(&slot)),
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
            evacuated_shard: shard,
        })
    })
}
