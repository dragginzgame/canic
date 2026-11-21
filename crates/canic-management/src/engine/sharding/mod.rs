//! Independent hub grants and shared-shard placement over Store-backed child effects.
mod capacity;
mod drain;
mod workers;
pub use capacity::{configure_capacity, increase_pool};
pub use drain::{begin_shard_drain, inspect_shard, release_shard};
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

pub(crate) const BYTES: usize = 64 * 1024;
const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: BYTES,
    allocation: BYTES * 16,
    nodes: BYTES,
    depth: 24,
};

const SLOT_COUNT: usize = ChildId::SHARDS.len() + ChildId::INSTANCES.len();

#[derive(Serialize, Deserialize)]
struct Ledger {
    approval: ShardPool,
    counts: [u32; SLOT_COUNT],
    generations: [Option<[u8; 32]>; SLOT_COUNT],
    principals: [Option<String>; SLOT_COUNT],
}

#[derive(Default, Serialize, Deserialize)]
pub(super) struct Pools {
    permanent: [bool; 2],
    draining: BTreeMap<ChildId, [u8; 32]>,
    shards: Option<Ledger>,
    capacity: Option<stored::SubnetCapacity>,
    shard_increases: Vec<u8>,
    instance_increases: Vec<u8>,
    instances: Option<stored::InstancePool>,
    projects: Vec<ProjectClaim>,
    #[serde(skip)]
    workers: Option<workers::Workers>,
}

#[derive(Serialize, Deserialize)]
struct ProjectClaim {
    key: Key,
    generation: [u8; 32],
    principal: Option<String>,
}

impl Pools {
    fn pool_limit(&self) -> u8 {
        self.capacity.as_ref().map_or(2, |c| c.max_children)
    }

    fn shard_slots(&self, grant: &ShardPool) -> Vec<ChildId> {
        let count = self
            .shard_increases
            .last()
            .copied()
            .unwrap_or(grant.max_shards);
        ChildId::SHARDS
            .into_iter()
            .filter(|slot| !self.permanent.get(slot.index()).copied().unwrap_or(false))
            .take(usize::from(count))
            .collect()
    }

    fn instance_slots(&self, grant: &stored::InstancePool) -> &'static [ChildId] {
        let count = self
            .instance_increases
            .last()
            .copied()
            .unwrap_or(grant.max_instances);
        &ChildId::INSTANCES[..usize::from(count)]
    }

    fn used(&self) -> usize {
        self.permanent.iter().filter(|&&used| used).count()
            + self.projects.len()
            + self.workers.as_ref().map_or(0, |w| w.claims.len())
            + self
                .shards
                .as_ref()
                .map_or(0, |l| l.generations.iter().flatten().count())
    }

    fn full(&self) -> bool {
        self.capacity
            .as_ref()
            .is_some_and(|c| self.used() >= usize::from(c.max_children))
    }

    pub(super) fn encode(&self) -> Vec<u8> {
        if let Some(workers) = &self.workers {
            let mut bytes = b"WKR1".to_vec();
            bytes.extend(
                ic_wire::encode(
                    &(self, workers),
                    ic_wire::Encoding::Payload,
                    ic_wire::Limits {
                        bytes: BYTES - 4,
                        ..LIMITS
                    },
                )
                .expect("bounded worker metadata"),
            );
            return bytes;
        }
        ic_wire::encode(self, ic_wire::Encoding::Payload, LIMITS)
            .expect("bounded placement metadata")
    }
}

