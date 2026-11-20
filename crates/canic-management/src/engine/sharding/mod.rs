//! Independent hub grants and shared-shard placement over Store-backed child effects.
mod capacity;
pub use capacity::{configure_capacity, increase_pool};

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

#[derive(Clone, Serialize, Deserialize)]
#[serde(bound(
    serialize = "[u16; N]: Serialize, [Option<[u8; 32]>; N]: Serialize, [Option<String>; N]: Serialize",
    deserialize = "[u16; N]: Deserialize<'de>, [Option<[u8; 32]>; N]: Deserialize<'de>, [Option<String>; N]: Deserialize<'de>"
))]
struct Ledger<const N: usize> {
    approval: ShardPool,
    claims: BTreeMap<Key, ChildId>,
    counts: [u16; N],
    generations: [Option<[u8; 32]>; N],
    principals: [Option<String>; N],
}

const SLOT_COUNT: usize = ChildId::SHARDS.len() + ChildId::INSTANCES.len();

impl<const N: usize> Ledger<N> {
    fn resize<const M: usize>(self) -> Ledger<M> {
        Ledger {
            approval: self.approval,
            claims: self.claims,
            counts: std::array::from_fn(|i| self.counts.get(i).copied().unwrap_or_default()),
            generations: std::array::from_fn(|i| self.generations.get(i).copied().flatten()),
            principals: std::array::from_fn(|i| self.principals.get(i).cloned().flatten()),
        }
    }
}

// The historical untagged Option starts with 0 or 1. Once an instance grant
// exists, older Roots must refuse the entire record on reopen.
const INSTANCE_LAYOUT: &[u8] = b"CPL1";

#[derive(Default, Serialize, Deserialize)]
pub(super) struct Pools {
    #[serde(skip)]
    permanent: [bool; 2],
    shards: Option<Ledger<SLOT_COUNT>>,
    capacity: Option<stored::SubnetCapacity>,
    shard_increases: Vec<u8>,
    instance_increases: Vec<u8>,
    instances: Option<stored::InstancePool>,
    projects: Vec<ProjectClaim>,
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
        let (tag, payload) = if self.permanent.contains(&true) {
            (
                b"CPL4".as_slice(),
                ic_wire::encode(&(self.permanent, self), ic_wire::Encoding::Payload, LIMITS),
            )
        } else if self.capacity.is_some() {
            (
                b"CPL3".as_slice(),
                ic_wire::encode(self, ic_wire::Encoding::Payload, LIMITS),
            )
        } else {
            let historical = self.shards.clone().map(Ledger::resize::<2>);
            if self.instances.is_none() {
                return ic_wire::encode(&historical, ic_wire::Encoding::Payload, LIMITS)
                    .expect("bounded shard ledger");
            }
            if self.projects.is_empty() {
                (
                    INSTANCE_LAYOUT,
                    ic_wire::encode(
                        &(historical, &self.instances),
                        ic_wire::Encoding::Payload,
                        LIMITS,
                    ),
                )
            } else {
                (
                    b"CPL2".as_slice(),
                    ic_wire::encode(
                        &(historical, &self.instances, &self.projects),
                        ic_wire::Encoding::Payload,
                        LIMITS,
                    ),
                )
            }
        };
        let mut bytes = tag.to_vec();
        bytes.extend(payload.expect("bounded placement grants"));
        assert!(bytes.len() <= BYTES);
        bytes
    }
}

