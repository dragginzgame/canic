//! Independent worker allowance. Ordinals are permanent within the approved pool.
use super::{Key, Pools, STATE, api, create, ordinal, placement, validate_pool_install};
use canic::sharding::{Allocation, Placement};
use canic_management_protocol::{Refusal, children::ChildId, stored};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(super) struct Workers {
    pub grant: stored::InstancePool,
    pub claims: Vec<WorkerClaim>,
}

#[derive(Serialize, Deserialize)]
pub(super) struct WorkerClaim {
    pub returned: bool,
    pub created: bool,
    pub slot: ChildId,
    pub generation: [u8; 32],
    pub principal: Option<String>,
}

impl Pools {
    pub(super) fn worker_at(&self, slot: ChildId) -> Option<(&Workers, &WorkerClaim)> {
        self.workers.values().find_map(|pool| {
            let claim = pool
                .claims
                .iter()
                .find(|claim| claim.slot == slot && !claim.returned)?;
            Some((pool, claim))
        })
    }
}

// Keep the original evidence before making this management slot available again.
pub(super) fn archive(state: &mut super::super::State, slot: ChildId, generation: [u8; 32]) {
    let (name, i, claim) = state
        .pools
        .workers
        .iter_mut()
        .find_map(|(name, w)| {
            w.claims
                .iter_mut()
                .enumerate()
                .find(|(_, c)| c.slot == slot && c.generation == generation && !c.returned)
                .map(|(i, c)| (name, i, c))
        })
        .expect("live worker allocation");
    claim.returned = true;
    state
        .pool
        .put(&Key::WorkerClaim(name.clone(), ordinal(i)), claim);
    let journal = &state.journal.rows()[&slot];
    let artifact = &state.artifact.rows()[&slot];
    assert_eq!(journal.0.attempts[0], u8::from(claim.created));
    let bytes = ic_wire::encode(
        &(journal, artifact),
        ic_wire::Encoding::Payload,
        canic_management_protocol::stored::LIMITS,
    )
    .expect("worker custody");
    assert!(
        state
            .worker_history
            .insert(claim.generation, bytes)
            .is_none()
    );
    state.set_journal(slot, Default::default());
    state
        .artifact
        .set(slot, Default::default())
        .expect("empty child artifact");
}

impl Workers {
    fn live(&self) -> usize {
        self.claims.iter().filter(|claim| !claim.returned).count()
    }

    pub fn minimum_reuse_cycles(&self) -> Result<std::num::NonZeroU64, &'static str> {
        let install = canic::topology::Install::decode(self.grant.arguments.as_slice())?;
        canic::topology::worker::minimum_reuse_cycles(&install.application)
            .ok_or("grant excludes recycling")
    }

    pub fn validate(&self, pools: &Pools) {
        validate_worker(&self.grant).expect("retained worker grant");
        assert!(compatible(pools, &self.grant));
        assert!(self.live() <= usize::from(self.grant.max_instances));
        assert!(self.live() + pools.projects.len() <= ChildId::INSTANCES.len());
    }
}

fn validate_worker(grant: &stored::InstancePool) -> Result<(), Refusal> {
    let install = canic::topology::Install::decode(grant.arguments.as_slice())
        .map_err(|_| Refusal::Binding)?;
    validate_pool_install(grant, &install)
}

pub(super) fn separate(first: &stored::InstancePool, other: &stored::InstancePool) -> bool {
    first.root == other.root
        && first.reference.store == other.reference.store
        && first.hub != other.hub
        && first.component != other.component
}

fn compatible(pools: &Pools, grant: &stored::InstancePool) -> bool {
    pools
        .capacity
        .as_ref()
        .is_some_and(|c| u32::from(grant.max_instances) <= c.max_children)
        && pools
            .workers
            .values()
            .all(|other| other.grant.reference.store == grant.reference.store)
        && pools
            .instances
            .as_ref()
            .is_none_or(|other| separate(grant, other))
        && pools
            .shards
            .values()
            .all(|other| super::compatible(&other.approval, grant))
}