pub(super) fn open(bytes: &[u8], claim_count: u64) -> Pools {
    let pools: Pools = if let Some(payload) = bytes.strip_prefix(b"WKR1") {
        let (mut pools, workers): (Pools, workers::Workers) =
            ic_wire::decode(payload, ic_wire::Encoding::Payload, LIMITS).expect("worker metadata");
        workers.validate(&pools);
        pools.workers = Some(workers);
        pools
    } else {
        ic_wire::decode(bytes, ic_wire::Encoding::Payload, LIMITS).expect("placement metadata")
    };
    assert_eq!(
        claim_count,
        pools.shards.as_ref().map_or(0, |ledger| ledger
            .counts
            .iter()
            .copied()
            .map(u64::from)
            .sum()),
        "placement index count"
    );
    for (&slot, generation) in &pools.draining {
        let ledger = pools.shards.as_ref().expect("draining pool");
        assert!(
            ChildId::SHARDS.contains(&slot)
                && ledger.generations[slot.index()].as_ref() == Some(generation)
                && ledger.principals[slot.index()].is_some()
        );
    }
    if pools.permanent.contains(&true) {
        pools
            .capacity
            .as_ref()
            .expect("permanent children require subnet capacity");
    }
    if let Some(capacity) = &pools.capacity {
        assert!(
            capacity::valid_capacity(capacity)
                && pools.used() <= usize::from(capacity.max_children)
        );
    }
    for (initial, increases) in [
        (
            pools.instances.as_ref().map(|a| a.max_instances),
            &pools.instance_increases,
        ),
        (
            pools.shards.as_ref().map(|l| l.approval.max_shards),
            &pools.shard_increases,
        ),
    ] {
        if !increases.is_empty() {
            let cap = pools
                .capacity
                .as_ref()
                .expect("approved subnet ceiling")
                .max_children;
            let original = initial.expect("original pool grant");
            assert!(
                original < increases[0]
                    && increases.windows(2).all(|w| w[0] < w[1])
                    && increases.last() <= Some(&cap)
            );
        }
    }
    if let Some(grant) = &pools.instances {
        assert!(grant.max_instances <= pools.pool_limit());
        validate_instance(grant).expect("retained instance grant");
        assert!(pools.projects.len() <= pools.instance_slots(grant).len());
        assert!(pools.projects.len() as u64 * CREATION_CYCLES <= grant.creation_budget);
        if let Some(shared) = &pools.shards {
            assert!(
                compatible(&shared.approval, grant),
                "overlapping placement grants"
            );
        }
    }
    if let Some(l) = &pools.shards {
        assert!(valid_limits(&l.approval) && l.approval.max_shards <= pools.pool_limit());
        check_shard_topology(&l.approval).expect("retained shard topology");
        let slots = pools.shard_slots(&l.approval);
        assert!(
            l.generations.iter().flatten().count() as u64 * CREATION_CYCLES
                <= l.approval.creation_budget
        );
        for (index, count) in l.counts.iter().copied().enumerate() {
            assert!(count <= l.approval.keys_per_shard);
            assert!(count == 0 || slots.iter().any(|slot| slot.index() == index));
            let occupied = count > 0 || pools.draining.keys().any(|slot| slot.index() == index);
            assert!(l.principals[index].is_none() || occupied);
            assert_eq!(occupied, l.generations[index].is_some());
        }
    }
    pools
}

fn valid_limits(a: &ShardPool) -> bool {
    (1..=ChildId::SHARDS.len()).contains(&usize::from(a.max_shards))
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
            s.pools.capacity.is_some() || s.pools.shards.is_some() || s.pools.instances.is_some()
        })
    })
}

/// Ordinary staging is available alongside placement only under an explicit backstop.
pub(crate) fn has_capacity() -> bool {
    STATE.with_borrow(|s| s.as_ref().is_some_and(|s| s.pools.capacity.is_some()))
}

pub(super) fn ordinary_slot(slot: ChildId) -> bool {
    STATE.with_borrow(|s| {
        let pools = &s.as_ref().expect("open Root").pools;
        ChildId::ALL.contains(&slot)
            && pools
                .shards
                .as_ref()
                .is_none_or(|l| l.generations[slot.index()].is_none())
    })
}

pub(super) fn retain_permanent(slot: ChildId) {
    STATE.with_borrow_mut(|s| {
        let state = s.as_mut().expect("open Root");
        if state.pools.capacity.is_some() {
            state.pools.permanent[slot.index()] = true;
            state.pool.set(state.pools.encode());
        }
    });
}

pub(super) fn at_capacity() -> bool {
    STATE.with_borrow(|s| s.as_ref().expect("open Root").pools.full())
}