pub(super) fn open(bytes: &[u8]) -> Pools {
    type LegacyShards = Option<Ledger<2>>;
    let pools = if bytes.starts_with(b"CPL3") || bytes.starts_with(b"CPL4") {
        let pools: Pools = if let Some(payload) = bytes.strip_prefix(b"CPL4") {
            let (permanent, mut pools): ([bool; 2], Pools) =
                ic_wire::decode(payload, ic_wire::Encoding::Payload, LIMITS)
                    .expect("mixed child ledger");
            assert!(permanent.contains(&true));
            pools.permanent = permanent;
            pools
        } else {
            ic_wire::decode(&bytes[4..], ic_wire::Encoding::Payload, LIMITS)
                .expect("expanded placement ledger")
        };
        let capacity = pools.capacity.as_ref().expect("subnet capacity");
        assert!(
            capacity::valid_capacity(capacity)
                && pools.used() <= usize::from(capacity.max_children)
        );
        pools
    } else {
        let (shards, instances, projects): (
            LegacyShards,
            Option<stored::InstancePool>,
            Vec<ProjectClaim>,
        ) = if let Some(payload) = bytes.strip_prefix(b"CPL2") {
            let old: (
                LegacyShards,
                Option<stored::InstancePool>,
                Vec<ProjectClaim>,
            ) = ic_wire::decode(payload, ic_wire::Encoding::Payload, LIMITS)
                .expect("project claims");
            assert!(old.1.is_some() && !old.2.is_empty());
            old
        } else if let Some(payload) = bytes.strip_prefix(INSTANCE_LAYOUT) {
            let (shards, grant): (LegacyShards, Option<stored::InstancePool>) =
                ic_wire::decode(payload, ic_wire::Encoding::Payload, LIMITS)
                    .expect("placement grants");
            (shards, Some(grant.expect("instance grant")), vec![])
        } else {
            (
                ic_wire::decode(bytes, ic_wire::Encoding::Payload, LIMITS).expect("shard ledger"),
                None,
                vec![],
            )
        };
        Pools {
            shards: shards.map(Ledger::resize),
            instances,
            projects,
            ..Default::default()
        }
    };
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
        let mut counts = [0_u16; SLOT_COUNT];
        let slots = pools.shard_slots(&l.approval);
        for slot in l.claims.values() {
            assert!(slots.contains(slot));
            counts[slot.index()] += 1;
            assert!(counts[slot.index()] <= l.approval.keys_per_shard);
        }
        assert_eq!(counts, l.counts);
        assert!(
            l.generations.iter().flatten().count() as u64 * CREATION_CYCLES
                <= l.approval.creation_budget
        );
        for (index, count) in counts.iter().copied().enumerate() {
            assert!(l.principals[index].is_none() || count > 0);
            assert_eq!(count > 0, l.generations[index].is_some());
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
        if a.max_shards > pools.pool_limit() {
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
    if a.expires_at_ns <= api::time() {
        return Err(Refusal::Time);
    }
    delivery::admit(&a.reference, &a.reference.manifest.hash)?;
    STATE.with_borrow_mut(|s| {
        let state = s.as_mut().unwrap();
        state.pools.shards = Some(Ledger {
            approval: a,
            claims: BTreeMap::new(),
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
        if a.max_instances > pools.pool_limit() {
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
                let claim = state
                    .pools
                    .projects
                    .get_mut(index)
                    .expect("claimed project");
                (claim.generation, &mut claim.principal)
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
        let l = s.as_ref().unwrap().pools.shards.as_ref().unwrap();
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
    let (approval, slots, counts, generations) = STATE.with_borrow(|s| {
        let pools = &s.as_ref().unwrap().pools;
        let ledger = pools.shards.as_ref().unwrap();
        (
            ledger.approval.clone(),
            pools.shard_slots(&ledger.approval),
            ledger.counts,
            ledger.generations,
        )
    });
    if usize::from(counts.iter().sum::<u16>()) == slots.len() * usize::from(approval.keys_per_shard)
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
        assert!(ledger.claims.insert(key, slot).is_none());
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
        let generation = pools
            .shards
            .as_ref()
            .and_then(|ledger| ledger.generations[slot.index()]);
        if pools.permanent.get(slot.index()) == Some(&true) {
            assert!(
                generation.is_none() && state.journal.0[&slot].0.approval.is_some(),
                "permanent child custody"
            );
            continue;
        }
        if index >= 2
            || pools.shards.is_some()
            || pools.instances.is_some()
            || pools.capacity.is_some()
        {
            let source = state.artifact.0.get(&slot).and_then(Artifact::sources);
            assert_eq!(
                generation,
                source.map(|s| s.creation.operation),
                "shard journal binding"
            );
        }
    }
    for (index, slot) in ChildId::INSTANCES.into_iter().enumerate() {
        let Some(journal) = state.journal.0.get(&slot) else {
            assert!(claims.get(index).is_none());
            continue;
        };
        let source = state.artifact.0[&slot].sources();
        if let Some(claim) = claims.get(index) {
            assert!(claims[..index].iter().all(|other| other.key != claim.key));
            let grant = state.pools.instances.as_ref().expect("project grant");
            let Artifact::Application(sources, app) = &state.artifact.0[&slot] else {
                panic!("project install custody")
            };
            assert_eq!(app.creation, grant.arguments);
            assert_eq!(sources.creation.reference, grant.reference);
            assert_eq!(sources.creation.operation, claim.generation);
            assert!(journal.0.approval.is_some());
            if claim.principal.is_some() {
                assert_eq!(claim.principal, journal.0.worker);
            }
            assert_eq!(claim.principal.is_some(), journal.0.phase == Phase::Ready);
        } else {
            assert!(source.is_none() && journal.0.phase == Phase::Empty);
        }
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