fn installed_hub(state: &super::super::State, hub: &str) -> bool {
    state.pools.permanent.iter().copied().any(|slot| {
        let child = &state.journal.rows()[&slot].0;
        child.phase == canic_management_protocol::Phase::Ready
            && child.worker.as_deref() == Some(hub)
    })
}

/// Approve an independent worker hub using the existing bounded creation contract.
/// # Errors
/// Refuses another owner, invalid release, overlapping grant or expired deadline.
/// # Panics
/// Requires Root's controller and opened placement state.
pub fn configure_workers(grant: stored::InstancePool) -> Result<(), Refusal> {
    let expires = grant.expires_at_ns;
    super::admit_grant(
        grant,
        expires,
        |state, grant| {
            let pools = &state.pools;
            if let Some(workers) = pools.workers.get(&grant.component) {
                return Some(super::existing_grant(&workers.grant, grant));
            }
            if pools.workers.len() >= usize::from(pools.pool_limit().min(stored::MAX_INSTANCES)) {
                return Some(Err(Refusal::Binding));
            }
            (!installed_hub(state, &grant.hub) || !compatible(pools, grant))
                .then_some(Err(Refusal::Binding))
        },
        validate_worker,
        |pools, rows, grant| {
            rows.put(&Key::Worker(grant.component.clone()), &grant);
            pools.workers.insert(
                grant.component.clone(),
                Workers {
                    grant,
                    claims: Vec::new(),
                },
            );

            Ok(())
        },
    )
}

/// Observe a worker ordinal under the original hub grant.
/// # Panics
/// Requires the approved hub and intact allocation custody.
#[must_use]
pub fn lookup_worker(component: &str, ordinal: u8) -> Placement {
    resolve_worker(component, ordinal, false, None)
}

pub(super) fn resolve_worker(
    component: &str,
    ordinal: u8,
    reserve: bool,
    paid: Option<super::payment::Settlement>,
) -> Placement {
    let index = usize::from(ordinal);
    let (grant, slot, origin) = match STATE.with_borrow(|s| {
        let state = s.as_ref().unwrap();
        let workers = state
            .pools
            .workers
            .get(component)
            .filter(|w| w.grant.hub == api::caller().to_text())
            .expect("calling hub must own requested worker pool");
        if let Some(claim) = workers.claims.get(index) {
            let slot = claim.slot;
            let allocation = Allocation {
                slot: slot.index() as u32,
                generation: claim.generation,
            };
            if claim.returned || state.pools.withdrawn(slot) {
                return Err(Placement::Unavailable(allocation, claim.principal.clone()));
            }
            return Err(placement::resolve(
                allocation,
                claim.principal.as_deref(),
                state.artifact.rows()[&slot]
                    .sources()
                    .map(|s| s.creation.operation),
                &state.journal.rows()[&slot].0,
            ));
        }
        if !reserve {
            return Err(Placement::Unknown);
        }
        if workers.live() >= usize::from(workers.grant.max_instances) {
            return Err(Placement::Full);
        }

        if index != workers.claims.len() {
            return Err(Placement::Busy);
        }
        let origin = paid.ok_or(Placement::GrowthRefused)?.origin();
        let slot = ChildId::INSTANCES
            .iter()
            .skip(state.pools.projects.len())
            .rev()
            .find(|slot| state.pools.worker_at(**slot).is_none())
            .copied()
            .ok_or(Placement::Full)?;
        Ok((workers.grant.clone(), slot, origin))
    }) {
        Ok(grant) => grant,
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
        let created = state.pools.standby_claim(slot).is_none();
        state
            .pools
            .workers
            .get_mut(component)
            .unwrap()
            .claims
            .push(WorkerClaim {
                created,
                returned: false,
                slot,
                generation,
                principal: None,
            });
        state.pool.put(
            &Key::WorkerClaim(component.to_owned(), super::ordinal(index)),
            &state.pools.workers[component].claims[index],
        );
    });
    Placement::Pending(Allocation {
        slot: slot.index() as u32,
        generation,
    })
}