/// Ingress guard before accepting an account key.
/// # Errors
/// Refuses any principal other than the approved hub.
pub fn hub() -> Result<(), &'static str> {
    STATE.with_borrow(|s| {
        s.as_ref()
            .and_then(|s| s.pools.shards.as_ref())
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
    if let Some(result) = STATE.with_borrow(|s| {
        let pools = &s.as_ref().expect("open Root").pools;
        if a.max_shards > pools.pool_limit()
            || pools
                .workers
                .as_ref()
                .is_some_and(|w| !compatible(&a, &w.grant))
        {
            return Some(Err(Refusal::Binding));
        }
        if let Some(l) = &pools.shards {
            return Some(if l.approval == a {
                Ok(())
            } else {
                Err(Refusal::Occupied)
            });
        }
        pools
            .instances
            .as_ref()
            .filter(|i| !compatible(&a, i))
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
        state.pools.shards = Some(Ledger {
            approval: a,
            counts: [0; SLOT_COUNT],
            generations: [None; SLOT_COUNT],
            principals: Default::default(),
        });
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
        && shared.hub != instances.hub
        && shared.pool != instances.component
}

fn validate_instance(a: &stored::InstancePool) -> Result<(), Refusal> {
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
    let install =
        canic::topology::Install::decode(a.arguments.as_slice()).map_err(|_| Refusal::Binding)?;
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
                .as_ref()
                .is_some_and(|w| !workers::separate(&a, &w.grant))
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
            .as_ref()
            .filter(|l| !compatible(&l.approval, &a))
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
        let principal = state.pools.shards.as_ref().unwrap().principals[slot.index()].as_deref();
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
                        .as_mut()
                        .and_then(|w| w.claims.get_mut(workers::worker_index(slot)?))
                        .expect("claimed worker");
                    (claim.generation, &mut claim.principal)
                }
            } else {
                let Some(ledger) = state.pools.shards.as_mut() else {
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
pub fn lookup(key: &Key) -> Placement {
    hub().expect("hub required");
    STATE.with_borrow(|s| {
        let state = s.as_ref().unwrap();
        let l = state.pools.shards.as_ref().unwrap();
        state.claims.get(key).map_or(Placement::Unknown, |id| {
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
pub fn assign(key: Key) -> Placement {
    let retained = lookup(&key);
    if retained != Placement::Unknown {
        return retained;
    }
    let (approval, slots, counts, generations) = STATE.with_borrow(|s| {
        let pools = &s.as_ref().unwrap().pools;
        let ledger = pools.shards.as_ref().unwrap();
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
        if STATE.with_borrow(|s| s.as_ref().unwrap().pools.full()) {
            return Placement::Full;
        }
        let issued = generations.iter().flatten().count() as u64;
        let now = api::time();
        if (issued + 1) * CREATION_CYCLES > approval.creation_budget
            || now >= approval.expires_at_ns
        {
            return Placement::GrowthRefused;
        }
        let Some(generation) = create(
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
        let ledger = state.pools.shards.as_mut().unwrap();
        assert!(
            state
                .claims
                .insert(key, u8::try_from(slot.index()).unwrap())
                .is_none()
        );
        ledger.counts[slot.index()] += 1;
        ledger.generations[slot.index()] = Some(generation);
        state.pool.set(state.pools.encode());
    });
    resolve(slot, generation)
}

fn create(
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
            super::Origin::Placement,
            slot,
            input.approval,
            Artifact::Stored(stored::Sources {
                creation,
                upgrades: vec![],
            }),
        )
    } else {
        super::application::admit_creation(
            super::Origin::Placement,
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
    Some(
        delivery::source(slot, false)
            .expect("admitted creation")
            .operation,
    )
}

pub(super) fn validate_claims(state: &super::State) {
    #[cfg(feature = "role-issuer")]
    super::role_issuer::validate_closures(state, |p, g| {
        drain::closed_assignment(&state.pools, p, g)
    });
    let claims = &state.pools.projects;
    let pools = &state.pools;
    if pools.capacity.is_some() {
        assert_eq!(
            pools.used(),
            state
                .journal
                .0
                .values()
                .filter(|entry| entry.0.approval.is_some())
                .count(),
            "subnet reservation custody"
        );
    } else {
        assert_eq!(state.journal.0.len() > 2, !claims.is_empty());
    }
    for (index, slot) in ChildId::SHARDS.into_iter().enumerate() {
        let shard = pools.shards.as_ref().and_then(|ledger| {
            ledger.generations[slot.index()].map(|generation| (ledger, generation))
        });
        if pools.permanent.get(slot.index()) == Some(&true) {
            assert!(
                shard.is_none() && state.journal.0[&slot].0.approval.is_some(),
                "permanent child custody"
            );
        } else if let Some((ledger, generation)) = shard {
            validate_allocation(
                state,
                slot,
                generation,
                ledger.principals[slot.index()].as_deref(),
                &ledger.approval.reference,
                &ledger.approval.arguments,
            );
        } else if index >= 2
            || pools.shards.is_some()
            || pools.instances.is_some()
            || pools.capacity.is_some()
        {
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
            assert!(
                claims.get(index).is_none()
                    && pools
                        .workers
                        .as_ref()
                        .and_then(|w| w.claim_at(slot))
                        .is_none()
            );
            continue;
        };
        let source = state.artifact.0[&slot].sources();
        if let Some(claim) = claims.get(index) {
            assert!(claims[..index].iter().all(|other| other.key != claim.key));
            let grant = state.pools.instances.as_ref().expect("project grant");
            validate_allocation(
                state,
                slot,
                claim.generation,
                claim.principal.as_deref(),
                &grant.reference,
                &grant.arguments,
            );
        } else if let Some((grant, claim)) = pools
            .workers
            .as_ref()
            .and_then(|w| Some((&w.grant, w.claim_at(slot)?)))
        {
            validate_allocation(
                state,
                slot,
                claim.generation,
                claim.principal.as_deref(),
                &grant.reference,
                &grant.arguments,
            );
        } else {
            assert!(source.is_none() && journal.0.phase == Phase::Empty);
        }
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
    let Some((grant, count, slot)) = STATE.with_borrow(|s| {
        let pools = &s.as_ref().unwrap().pools;
        let grant = pools.instances.as_ref().unwrap();
        let count = pools.projects.len();
        if pools.full() {
            return None;
        }
        let &slot = pools.instance_slots(grant).get(count)?;
        Some((grant.clone(), count, slot))
    }) else {
        return Placement::Full;
    };
    let now = api::time();
    if now >= grant.expires_at_ns || (count as u64 + 1) * CREATION_CYCLES > grant.creation_budget {
        return Placement::GrowthRefused;
    }
    let Some(generation) = create(
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
            .find(|(_, journal)| journal.0.worker.as_deref() == Some(caller.as_str()))
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
                let ledger = state.pools.shards.as_ref().ok_or("unassigned caller")?;
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
